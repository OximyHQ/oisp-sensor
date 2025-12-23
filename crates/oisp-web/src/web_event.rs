//! WebEvent - Simplified event format for frontend consumption
//!
//! This is a flattened, frontend-optimized event format separate from the OISP spec.
//! Key design principle: PID is the primary organizing key (like AgentSight).

use oisp_core::events::OispEvent;
use serde::Serialize;

/// Simplified event format for frontend consumption.
///
/// Unlike the full OISP spec which has nested structures and optional fields,
/// WebEvent is flat and guarantees pid/comm are always present.
/// This enables the frontend to easily group events by process.
#[derive(Debug, Clone, Serialize)]
pub struct WebEvent {
    /// Unique event ID
    pub id: String,

    /// Unix timestamp in milliseconds
    pub timestamp: i64,

    /// Event type for frontend display
    #[serde(rename = "type")]
    pub event_type: WebEventType,

    /// Process ID - REQUIRED, primary grouping key
    pub pid: u32,

    /// Parent process ID - for building process tree hierarchy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppid: Option<u32>,

    /// Process name - REQUIRED (e.g., "claude", "python3", "node")
    pub comm: String,

    /// Event-specific payload (simplified for frontend)
    pub data: WebEventData,
}

/// Event types for frontend display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebEventType {
    /// AI prompt/request
    AiPrompt,
    /// AI response
    AiResponse,
    /// Process started
    ProcessExec,
    /// Process exited
    ProcessExit,
    /// File opened
    FileOpen,
    /// File written
    FileWrite,
    /// Network connection
    NetworkConnect,
    /// Generic/other event
    Other,
}

/// Event-specific data payload
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum WebEventData {
    AiPrompt(AiPromptData),
    AiResponse(AiResponseData),
    ProcessExec(ProcessExecData),
    ProcessExit(ProcessExitData),
    FileOp(FileOpData),
    Network(NetworkData),
    Generic(serde_json::Value),
}

/// AI prompt/request data
#[derive(Debug, Clone, Serialize)]
pub struct AiPromptData {
    /// AI provider (openai, anthropic, etc.)
    pub provider: String,
    /// Model ID
    pub model: String,
    /// Number of messages in the request
    pub message_count: usize,
    /// Whether streaming is enabled
    pub streaming: bool,
    /// Number of tools available
    pub tool_count: usize,
    /// Estimated token count (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<u64>,
    /// Request endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

/// AI response data
#[derive(Debug, Clone, Serialize)]
pub struct AiResponseData {
    /// AI provider
    pub provider: String,
    /// Model ID
    pub model: String,
    /// Response latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    /// Input tokens used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Output tokens generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Number of tool calls in response
    pub tool_calls: usize,
    /// Whether response was successful
    pub success: bool,
}

/// Process execution data
#[derive(Debug, Clone, Serialize)]
pub struct ProcessExecData {
    /// Executable path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// Full command line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,
    /// Current working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Process exit data
#[derive(Debug, Clone, Serialize)]
pub struct ProcessExitData {
    /// Exit code
    pub exit_code: i32,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// File operation data
#[derive(Debug, Clone, Serialize)]
pub struct FileOpData {
    /// File path
    pub path: String,
    /// Operation type (open, read, write, close)
    pub operation: String,
    /// Bytes transferred (for read/write)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

/// Network connection data
#[derive(Debug, Clone, Serialize)]
pub struct NetworkData {
    /// Remote address
    pub remote_addr: String,
    /// Remote port
    pub remote_port: u16,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

impl WebEvent {
    /// Convert an OispEvent to a WebEvent for frontend consumption
    pub fn from_oisp_event(event: &OispEvent) -> Self {
        let envelope = event.envelope();

        // Extract process info - pid and comm are REQUIRED
        let (pid, ppid, comm) = Self::extract_process_info(envelope);

        // Map event type and build data payload
        let (event_type, data) = Self::build_event_data(event);

        Self {
            id: envelope.event_id.clone(),
            timestamp: envelope.ts.timestamp_millis(),
            event_type,
            pid,
            ppid,
            comm,
            data,
        }
    }

    /// Extract process information from envelope
    /// Returns (pid, ppid, comm) - pid and comm always have values
    fn extract_process_info(
        envelope: &oisp_core::events::EventEnvelope,
    ) -> (u32, Option<u32>, String) {
        if let Some(proc) = &envelope.process {
            let pid = proc.pid;
            let ppid = proc.ppid;
            let comm = proc
                .name
                .clone()
                .or_else(|| {
                    proc.exe
                        .as_ref()
                        .and_then(|e| e.rsplit('/').next().map(String::from))
                })
                .unwrap_or_else(|| "unknown".to_string());
            (pid, ppid, comm)
        } else {
            // Fallback: no process info available
            (0, None, "unknown".to_string())
        }
    }

    /// Build event type and data payload from OispEvent
    fn build_event_data(event: &OispEvent) -> (WebEventType, WebEventData) {
        match event {
            OispEvent::AiRequest(e) => {
                let provider = e
                    .data
                    .provider
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let model = e
                    .data
                    .model
                    .as_ref()
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let endpoint = e.data.provider.as_ref().and_then(|p| p.endpoint.clone());

                (
                    WebEventType::AiPrompt,
                    WebEventData::AiPrompt(AiPromptData {
                        provider,
                        model,
                        message_count: e.data.messages_count.unwrap_or(e.data.messages.len()),
                        streaming: e.data.streaming.unwrap_or(false),
                        tool_count: e.data.tools_count.unwrap_or(e.data.tools.len()),
                        estimated_tokens: e.data.estimated_tokens,
                        endpoint,
                    }),
                )
            }

            OispEvent::AiResponse(e) => {
                let provider = e
                    .data
                    .provider
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let model = e
                    .data
                    .model
                    .as_ref()
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let (input_tokens, output_tokens) = e
                    .data
                    .usage
                    .as_ref()
                    .map(|u| (u.prompt_tokens, u.completion_tokens))
                    .unwrap_or((None, None));

                let finish_reason = e.data.finish_reason.as_ref().map(|fr| format!("{:?}", fr));

                (
                    WebEventType::AiResponse,
                    WebEventData::AiResponse(AiResponseData {
                        provider,
                        model,
                        latency_ms: e.data.latency_ms.map(|ms| ms as i64),
                        input_tokens,
                        output_tokens,
                        finish_reason,
                        tool_calls: e.data.tool_calls_count.unwrap_or(e.data.tool_calls.len()),
                        success: e.data.success.unwrap_or(true),
                    }),
                )
            }

            OispEvent::ProcessExec(e) => {
                // Build cmdline from exe + args
                let cmdline = if e.data.args.is_empty() {
                    Some(e.data.exe.clone())
                } else {
                    Some(format!("{} {}", e.data.exe, e.data.args.join(" ")))
                };

                (
                    WebEventType::ProcessExec,
                    WebEventData::ProcessExec(ProcessExecData {
                        exe: Some(e.data.exe.clone()),
                        cmdline,
                        cwd: e.data.cwd.clone(),
                    }),
                )
            }

            OispEvent::ProcessExit(e) => (
                WebEventType::ProcessExit,
                WebEventData::ProcessExit(ProcessExitData {
                    exit_code: e.data.exit_code,
                    duration_ms: e.data.runtime_ms.map(|ms| ms as i64),
                }),
            ),

            OispEvent::FileOpen(e) => (
                WebEventType::FileOpen,
                WebEventData::FileOp(FileOpData {
                    path: e.data.path.clone(),
                    operation: "open".to_string(),
                    bytes: None,
                }),
            ),

            OispEvent::FileWrite(e) => (
                WebEventType::FileWrite,
                WebEventData::FileOp(FileOpData {
                    path: e.data.path.clone(),
                    operation: "write".to_string(),
                    bytes: e.data.bytes_written,
                }),
            ),

            OispEvent::NetworkConnect(e) => {
                let remote_addr = e
                    .data
                    .dest
                    .ip
                    .clone()
                    .or_else(|| e.data.dest.domain.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let remote_port = e.data.dest.port.unwrap_or(0);

                let protocol = e
                    .data
                    .protocol
                    .as_ref()
                    .map(|p| format!("{:?}", p).to_lowercase())
                    .unwrap_or_else(|| "tcp".to_string());

                (
                    WebEventType::NetworkConnect,
                    WebEventData::Network(NetworkData {
                        remote_addr,
                        remote_port,
                        protocol,
                    }),
                )
            }

            // Handle other event types as generic
            _ => {
                let data = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
                (WebEventType::Other, WebEventData::Generic(data))
            }
        }
    }
}

/// Response wrapper for the web-events API
#[derive(Debug, Serialize)]
pub struct WebEventsResponse {
    /// List of events in WebEvent format
    pub events: Vec<WebEvent>,
    /// Total number of events available
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_event_serialization() {
        let event = WebEvent {
            id: "test-123".to_string(),
            timestamp: 1703347200000,
            event_type: WebEventType::AiPrompt,
            pid: 1234,
            ppid: Some(1),
            comm: "claude".to_string(),
            data: WebEventData::AiPrompt(AiPromptData {
                provider: "anthropic".to_string(),
                model: "claude-3-opus".to_string(),
                message_count: 5,
                streaming: true,
                tool_count: 3,
                estimated_tokens: Some(1500),
                endpoint: Some("https://api.anthropic.com/v1/messages".to_string()),
            }),
        };

        let json = serde_json::to_string_pretty(&event).unwrap();
        assert!(json.contains("\"type\": \"ai_prompt\""));
        assert!(json.contains("\"pid\": 1234"));
        assert!(json.contains("\"comm\": \"claude\""));
        assert!(json.contains("\"provider\": \"anthropic\""));
    }

    #[test]
    fn test_web_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&WebEventType::AiPrompt).unwrap(),
            "\"ai_prompt\""
        );
        assert_eq!(
            serde_json::to_string(&WebEventType::AiResponse).unwrap(),
            "\"ai_response\""
        );
        assert_eq!(
            serde_json::to_string(&WebEventType::ProcessExec).unwrap(),
            "\"process_exec\""
        );
    }
}
