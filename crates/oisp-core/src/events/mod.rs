//! OISP Event types - conforming to oisp-spec v0.1
//!
//! All events share a common envelope structure with event-type-specific data.

pub mod agent;
pub mod ai;
pub mod envelope;
pub mod file;
pub mod network;
pub mod process;

pub use agent::*;
pub use ai::*;
pub use envelope::*;
pub use file::*;
pub use network::*;
pub use process::*;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// All possible OISP event types
///
/// Serializes to OISP spec format with envelope fields at root and event-specific data in `data`
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum OispEvent {
    // AI events
    AiRequest(AiRequestEvent),
    AiResponse(AiResponseEvent),
    AiStreamingChunk(AiStreamingChunkEvent),
    AiEmbedding(AiEmbeddingEvent),

    // Agent events
    AgentToolCall(AgentToolCallEvent),
    AgentToolResult(AgentToolResultEvent),
    AgentPlanStep(AgentPlanStepEvent),
    AgentRagRetrieve(AgentRagRetrieveEvent),
    AgentSession(AgentSessionEvent),

    // Process events
    ProcessExec(ProcessExecEvent),
    ProcessExit(ProcessExitEvent),
    ProcessFork(ProcessForkEvent),

    // File events
    FileOpen(FileOpenEvent),
    FileRead(FileReadEvent),
    FileWrite(FileWriteEvent),
    FileClose(FileCloseEvent),

    // Network events
    NetworkConnect(NetworkConnectEvent),
    NetworkAccept(NetworkAcceptEvent),
    NetworkFlow(NetworkFlowEvent),
    NetworkDns(NetworkDnsEvent),

    // Capture events (debugging/low-level)
    CaptureRaw(CaptureRawEvent),
}

impl OispEvent {
    /// Get the event type string
    pub fn event_type(&self) -> &'static str {
        match self {
            OispEvent::AiRequest(_) => "ai.request",
            OispEvent::AiResponse(_) => "ai.response",
            OispEvent::AiStreamingChunk(_) => "ai.streaming_chunk",
            OispEvent::AiEmbedding(_) => "ai.embedding",
            OispEvent::AgentToolCall(_) => "agent.tool_call",
            OispEvent::AgentToolResult(_) => "agent.tool_result",
            OispEvent::AgentPlanStep(_) => "agent.plan_step",
            OispEvent::AgentRagRetrieve(_) => "agent.rag_retrieve",
            OispEvent::AgentSession(_) => "agent.session",
            OispEvent::ProcessExec(_) => "process.exec",
            OispEvent::ProcessExit(_) => "process.exit",
            OispEvent::ProcessFork(_) => "process.fork",
            OispEvent::FileOpen(_) => "file.open",
            OispEvent::FileRead(_) => "file.read",
            OispEvent::FileWrite(_) => "file.write",
            OispEvent::FileClose(_) => "file.close",
            OispEvent::NetworkConnect(_) => "network.connect",
            OispEvent::NetworkAccept(_) => "network.accept",
            OispEvent::NetworkFlow(_) => "network.flow",
            OispEvent::NetworkDns(_) => "network.dns",
            OispEvent::CaptureRaw(_) => "capture.raw",
        }
    }

    /// Check if this is an AI-related event
    pub fn is_ai_event(&self) -> bool {
        matches!(
            self,
            OispEvent::AiRequest(_)
                | OispEvent::AiResponse(_)
                | OispEvent::AiStreamingChunk(_)
                | OispEvent::AiEmbedding(_)
                | OispEvent::AgentToolCall(_)
                | OispEvent::AgentToolResult(_)
                | OispEvent::AgentPlanStep(_)
                | OispEvent::AgentRagRetrieve(_)
                | OispEvent::AgentSession(_)
        )
    }

    /// Get the envelope from any event
    pub fn envelope(&self) -> &EventEnvelope {
        match self {
            OispEvent::AiRequest(e) => &e.envelope,
            OispEvent::AiResponse(e) => &e.envelope,
            OispEvent::AiStreamingChunk(e) => &e.envelope,
            OispEvent::AiEmbedding(e) => &e.envelope,
            OispEvent::AgentToolCall(e) => &e.envelope,
            OispEvent::AgentToolResult(e) => &e.envelope,
            OispEvent::AgentPlanStep(e) => &e.envelope,
            OispEvent::AgentRagRetrieve(e) => &e.envelope,
            OispEvent::AgentSession(e) => &e.envelope,
            OispEvent::ProcessExec(e) => &e.envelope,
            OispEvent::ProcessExit(e) => &e.envelope,
            OispEvent::ProcessFork(e) => &e.envelope,
            OispEvent::FileOpen(e) => &e.envelope,
            OispEvent::FileRead(e) => &e.envelope,
            OispEvent::FileWrite(e) => &e.envelope,
            OispEvent::FileClose(e) => &e.envelope,
            OispEvent::NetworkConnect(e) => &e.envelope,
            OispEvent::NetworkAccept(e) => &e.envelope,
            OispEvent::NetworkFlow(e) => &e.envelope,
            OispEvent::NetworkDns(e) => &e.envelope,
            OispEvent::CaptureRaw(e) => &e.envelope,
        }
    }
}

/// Event type categories for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Ai,
    Agent,
    Process,
    File,
    Network,
    Capture,
}

impl EventCategory {
    pub fn from_event_type(event_type: &str) -> Option<Self> {
        let prefix = event_type.split('.').next()?;
        match prefix {
            "ai" => Some(EventCategory::Ai),
            "agent" => Some(EventCategory::Agent),
            "process" => Some(EventCategory::Process),
            "file" => Some(EventCategory::File),
            "network" => Some(EventCategory::Network),
            "capture" => Some(EventCategory::Capture),
            _ => None,
        }
    }
}

// Custom serialization for OISP spec compliance
// Envelope fields at root, event-specific data in `data` field
impl Serialize for OispEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        // Get envelope and event type
        let envelope = self.envelope();
        let event_type = self.event_type();

        // Count fields: envelope fields + event_type + data
        let mut map = serializer.serialize_map(None)?;

        // Serialize envelope fields at root level
        map.serialize_entry("oisp_version", &envelope.oisp_version)?;
        map.serialize_entry("event_id", &envelope.event_id)?;
        map.serialize_entry("event_type", event_type)?;
        map.serialize_entry("ts", &envelope.ts)?;

        if let Some(ref ts_mono) = envelope.ts_mono {
            map.serialize_entry("ts_mono", ts_mono)?;
        }
        if let Some(ref host) = envelope.host {
            map.serialize_entry("host", host)?;
        }
        if let Some(ref actor) = envelope.actor {
            map.serialize_entry("actor", actor)?;
        }
        if let Some(ref process) = envelope.process {
            map.serialize_entry("process", process)?;
        }
        if let Some(ref app) = envelope.app {
            map.serialize_entry("app", app)?;
        }
        if let Some(ref web_context) = envelope.web_context {
            map.serialize_entry("web_context", web_context)?;
        }
        map.serialize_entry("source", &envelope.source)?;
        map.serialize_entry("confidence", &envelope.confidence)?;

        if !envelope.attrs.is_empty() {
            map.serialize_entry("attrs", &envelope.attrs)?;
        }
        if !envelope.ext.is_empty() {
            map.serialize_entry("ext", &envelope.ext)?;
        }
        if !envelope.related_events.is_empty() {
            map.serialize_entry("related_events", &envelope.related_events)?;
        }
        if let Some(ref trace_ctx) = envelope.trace_context {
            map.serialize_entry("trace_context", trace_ctx)?;
        }

        // Serialize event-specific data in `data` field
        match self {
            OispEvent::AiRequest(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AiResponse(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AiStreamingChunk(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AiEmbedding(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AgentToolCall(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AgentToolResult(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AgentPlanStep(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AgentRagRetrieve(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::AgentSession(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::ProcessExec(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::ProcessExit(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::ProcessFork(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::FileOpen(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::FileRead(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::FileWrite(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::FileClose(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::NetworkConnect(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::NetworkAccept(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::NetworkFlow(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::NetworkDns(e) => map.serialize_entry("data", &e.data)?,
            OispEvent::CaptureRaw(e) => map.serialize_entry("data", &e.data)?,
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for OispEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // Deserialize as a generic JSON value first
        let value = serde_json::Value::deserialize(deserializer)?;

        let obj = value
            .as_object()
            .ok_or_else(|| D::Error::custom("expected object"))?;

        // Extract event_type to determine which variant to parse
        let event_type = obj
            .get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D::Error::custom("missing event_type"))?;

        // Parse envelope from root fields
        let envelope: EventEnvelope =
            serde_json::from_value(value.clone()).map_err(D::Error::custom)?;

        // Parse data field
        let data = obj
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        // Construct the appropriate event type
        match event_type {
            "ai.request" => {
                let event_data: AiRequestData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AiRequest(AiRequestEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "ai.response" => {
                let event_data: AiResponseData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AiResponse(AiResponseEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "ai.streaming_chunk" => {
                let event_data: AiStreamingChunkData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AiStreamingChunk(AiStreamingChunkEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "ai.embedding" => {
                let event_data: AiEmbeddingData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AiEmbedding(AiEmbeddingEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "agent.tool_call" => {
                let event_data: AgentToolCallData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AgentToolCall(AgentToolCallEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "agent.tool_result" => {
                let event_data: AgentToolResultData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AgentToolResult(AgentToolResultEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "agent.plan_step" => {
                let event_data: AgentPlanStepData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AgentPlanStep(AgentPlanStepEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "agent.rag_retrieve" => {
                let event_data: AgentRagRetrieveData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AgentRagRetrieve(AgentRagRetrieveEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "agent.session" => {
                let event_data: AgentSessionData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::AgentSession(AgentSessionEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "process.exec" => {
                let event_data: ProcessExecData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::ProcessExec(ProcessExecEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "process.exit" => {
                let event_data: ProcessExitData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::ProcessExit(ProcessExitEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "process.fork" => {
                let event_data: ProcessForkData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::ProcessFork(ProcessForkEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "file.open" => {
                let event_data: FileOpenData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::FileOpen(FileOpenEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "file.read" => {
                let event_data: FileReadData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::FileRead(FileReadEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "file.write" => {
                let event_data: FileWriteData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::FileWrite(FileWriteEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "file.close" => {
                let event_data: FileCloseData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::FileClose(FileCloseEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "network.connect" => {
                let event_data: NetworkConnectData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::NetworkConnect(NetworkConnectEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "network.accept" => {
                let event_data: NetworkAcceptData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::NetworkAccept(NetworkAcceptEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "network.flow" => {
                let event_data: NetworkFlowData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::NetworkFlow(NetworkFlowEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "network.dns" => {
                let event_data: NetworkDnsData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::NetworkDns(NetworkDnsEvent {
                    envelope,
                    data: event_data,
                }))
            }
            "capture.raw" => {
                let event_data: CaptureRawData =
                    serde_json::from_value(data).map_err(D::Error::custom)?;
                Ok(OispEvent::CaptureRaw(CaptureRawEvent {
                    envelope,
                    data: event_data,
                }))
            }
            _ => Err(D::Error::custom(format!(
                "unknown event_type: {}",
                event_type
            ))),
        }
    }
}

/// Raw capture event for debugging and low-level visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRawEvent {
    pub envelope: EventEnvelope,
    pub data: CaptureRawData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRawData {
    pub kind: String,
    pub data: String, // String representation of data
    pub len: usize,
    pub pid: u32,
    pub tid: Option<u32>,
    pub comm: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_envelope() -> EventEnvelope {
        EventEnvelope {
            oisp_version: "0.1".to_string(),
            event_id: "test-event-123".to_string(),
            event_type: "test".to_string(),
            ts: Utc::now(),
            ts_mono: None,
            host: Some(Host {
                hostname: "test-host".to_string(),
                device_id: None,
                os: None,
                os_version: None,
                arch: None,
            }),
            actor: None,
            process: Some(ProcessInfo {
                pid: 1234,
                ppid: None,
                exe: None,
                name: Some("test-process".to_string()),
                cmdline: None,
                cwd: None,
                tid: None,
                container_id: None,
                hash: None,
                bundle_id: None,
                code_signature: None,
            }),
            app: None,
            source: Source::default(),
            confidence: Confidence::default(),
            attrs: Default::default(),
            ext: Default::default(),
            related_events: vec![],
            trace_context: None,
            web_context: None,
        }
    }

    #[test]
    fn test_serialize_ai_request_oisp_format() {
        let event = OispEvent::AiRequest(AiRequestEvent {
            envelope: create_test_envelope(),
            data: AiRequestData {
                request_id: "req-456".to_string(),
                provider: Some(ProviderInfo {
                    name: "openai".to_string(),
                    endpoint: Some("https://api.openai.com/v1/chat/completions".to_string()),
                    region: None,
                    organization_id: None,
                    project_id: None,
                }),
                model: Some(ModelInfo {
                    id: "gpt-4".to_string(),
                    name: Some("GPT-4".to_string()),
                    family: Some("gpt".to_string()),
                    version: None,
                    capabilities: None,
                    context_window: None,
                    max_output_tokens: None,
                }),
                auth: None,
                request_type: Some(RequestType::Completion),
                streaming: None,
                messages: vec![],
                messages_count: None,
                has_system_prompt: None,
                system_prompt_hash: None,
                tools: vec![],
                tools_count: None,
                tool_choice: None,
                parameters: None,
                has_rag_context: None,
                has_images: None,
                image_count: None,
                estimated_tokens: Some(100),
                conversation: None,
                agent: None,
            },
        });

        let json = serde_json::to_value(&event).unwrap();

        // Verify envelope fields are at root level
        assert_eq!(json["oisp_version"], "0.1");
        assert_eq!(json["event_id"], "test-event-123");
        assert_eq!(json["event_type"], "ai.request");
        assert!(json["ts"].is_string()); // ISO8601 timestamp
        assert!(json["process"].is_object());
        assert_eq!(json["process"]["pid"], 1234);
        assert_eq!(json["process"]["name"], "test-process");

        // Verify data field contains event-specific data
        assert!(json["data"].is_object());
        assert_eq!(json["data"]["request_id"], "req-456");
        assert_eq!(json["data"]["provider"]["name"], "openai");
        assert_eq!(json["data"]["model"]["id"], "gpt-4");
        assert_eq!(json["data"]["request_type"], "completion");
        assert_eq!(json["data"]["estimated_tokens"], 100);

        // Verify envelope fields are NOT in data
        assert!(json["data"]["oisp_version"].is_null());
        assert!(json["data"]["event_id"].is_null());
        assert!(json["data"]["ts"].is_null());
    }

    #[test]
    fn test_serialize_process_exec_oisp_format() {
        let event = OispEvent::ProcessExec(ProcessExecEvent {
            envelope: create_test_envelope(),
            data: ProcessExecData {
                exe: "/usr/bin/curl".to_string(),
                args: vec!["curl".to_string(), "https://api.openai.com".to_string()],
                cwd: Some("/home/user".to_string()),
                env: Default::default(),
                interpreter: None,
                script_path: None,
                is_shell: None,
                is_script: None,
                is_interactive: None,
                binary_hash: None,
                code_signature: None,
            },
        });

        let json = serde_json::to_value(&event).unwrap();

        // Verify envelope fields at root
        assert_eq!(json["event_type"], "process.exec");
        assert_eq!(json["oisp_version"], "0.1");
        assert!(json["process"].is_object());

        // Verify data field contains process-specific data
        assert_eq!(json["data"]["exe"], "/usr/bin/curl");
        assert_eq!(json["data"]["args"][0], "curl");
        assert_eq!(json["data"]["cwd"], "/home/user");
    }

    #[test]
    fn test_deserialize_ai_request_oisp_format() {
        let json = r#"{
            "oisp_version": "0.1",
            "event_id": "evt-789",
            "event_type": "ai.request",
            "ts": "2024-01-15T12:00:00Z",
            "process": {
                "pid": 5678,
                "name": "python"
            },
            "source": {
                "collector": "oisp-sensor"
            },
            "confidence": {
                "level": "medium",
                "completeness": "partial"
            },
            "data": {
                "request_id": "req-abc",
                "provider": {
                    "name": "anthropic"
                },
                "model": {
                    "id": "claude-3"
                },
                "request_type": "completion"
            }
        }"#;

        let event: OispEvent = serde_json::from_str(json).unwrap();

        // Verify it parsed correctly
        assert_eq!(event.event_type(), "ai.request");

        let envelope = event.envelope();
        assert_eq!(envelope.event_id, "evt-789");
        assert_eq!(envelope.process.as_ref().unwrap().pid, 5678);

        if let OispEvent::AiRequest(ai_req) = event {
            assert_eq!(ai_req.data.request_id, "req-abc");
            assert_eq!(ai_req.data.provider.as_ref().unwrap().name, "anthropic");
            assert_eq!(ai_req.data.model.as_ref().unwrap().id, "claude-3");
        } else {
            panic!("Expected AiRequest event");
        }
    }

    #[test]
    fn test_roundtrip_serialization() {
        let event = OispEvent::AiResponse(AiResponseEvent {
            envelope: create_test_envelope(),
            data: AiResponseData {
                request_id: "req-123".to_string(),
                provider_request_id: None,
                provider: None,
                model: None,
                status_code: Some(200),
                success: Some(true),
                error: None,
                choices: vec![],
                tool_calls: vec![],
                tool_calls_count: None,
                usage: Some(Usage {
                    prompt_tokens: Some(100),
                    completion_tokens: Some(50),
                    total_tokens: Some(150),
                    cached_tokens: None,
                    reasoning_tokens: None,
                    input_cost_usd: None,
                    output_cost_usd: None,
                    total_cost_usd: None,
                }),
                latency_ms: Some(1500),
                time_to_first_token_ms: None,
                was_cached: None,
                finish_reason: Some(FinishReason::Stop),
                thinking: None,
            },
        });

        // Serialize to JSON
        let json_str = serde_json::to_string(&event).unwrap();

        // Deserialize back
        let deserialized: OispEvent = serde_json::from_str(&json_str).unwrap();

        // Verify it matches
        assert_eq!(deserialized.event_type(), "ai.response");
        assert_eq!(deserialized.envelope().event_id, "test-event-123");

        if let OispEvent::AiResponse(resp) = deserialized {
            assert_eq!(resp.data.request_id, "req-123");
            assert_eq!(resp.data.usage.as_ref().unwrap().total_tokens, Some(150));
            assert_eq!(resp.data.finish_reason, Some(FinishReason::Stop));
        } else {
            panic!("Expected AiResponse event");
        }
    }

    #[test]
    fn test_event_type_methods() {
        let request = OispEvent::AiRequest(AiRequestEvent {
            envelope: create_test_envelope(),
            data: AiRequestData {
                request_id: "req-test".to_string(),
                provider: None,
                model: None,
                auth: None,
                request_type: Some(RequestType::Completion),
                streaming: None,
                messages: vec![],
                messages_count: None,
                has_system_prompt: None,
                system_prompt_hash: None,
                tools: vec![],
                tools_count: None,
                tool_choice: None,
                parameters: None,
                has_rag_context: None,
                has_images: None,
                image_count: None,
                estimated_tokens: None,
                conversation: None,
                agent: None,
            },
        });

        assert_eq!(request.event_type(), "ai.request");
        assert!(request.is_ai_event());

        let process_exec = OispEvent::ProcessExec(ProcessExecEvent {
            envelope: create_test_envelope(),
            data: ProcessExecData {
                exe: "/bin/ls".to_string(),
                args: vec![],
                cwd: None,
                env: Default::default(),
                interpreter: None,
                script_path: None,
                is_shell: None,
                is_script: None,
                is_interactive: None,
                binary_hash: None,
                code_signature: None,
            },
        });

        assert_eq!(process_exec.event_type(), "process.exec");
        assert!(!process_exec.is_ai_event());
    }

    #[test]
    fn test_event_category_from_event_type() {
        assert_eq!(
            EventCategory::from_event_type("ai.request"),
            Some(EventCategory::Ai)
        );
        assert_eq!(
            EventCategory::from_event_type("agent.tool_call"),
            Some(EventCategory::Agent)
        );
        assert_eq!(
            EventCategory::from_event_type("process.exec"),
            Some(EventCategory::Process)
        );
        assert_eq!(
            EventCategory::from_event_type("file.write"),
            Some(EventCategory::File)
        );
        assert_eq!(
            EventCategory::from_event_type("network.connect"),
            Some(EventCategory::Network)
        );
        assert_eq!(EventCategory::from_event_type("unknown.type"), None);
    }
}
