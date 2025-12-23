//! Main decoder plugin

use crate::ai::{detect_provider_from_body, is_ai_request, parse_ai_request, parse_ai_response};
use crate::http::{is_http_request, is_http_response, parse_request, parse_response};
use crate::sse::StreamReassembler;

use oisp_core::events::*;
use oisp_core::plugins::{
    DecodePlugin, Plugin, PluginInfo, PluginResult, RawCaptureEvent, RawEventKind,
};
use oisp_core::providers::ProviderRegistry;

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;

/// HTTP decoder plugin
pub struct HttpDecoder {
    provider_registry: ProviderRegistry,
    // Track pending requests for correlation
    pending_requests: RwLock<HashMap<String, PendingRequest>>,
    // Track streaming responses
    stream_reassemblers: RwLock<HashMap<String, StreamReassembler>>,
}

struct PendingRequest {
    request_id: String,
    request_data: AiRequestData,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl HttpDecoder {
    pub fn new() -> Self {
        Self {
            provider_registry: ProviderRegistry::new(),
            pending_requests: RwLock::new(HashMap::new()),
            stream_reassemblers: RwLock::new(HashMap::new()),
        }
    }

    fn decode_ssl_write(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let mut events = Vec::new();

        // Try to parse as HTTP request
        if is_http_request(&raw.data) {
            if let Some(http_req) = parse_request(&raw.data) {
                // Check if this is an AI provider
                let domain = http_req.host.as_deref().unwrap_or("");
                let provider = self.provider_registry.detect_from_domain(domain);

                if let Some(provider) = provider {
                    // Try to parse body as JSON
                    if let Some(body) = &http_req.body {
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                            if is_ai_request(&json) {
                                let endpoint = format!("https://{}{}", domain, http_req.path);

                                if let Some(request_data) =
                                    parse_ai_request(&json, provider, &endpoint)
                                {
                                    let envelope = self.create_envelope(raw, "ai.request");

                                    // Store for response correlation
                                    let key =
                                        format!("{}:{}", raw.pid, raw.metadata.fd.unwrap_or(0));
                                    let mut pending = self.pending_requests.write().unwrap();
                                    pending.insert(
                                        key,
                                        PendingRequest {
                                            request_id: request_data.request_id.clone(),
                                            request_data: request_data.clone(),
                                            timestamp: envelope.ts,
                                        },
                                    );

                                    events.push(OispEvent::AiRequest(AiRequestEvent {
                                        envelope,
                                        data: request_data,
                                    }));
                                }
                            }
                        }
                    }
                }

                // Always emit network.connect for AI provider connections
                if self.provider_registry.is_ai_domain(domain) {
                    // Network connect event could be added here
                }
            }
        }

        Ok(events)
    }

    fn decode_ssl_read(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let mut events = Vec::new();

        // Try to parse as HTTP response
        if is_http_response(&raw.data) {
            if let Some(http_resp) = parse_response(&raw.data) {
                // Look up pending request
                let key = format!("{}:{}", raw.pid, raw.metadata.fd.unwrap_or(0));
                let pending = self.pending_requests.read().unwrap();

                if let Some(pending_req) = pending.get(&key) {
                    if http_resp.is_streaming {
                        // Handle streaming response
                        let mut reassemblers = self.stream_reassemblers.write().unwrap();
                        let reassembler = reassemblers.entry(key.clone()).or_default();

                        if let Some(body) = &http_resp.body {
                            reassembler.feed(body);
                        }

                        // Check if stream is complete
                        if reassembler.is_complete() {
                            // TODO: Emit complete response
                        }
                    } else {
                        // Non-streaming response
                        if let Some(body) = &http_resp.body {
                            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
                                // Detect provider from body or use the one from request
                                let provider = detect_provider_from_body(&json)
                                    .or_else(|| {
                                        pending_req.request_data.provider.as_ref().and_then(|p| {
                                            match p.name.as_str() {
                                                "openai" => {
                                                    Some(oisp_core::providers::Provider::OpenAI)
                                                }
                                                "anthropic" => {
                                                    Some(oisp_core::providers::Provider::Anthropic)
                                                }
                                                _ => None,
                                            }
                                        })
                                    })
                                    .unwrap_or(oisp_core::providers::Provider::Unknown);

                                if let Some(response_data) =
                                    parse_ai_response(&json, &pending_req.request_id, provider)
                                {
                                    let envelope = self.create_envelope(raw, "ai.response");

                                    // Calculate latency
                                    let latency = envelope.ts - pending_req.timestamp;
                                    let mut response_data = response_data;
                                    response_data.latency_ms =
                                        Some(latency.num_milliseconds() as u64);
                                    response_data.status_code = Some(http_resp.status_code);

                                    events.push(OispEvent::AiResponse(AiResponseEvent {
                                        envelope,
                                        data: response_data,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    fn decode_process_exec(&self, raw: &RawCaptureEvent) -> PluginResult<Vec<OispEvent>> {
        let envelope = self.create_envelope(raw, "process.exec");

        // Parse exec data from raw event metadata
        let data = ProcessExecData {
            exe: raw.metadata.exe.clone().unwrap_or_default(),
            args: Vec::new(), // Would be parsed from raw data
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
