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
use tracing::{debug, info, trace, warn};

/// Maximum time to keep a pending request before discarding
const PENDING_REQUEST_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum number of pending requests to keep (prevents memory leaks)
const MAX_PENDING_REQUESTS: usize = 10000;

/// HTTP decoder plugin
pub struct HttpDecoder {
    provider_registry: ProviderRegistry,
    // Track partial requests being reassembled
    partial_requests: RwLock<HashMap<CorrelationKey, RequestReassembler>>,
    // Track partial responses being reassembled
    partial_responses: RwLock<HashMap<CorrelationKey, ResponseReassembler>>,
    // Track pending requests for correlation
    pending_requests: RwLock<HashMap<CorrelationKey, PendingRequest>>,
    // Track streaming responses (OpenAI style)
    stream_reassemblers: RwLock<HashMap<CorrelationKey, StreamReassembler>>,
    // Track Anthropic streaming responses
    anthropic_reassemblers: RwLock<HashMap<CorrelationKey, AnthropicStreamReassembler>>,
    // Last cleanup time
    last_cleanup: RwLock<Instant>,
}

#[derive(Clone)]
struct ResponseReassembler {
    headers: crate::http::ParsedHttpResponse,
    body_buffer: Vec<u8>,
    created_at: Instant,
}

impl ResponseReassembler {
    fn new(headers: crate::http::ParsedHttpResponse) -> Self {
        let body_initial = headers.body.clone().unwrap_or_default();
        Self {
            headers,
            body_buffer: body_initial,
            created_at: Instant::now(),
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.body_buffer.extend_from_slice(data);
    }

    fn is_complete(&self) -> bool {
        if self.headers.is_chunked {
            // Check for the terminal chunk: "0\r\n\r\n"
            let len = self.body_buffer.len();
            if len >= 5 {
                &self.body_buffer[len - 5..] == b"0\r\n\r\n"
            } else {
                false
            }
        } else if let Some(content_len) = self.headers.content_length {
            self.body_buffer.len() >= content_len
        } else {
            // No length info and not chunked - usually means end of stream
            true
        }
    }

    fn decompress_if_needed(&mut self) {
        info!("decompress_if_needed: is_gzipped={}, is_chunked={}, body_buffer_len={}", 
            self.headers.is_gzipped, self.headers.is_chunked, self.body_buffer.len());
        
        if self.headers.is_gzipped {
            use flate2::read::GzDecoder;
            use std::io::Read;
            
            // For chunked encoding, we need to extract the actual data from the chunks first.
            // Our self.body_buffer contains the RAW chunked stream.
            let raw_data = if self.headers.is_chunked {
                if let Some(decoded) = crate::http::decode_chunked_body(&self.body_buffer) {
                    info!("Chunked decode succeeded: {} -> {} bytes", self.body_buffer.len(), decoded.len());
                    decoded
                } else {
                    info!("Chunked decode FAILED, using raw buffer");
                    self.body_buffer.clone()
                }
            } else {
                self.body_buffer.clone()
            };

            info!("Attempting gzip decompress of {} bytes, first 20: {:?}", 
                raw_data.len(), &raw_data[..std::cmp::min(20, raw_data.len())]);
            
            let mut decoder = GzDecoder::new(&raw_data[..]);
            let mut decompressed = Vec::new();
            match decoder.read_to_end(&mut decompressed) {
                Ok(_) => {
                    info!("Gzip decompress succeeded: {} -> {} bytes", raw_data.len(), decompressed.len());
                    self.body_buffer = decompressed;
                }
                Err(e) => {
                    info!("Gzip decompress FAILED: {}", e);
                    self.body_buffer = raw_data;
                }
            }
        } else if self.headers.is_chunked {
            // Not gzipped, but still chunked - need to decode chunks
            if let Some(decoded) = crate::http::decode_chunked_body(&self.body_buffer) {
                self.body_buffer = decoded;
            }
        }
        
        info!("After decompress: body_buffer_len={}, preview: {:?}", 
            self.body_buffer.len(), 
            String::from_utf8_lossy(&self.body_buffer[..std::cmp::min(100, self.body_buffer.len())]));
    }
}

#[derive(Clone)]
struct RequestReassembler {
    buffer: Vec<u8>,
    expected_body_len: Option<usize>,
    header_len: Option<usize>,
    created_at: Instant,
}

impl RequestReassembler {
    fn new(data: &[u8]) -> Self {
        let mut reassembler = Self {
            buffer: data.to_vec(),
            expected_body_len: None,
            header_len: None,
            created_at: Instant::now(),
        };
        reassembler.try_parse_headers();
        reassembler
    }

    fn try_parse_headers(&mut self) {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        if let Ok(httparse::Status::Complete(header_len)) = req.parse(&self.buffer) {
            self.header_len = Some(header_len);
            for header in req.headers.iter() {
                if header.name.to_lowercase() == "content-length" {
                    if let Ok(val_str) = std::str::from_utf8(header.value) {
                        if let Ok(len) = val_str.trim().parse::<usize>() {
                            self.expected_body_len = Some(len);
                        }
                    }
                }
            }
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        if self.header_len.is_none() {
            self.try_parse_headers();
        }
    }

    fn is_complete(&self) -> bool {
        match (self.header_len, self.expected_body_len) {
            (Some(h_len), Some(b_len)) => self.buffer.len() >= h_len + b_len,
            (Some(h_len), None) => {
                // If no content-length, assume complete if headers end with \r\n\r\n
                // (Though for POST this usually means no body)
                self.buffer.len() >= h_len
            }
            _ => false,
        }
    }
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
            partial_requests: RwLock::new(HashMap::new()),
            partial_responses: RwLock::new(HashMap::new()),
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

        // Cleanup partial requests
        {
            let mut partial = self.partial_requests.write().unwrap();
            partial.retain(|_, req| now.duration_since(req.created_at) < PENDING_REQUEST_TIMEOUT);
        }

        // Cleanup partial responses
        {
            let mut partial = self.partial_responses.write().unwrap();
            partial.retain(|_, resp| now.duration_since(resp.created_at) < PENDING_REQUEST_TIMEOUT);
        }

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

        let key = CorrelationKey::from_event(raw);

        // Check if we have an existing partial request for this connection
        let is_new_request = is_http_request(&raw.data);
        let reassembler_opt = {
            let mut partial = self.partial_requests.write().unwrap();
            if is_new_request {
                // New request starting - replace any old one for this key
                let reassembler = RequestReassembler::new(&raw.data);
                partial.insert(key.clone(), reassembler);
                partial.get(&key).cloned()
            } else {
                // Not a new request, see if it's a continuation of a partial one
                if let Some(reassembler) = partial.get_mut(&key) {
                    reassembler.feed(&raw.data);
                    Some(reassembler.clone())
                } else {
                    None
                }
            }
        };

        let reassembler = match reassembler_opt {
            Some(r) => r,
            None => {
                info!("SSL write is not HTTP request and no partial request found, skipping (data starts with: {:?})", 
                    String::from_utf8_lossy(&raw.data[..std::cmp::min(raw.data.len(), 20)]));
                return Ok(events);
            }
        };

        if !reassembler.is_complete() {
            debug!("HTTP request not yet complete, buffering (current size: {} bytes)", reassembler.buffer.len());
            return Ok(events);
        }

        // Request is complete! Remove from partials and proceed to decode
        self.partial_requests.write().unwrap().remove(&key);

        let http_req = match parse_request(&reassembler.buffer) {
            Some(req) => req,
            None => {
                info!("Failed to parse reassembled HTTP request");
                return Ok(events);
            }
        };

        // Check if this is an AI provider
        let domain = http_req.host.as_deref().unwrap_or("");
        let provider = match self.provider_registry.detect_from_domain(domain) {
            Some(p) => p,
            None => {
                info!("Domain {} is not a known AI provider", domain);
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

        // 1. Check for existing partial response
        let is_new_response = is_http_response(&raw.data);
        let reassembler_opt: Option<ResponseReassembler> = {
            let mut partials = self.partial_responses.write().unwrap();
            if is_new_response {
                if let Some(http_resp) = parse_response(&raw.data) {
                    let reassembler = ResponseReassembler::new(http_resp);
                    partials.insert(key.clone(), reassembler);
                    partials.get(&key).cloned()
                } else {
                    None
                }
            } else if let Some(reassembler) = partials.get_mut(&key) {
                reassembler.feed(&raw.data);
                Some(reassembler.clone())
            } else {
                None
            }
        };

        // 2. If we have a reassembler, check if it's complete
        if let Some(mut reassembler) = reassembler_opt {
            info!("Response reassembler: body_buffer_len={}, is_complete={}", 
                reassembler.body_buffer.len(), reassembler.is_complete());
            
            if reassembler.is_complete() {
                info!("Response COMPLETE for pid={}, buffer ends with: {:?}", 
                    key.pid, &reassembler.body_buffer[reassembler.body_buffer.len().saturating_sub(10)..]);
                
                // Remove from partials
                self.partial_responses.write().unwrap().remove(&key);

                // Find the matching pending request
                let pending_opt = {
                    let pending = self.pending_requests.read().unwrap();
                    pending
                        .get(&key)
                        .cloned()
                        .or_else(|| pending.get(&key.without_tid()).cloned())
                };

                if let Some(pending_req) = pending_opt {
                    info!("Found pending request for response: request_id={}", pending_req.request_id);
                    // Decompress body if needed
                    reassembler.decompress_if_needed();
                    
                    // Update headers with full body
                    let mut full_resp = reassembler.headers;
                    full_resp.body = Some(reassembler.body_buffer);

                    if full_resp.is_streaming || pending_req.is_streaming {
                        self.handle_streaming_response(
                            &key,
                            &pending_req,
                            &full_resp.body,
                            raw,
                            &mut events,
                        );
                    } else {
                        self.handle_complete_response(&key, &pending_req, &full_resp, raw, &mut events);
                    }
                }
            }
            return Ok(events);
        }

        // 3. Fallback for unexpected data or AI-specific streaming
        let pending_opt = {
            let pending = self.pending_requests.read().unwrap();
            pending
                .get(&key)
                .cloned()
                .or_else(|| pending.get(&key.without_tid()).cloned())
        };

        if let Some(pending_req) = pending_opt {
            if pending_req.is_streaming {
                self.handle_streaming_chunk(&key, &pending_req, &raw.data, raw, &mut events);
            }
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
                        thinking: None, // Streaming doesn't capture thinking blocks yet
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
                        thinking: None, // Streaming doesn't capture thinking blocks yet
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
                        thinking: None, // Streaming doesn't capture thinking blocks yet
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
        info!("handle_complete_response: status={}, body_len={:?}", 
            http_resp.status_code, http_resp.body.as_ref().map(|b| b.len()));
        
        let body = match &http_resp.body {
            Some(b) => b,
            None => {
                info!("handle_complete_response: No body, returning");
                return;
            }
        };

        let json: serde_json::Value = match serde_json::from_slice(body) {
            Ok(j) => j,
            Err(e) => {
                info!("handle_complete_response: JSON parse FAILED: {}", e);
                info!("Body preview: {:?}", String::from_utf8_lossy(&body[..std::cmp::min(body.len(), 200)]));
                return;
            }
        };
        
        info!("handle_complete_response: JSON parsed successfully");

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
        envelope.ts = chrono::Utc::now();
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
