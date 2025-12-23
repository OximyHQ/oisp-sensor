//! Main decoder plugin
//!
//! Decodes raw SSL/TLS capture events into structured OISP events.
//! Handles HTTP request/response correlation and AI provider detection.

use crate::ai::{
    detect_provider_from_body, is_ai_request, parse_ai_request, parse_ai_response,
    parse_anthropic_request, parse_anthropic_response,
};
use crate::http::{is_http_request, is_http_response, parse_request, parse_response};
use crate::sse::{AnthropicStreamReassembler, StreamReassembler};

use oisp_core::events::*;
use oisp_core::plugins::{
    DecodePlugin, Plugin, PluginConfig, PluginInfo, PluginResult, RawCaptureEvent, RawEventKind,
};
use oisp_core::providers::{Provider, ProviderRegistry};

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

/// Maximum time to keep a pending request before discarding
const PENDING_REQUEST_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum number of pending requests to keep (prevents memory leaks)
const MAX_PENDING_REQUESTS: usize = 10000;

/// HTTP decoder plugin
pub struct HttpDecoder {
    provider_registry: ProviderRegistry,
    // Track pending requests for correlation
    pending_requests: RwLock<HashMap<CorrelationKey, PendingRequest>>,
    // Track streaming responses (OpenAI style)
    stream_reassemblers: RwLock<HashMap<CorrelationKey, StreamReassembler>>,
    // Track Anthropic streaming responses
    anthropic_reassemblers: RwLock<HashMap<CorrelationKey, AnthropicStreamReassembler>>,
    // Last cleanup time
    last_cleanup: RwLock<Instant>,
}

/// Key for correlating requests and responses
/// Uses PID + optional TID for more accurate correlation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CorrelationKey {
    pid: u32,
    tid: Option<u32>,
    /// File descriptor if available (for multiple connections per process)
    fd: Option<i32>,
}

impl CorrelationKey {
    fn from_event(raw: &RawCaptureEvent) -> Self {
        Self {
            pid: raw.pid,
            tid: raw.tid,
            fd: raw.metadata.fd,
        }
    }

    /// Alternative key without TID for fallback matching
    fn without_tid(&self) -> Self {
        Self {
            pid: self.pid,
            tid: None,
            fd: self.fd,
        }
    }
}

#[derive(Clone)]
struct PendingRequest {
    request_id: String,
    request_data: AiRequestData,
    timestamp: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    created_at: Instant,
    provider: Provider,
    is_streaming: bool,
    #[allow(dead_code)]
    host: Option<String>,
}

impl HttpDecoder {
    pub fn new() -> Self {
        Self {
            provider_registry: ProviderRegistry::new(),
            pending_requests: RwLock::new(HashMap::new()),
            stream_reassemblers: RwLock::new(HashMap::new()),
            anthropic_reassemblers: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Cleanup stale pending requests periodically
    fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read().unwrap();
            last.elapsed() > Duration::from_secs(60) // Cleanup every minute
        };

        if should_cleanup {
            self.cleanup_stale_requests();
            *self.last_cleanup.write().unwrap() = Instant::now();
        }
    }

    fn cleanup_stale_requests(&self) {
        let now = Instant::now();

        // Cleanup pending requests
        {
            let mut pending = self.pending_requests.write().unwrap();
            let before = pending.len();
            pending.retain(|_, req| now.duration_since(req.created_at) < PENDING_REQUEST_TIMEOUT);
            let removed = before - pending.len();
            if removed > 0 {
                debug!("Cleaned up {} stale pending requests", removed);
            }
        }

        // Cleanup stream reassemblers (keep for 5 minutes)
        {
            let mut reassemblers = self.stream_reassemblers.write().unwrap();
            if reassemblers.len() > MAX_PENDING_REQUESTS {
                warn!(
                    "Too many stream reassemblers ({}), clearing oldest",
                    reassemblers.len()
                );
                reassemblers.clear();
            }
        }

        {
            let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
            if reassemblers.len() > MAX_PENDING_REQUESTS {
                warn!(
                    "Too many Anthropic reassemblers ({}), clearing oldest",
                    reassemblers.len()
                );
                reassemblers.clear();
            }
        }
    }

    fn decode_ssl_write(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        self.maybe_cleanup();
        let mut events = Vec::new();

        // Try to parse as HTTP request
        if !is_http_request(&raw.data) {
            trace!("SSL write is not HTTP request, skipping");
            return Ok(events);
        }

        let http_req = match parse_request(&raw.data) {
            Some(req) => req,
            None => {
                trace!("Failed to parse HTTP request");
                return Ok(events);
            }
        };

        // Check if this is an AI provider
        let domain = http_req.host.as_deref().unwrap_or("");
        let provider = match self.provider_registry.detect_from_domain(domain) {
            Some(p) => p,
            None => {
                trace!("Domain {} is not a known AI provider", domain);
                return Ok(events);
            }
        };

        debug!("Detected AI provider {:?} for domain {}", provider, domain);

        // Try to parse body as JSON
        let body = match &http_req.body {
            Some(b) => b,
            None => {
                trace!("No body in HTTP request");
                return Ok(events);
            }
        };

        let json: serde_json::Value = match serde_json::from_slice(body) {
            Ok(j) => j,
            Err(e) => {
                trace!("Failed to parse request body as JSON: {}", e);
                return Ok(events);
            }
        };

        if !is_ai_request(&json) {
            trace!("Request does not look like an AI request");
            return Ok(events);
        }

        let endpoint = format!("https://{}{}", domain, http_req.path);

        // Parse request based on provider
        let request_data = match provider {
            Provider::Anthropic => parse_anthropic_request(&json, &endpoint),
            _ => parse_ai_request(&json, provider, &endpoint),
        };

        let request_data = match request_data {
            Some(data) => data,
            None => {
                trace!("Failed to parse AI request data");
                return Ok(events);
            }
        };

        let envelope = self.create_envelope(raw, "ai.request");
        let is_streaming = request_data.streaming.unwrap_or(false);

        // Store for response correlation
        let key = CorrelationKey::from_event(raw);
        {
            let mut pending = self.pending_requests.write().unwrap();

            // Enforce max pending requests
            if pending.len() >= MAX_PENDING_REQUESTS {
                warn!("Max pending requests reached, removing oldest");
                // Find and remove the oldest
                if let Some(oldest_key) = pending
                    .iter()
                    .min_by_key(|(_, r)| r.created_at)
                    .map(|(k, _)| k.clone())
                {
                    pending.remove(&oldest_key);
                }
            }

            pending.insert(
                key.clone(),
                PendingRequest {
                    request_id: request_data.request_id.clone(),
                    request_data: request_data.clone(),
                    timestamp: envelope.ts,
                    created_at: Instant::now(),
                    provider,
                    is_streaming,
                    host: http_req.host.clone(),
                },
            );
        }

        debug!(
            "Parsed AI request: model={:?}, provider={:?}, streaming={}",
            request_data.model.as_ref().map(|m| &m.id),
            provider,
            is_streaming
        );

        events.push(OispEvent::AiRequest(AiRequestEvent {
            envelope,
            data: request_data,
        }));

        Ok(events)
    }

    fn decode_ssl_read(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        self.maybe_cleanup();
        let mut events = Vec::new();

        let key = CorrelationKey::from_event(raw);

        // Try to find a pending request (exact match first, then fallback)
        let pending_opt = {
            let pending = self.pending_requests.read().unwrap();
            pending
                .get(&key)
                .cloned()
                .or_else(|| pending.get(&key.without_tid()).cloned())
        };

        let pending_req = match pending_opt {
            Some(p) => p,
            None => {
                // No pending request - might be a response for a request we didn't capture
                trace!("No pending request found for correlation key {:?}", key);
                return Ok(events);
            }
        };

        // Check if this is HTTP response or streaming data
        if is_http_response(&raw.data) {
            if let Some(http_resp) = parse_response(&raw.data) {
                if http_resp.is_streaming || pending_req.is_streaming {
                    // Handle streaming response
                    self.handle_streaming_response(
                        &key,
                        &pending_req,
                        &http_resp.body,
                        raw,
                        &mut events,
                    );
                } else {
                    // Non-streaming response - parse immediately
                    self.handle_complete_response(&key, &pending_req, &http_resp, raw, &mut events);
                }
            }
        } else if pending_req.is_streaming {
            // Not HTTP response header, but we're expecting streaming data
            // Feed the raw data directly to the reassembler
            self.handle_streaming_chunk(&key, &pending_req, &raw.data, raw, &mut events);
        }

        Ok(events)
    }

    fn handle_streaming_response(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        body: &Option<Vec<u8>>,
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        let body = match body {
            Some(b) => b,
            None => return,
        };

        match pending_req.provider {
            Provider::Anthropic => {
                let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(body);

                if reassembler.is_complete() {
                    // Build complete response
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let (input_tokens, output_tokens) = reassembler.usage();

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: reassembler.stop_reason().map(|r| match r {
                                "end_turn" => FinishReason::Stop,
                                "max_tokens" => FinishReason::Length,
                                "tool_use" => FinishReason::ToolCalls,
                                _ => FinishReason::Other,
                            }),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: match (input_tokens, output_tokens) {
                                (Some(i), Some(o)) => Some(i + o),
                                _ => None,
                            },
                            cached_tokens: None,
                            reasoning_tokens: None,
                            input_cost_usd: None,
                            output_cost_usd: None,
                            total_cost_usd: None,
                        }),
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: reassembler.stop_reason().map(|r| match r {
                            "end_turn" => FinishReason::Stop,
                            _ => FinishReason::Other,
                        }),
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    // Cleanup
                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
            _ => {
                // OpenAI-style streaming
                let mut reassemblers = self.stream_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(body);

                if reassembler.is_complete() {
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: reassembler.finish_reason().map(|r| match r {
                                "stop" => FinishReason::Stop,
                                "length" => FinishReason::Length,
                                "tool_calls" => FinishReason::ToolCalls,
                                _ => FinishReason::Other,
                            }),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: None, // Streaming responses often don't include usage
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: reassembler.finish_reason().map(|r| match r {
                            "stop" => FinishReason::Stop,
                            _ => FinishReason::Other,
                        }),
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    // Cleanup
                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
        }
    }

    fn handle_streaming_chunk(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        data: &[u8],
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        // Feed to appropriate reassembler based on provider
        match pending_req.provider {
            Provider::Anthropic => {
                let mut reassemblers = self.anthropic_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(data);
                // Check completion similar to above
                if reassembler.is_complete() {
                    // Build and emit response (same logic as above)
                    let envelope = self.create_envelope(raw, "ai.response");
                    let latency = envelope.ts - pending_req.timestamp;

                    let (input_tokens, output_tokens) = reassembler.usage();

                    let response_data = AiResponseData {
                        request_id: pending_req.request_id.clone(),
                        provider_request_id: None,
                        provider: pending_req.request_data.provider.clone(),
                        model: pending_req.request_data.model.clone(),
                        status_code: Some(200),
                        success: Some(true),
                        error: None,
                        choices: vec![Choice {
                            index: 0,
                            message: Some(Message {
                                role: MessageRole::Assistant,
                                content: Some(MessageContent::Text(
                                    reassembler.content().to_string(),
                                )),
                                content_hash: None,
                                content_length: Some(reassembler.content().len()),
                                has_images: None,
                                image_count: None,
                                tool_call_id: None,
                                name: None,
                            }),
                            finish_reason: Some(FinishReason::Stop),
                        }],
                        tool_calls: Vec::new(),
                        tool_calls_count: Some(0),
                        usage: Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: match (input_tokens, output_tokens) {
                                (Some(i), Some(o)) => Some(i + o),
                                _ => None,
                            },
                            cached_tokens: None,
                            reasoning_tokens: None,
                            input_cost_usd: None,
                            output_cost_usd: None,
                            total_cost_usd: None,
                        }),
                        latency_ms: Some(latency.num_milliseconds() as u64),
                        time_to_first_token_ms: None,
                        was_cached: None,
                        finish_reason: Some(FinishReason::Stop),
                    };

                    events.push(OispEvent::AiResponse(AiResponseEvent {
                        envelope,
                        data: response_data,
                    }));

                    reassemblers.remove(key);
                    self.pending_requests.write().unwrap().remove(key);
                }
            }
            _ => {
                let mut reassemblers = self.stream_reassemblers.write().unwrap();
                let reassembler = reassemblers.entry(key.clone()).or_default();
                reassembler.feed(data);
            }
        }
    }

    fn handle_complete_response(
        &self,
        key: &CorrelationKey,
        pending_req: &PendingRequest,
        http_resp: &crate::http::ParsedHttpResponse,
        raw: &RawCaptureEvent,
        events: &mut Vec<OispEvent>,
    ) {
        let body = match &http_resp.body {
            Some(b) => b,
            None => return,
        };

        let json: serde_json::Value = match serde_json::from_slice(body) {
            Ok(j) => j,
            Err(e) => {
                trace!("Failed to parse response body as JSON: {}", e);
                return;
            }
        };

        // Detect provider from body or use the one from request
        let provider = detect_provider_from_body(&json).unwrap_or(pending_req.provider);

        let response_data = match provider {
            Provider::Anthropic => parse_anthropic_response(&json, &pending_req.request_id),
            _ => parse_ai_response(&json, &pending_req.request_id, provider),
        };

        let response_data = match response_data {
            Some(data) => data,
            None => {
                trace!("Failed to parse AI response data");
                return;
            }
        };

        let envelope = self.create_envelope(raw, "ai.response");
        let latency = envelope.ts - pending_req.timestamp;

        let mut response_data = response_data;
        response_data.latency_ms = Some(latency.num_milliseconds() as u64);
        response_data.status_code = Some(http_resp.status_code);

        debug!(
            "Parsed AI response: status={}, latency={}ms",
            http_resp.status_code,
            latency.num_milliseconds()
        );

        events.push(OispEvent::AiResponse(AiResponseEvent {
            envelope,
            data: response_data,
        }));

        // Cleanup
        self.pending_requests.write().unwrap().remove(key);
    }

    fn decode_process_exec(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let envelope = self.create_envelope(raw, "process.exec");

        let data = ProcessExecData {
            exe: raw.metadata.exe.clone().unwrap_or_default(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            interpreter: None,
            script_path: None,
            is_shell: None,
            is_script: None,
            is_interactive: None,
            binary_hash: None,
            code_signature: None,
        };

        Ok(vec![OispEvent::ProcessExec(ProcessExecEvent {
            envelope,
            data,
        })])
    }

    fn decode_network_connect(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let envelope = self.create_envelope(raw, "network.connect");

        let data = NetworkConnectData {
            dest: Endpoint {
                ip: raw.metadata.remote_addr.clone(),
                port: raw.metadata.remote_port,
                domain: None,
                is_private: None,
                geo: None,
            },
            src: Some(Endpoint {
                ip: raw.metadata.local_addr.clone(),
                port: raw.metadata.local_port,
                domain: None,
                is_private: None,
                geo: None,
            }),
            protocol: Some(Protocol::Tcp),
            success: Some(true),
            error: None,
            latency_ms: None,
            tls: None,
        };

        Ok(vec![OispEvent::NetworkConnect(NetworkConnectEvent {
            envelope,
            data,
        })])
    }

    fn create_envelope(&self, raw: &RawCaptureEvent, event_type: &str) -> EventEnvelope {
        let mut envelope = EventEnvelope::new(event_type);
        envelope.ts = chrono::DateTime::from_timestamp_nanos(raw.timestamp_ns as i64);
        envelope.ts_mono = Some(raw.timestamp_ns);

        envelope.process = Some(ProcessInfo {
            pid: raw.pid,
            ppid: raw.metadata.ppid,
            exe: raw.metadata.exe.clone(),
            name: raw.metadata.comm.clone(),
            cmdline: None,
            cwd: None,
            tid: raw.tid,
            container_id: None,
            hash: None,
            code_signature: None,
        });

        envelope.actor = raw.metadata.uid.map(|uid| Actor {
            uid: Some(uid),
            user: None,
            gid: None,
            session_id: None,
            identity: None,
        });

        envelope.source = Source {
            collector: "oisp-sensor".to_string(),
            collector_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            capture_method: Some(CaptureMethod::TlsBoundary),
            capture_point: match &raw.kind {
                RawEventKind::SslWrite => Some("ssl_write".to_string()),
                RawEventKind::SslRead => Some("ssl_read".to_string()),
                _ => None,
            },
            sensor_host: None,
        };

        envelope.confidence = Confidence {
            level: ConfidenceLevel::High,
            completeness: Completeness::Full,
            reasons: vec!["tls_boundary_capture".to_string()],
            content_source: Some("tls_boundary".to_string()),
            ai_detection_method: Some("known_endpoint".to_string()),
        };

        envelope
    }

    /// Get statistics about decoder state
    pub fn stats(&self) -> DecoderStats {
        DecoderStats {
            pending_requests: self.pending_requests.read().unwrap().len(),
            stream_reassemblers: self.stream_reassemblers.read().unwrap().len(),
            anthropic_reassemblers: self.anthropic_reassemblers.read().unwrap().len(),
        }
    }
}

/// Statistics about the decoder's internal state
#[derive(Debug, Clone)]
pub struct DecoderStats {
    pub pending_requests: usize,
    pub stream_reassemblers: usize,
    pub anthropic_reassemblers: usize,
}

impl Default for HttpDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for HttpDecoder {
    fn name(&self) -> &str {
        "http-decoder"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "HTTP/SSE decoder with AI provider fingerprinting"
    }
}

impl Plugin for HttpDecoder {
    fn init(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        // Could add configuration options here
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl DecodePlugin for HttpDecoder {
    fn can_decode(&self, raw: &RawCaptureEvent) -> bool {
        matches!(
            raw.kind,
            RawEventKind::SslWrite
                | RawEventKind::SslRead
                | RawEventKind::ProcessExec
                | RawEventKind::NetworkConnect
        )
    }

    async fn decode(&self, raw: RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        match raw.kind {
            RawEventKind::SslWrite => self.decode_ssl_write(&raw),
            RawEventKind::SslRead => self.decode_ssl_read(&raw),
            RawEventKind::ProcessExec => self.decode_process_exec(&raw),
            RawEventKind::NetworkConnect => self.decode_network_connect(&raw),
            _ => Ok(Vec::new()),
        }
    }

    fn priority(&self) -> i32 {
        100 // High priority for HTTP decoder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oisp_core::plugins::RawEventMetadata;

    fn create_raw_event(kind: RawEventKind, data: &[u8], pid: u32) -> RawCaptureEvent {
        RawCaptureEvent {
            id: ulid::Ulid::new().to_string(),
            timestamp_ns: 1000000000,
            kind,
            pid,
            tid: Some(1),
            data: data.to_vec(),
            metadata: RawEventMetadata {
                comm: Some("test".to_string()),
                uid: Some(1000),
                fd: Some(5),
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn test_decode_openai_request() {
        let decoder = HttpDecoder::new();

        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiRequest(req) = &events[0] {
            assert_eq!(req.data.model.as_ref().unwrap().id, "gpt-4");
            assert_eq!(req.data.messages.len(), 1);
        } else {
            panic!("Expected AiRequest event");
        }

        // Check that request is tracked
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 1);
    }

    #[tokio::test]
    async fn test_decode_openai_response() {
        let decoder = HttpDecoder::new();

        // First send request
        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw_req = create_raw_event(RawEventKind::SslWrite, request, 1234);
        decoder.decode(raw_req).await.unwrap();

        // Then send response
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: application/json\r\n\
                         \r\n\
                         {\"id\":\"chatcmpl-123\",\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"message\":{\"role\":\"assistant\",\"content\":\"Hi!\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}";

        let raw_resp = create_raw_event(RawEventKind::SslRead, response, 1234);
        let events = decoder.decode(raw_resp).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiResponse(resp) = &events[0] {
            assert!(resp.data.latency_ms.is_some());
            assert_eq!(resp.data.status_code, Some(200));
            assert_eq!(resp.data.choices.len(), 1);
            assert_eq!(resp.data.finish_reason, Some(FinishReason::Stop));
        } else {
            panic!("Expected AiResponse event");
        }

        // Request should be cleaned up
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 0);
    }

    #[tokio::test]
    async fn test_decode_anthropic_request() {
        let decoder = HttpDecoder::new();

        let request = b"POST /v1/messages HTTP/1.1\r\n\
                        Host: api.anthropic.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"claude-3-opus-20240229\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}],\"max_tokens\":1024}";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 1);
        if let OispEvent::AiRequest(req) = &events[0] {
            assert!(req.data.model.as_ref().unwrap().id.starts_with("claude"));
        } else {
            panic!("Expected AiRequest event");
        }
    }

    #[tokio::test]
    async fn test_correlation_by_pid() {
        let decoder = HttpDecoder::new();

        // Request from process 1234
        let request = b"POST /v1/chat/completions HTTP/1.1\r\n\
                        Host: api.openai.com\r\n\
                        Content-Type: application/json\r\n\
                        \r\n\
                        {\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}";

        let raw_req = create_raw_event(RawEventKind::SslWrite, request, 1234);
        decoder.decode(raw_req).await.unwrap();

        // Response from different process should not correlate
        let response = b"HTTP/1.1 200 OK\r\n\
                         Content-Type: application/json\r\n\
                         \r\n\
                         {\"id\":\"chatcmpl-123\",\"model\":\"gpt-4\",\"choices\":[]}";

        let raw_resp = create_raw_event(RawEventKind::SslRead, response, 5678);
        let events = decoder.decode(raw_resp).await.unwrap();

        // Should not produce any events (no matching request)
        assert_eq!(events.len(), 0);

        // Original request should still be pending
        let stats = decoder.stats();
        assert_eq!(stats.pending_requests, 1);
    }

    #[tokio::test]
    async fn test_non_ai_request_ignored() {
        let decoder = HttpDecoder::new();

        // Regular HTTP request to non-AI domain
        let request = b"GET /index.html HTTP/1.1\r\n\
                        Host: example.com\r\n\
                        \r\n";

        let raw = create_raw_event(RawEventKind::SslWrite, request, 1234);
        let events = decoder.decode(raw).await.unwrap();

        assert_eq!(events.len(), 0);
    }
}
