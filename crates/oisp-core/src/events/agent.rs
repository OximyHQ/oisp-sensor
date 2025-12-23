//! Agent-related events - tool calls and results

use super::ai::{RedactedContent, ToolArguments};
use super::envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent tool call event - when an agent invokes a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    #[serde(flatten)]
    pub data: AgentToolCallData,
}

/// Agent tool call data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallData {
    /// Tool call ID
    pub tool_call_id: String,

    /// Related AI request that triggered this
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Tool name
    pub tool_name: String,

    /// Tool type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,

    /// Arguments passed to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<ToolArguments>,

    /// Hash of arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments_hash: Option<String>,

    /// Parsed/structured arguments (for known tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_arguments: Option<ParsedToolArguments>,
}

/// Parsed tool arguments for known tool types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool_type")]
pub enum ParsedToolArguments {
    /// File read operation
    #[serde(rename = "read_file")]
    ReadFile {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        encoding: Option<String>,
    },

    /// File write operation
    #[serde(rename = "write_file")]
    WriteFile {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content_hash: Option<String>,
    },

    /// File edit operation
    #[serde(rename = "edit_file")]
    EditFile {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        changes_count: Option<usize>,
    },

    /// Command execution
    #[serde(rename = "execute")]
    Execute {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },

    /// Search operation
    #[serde(rename = "search")]
    Search {
        query: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    },

    /// Web fetch
    #[serde(rename = "web_fetch")]
    WebFetch {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<String>,
    },

    /// Unknown/other tool
    #[serde(rename = "other")]
    Other {
        #[serde(flatten)]
        raw: HashMap<String, serde_json::Value>,
    },
}

/// Agent tool result event - result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultEvent {
    #[serde(flatten)]
    pub envelope: EventEnvelope,

    #[serde(flatten)]
    pub data: AgentToolResultData,
}

/// Agent tool result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultData {
    /// Tool call ID this result is for
    pub tool_call_id: String,

    /// Tool name
    pub tool_name: String,

    /// Whether execution succeeded
    pub success: bool,

    /// Result content (may be redacted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ToolResultContent>,

    /// Result hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_hash: Option<String>,

    /// Result length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_length: Option<usize>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Side effects observed
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<ToolSideEffect>,
}

/// Tool result content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain text result
    Text(String),
    /// Structured result
    Structured(serde_json::Value),
    /// Redacted result
    Redacted(RedactedContent),
}

/// Side effect of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSideEffect {
    /// Type of side effect
    #[serde(rename = "type")]
    pub effect_type: SideEffectType,

    /// Description or details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Related event ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

/// Types of side effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectType {
    /// File was created
    FileCreated,
    /// File was modified
    FileModified,
    /// File was deleted
    FileDeleted,
    /// Process was spawned
    ProcessSpawned,
    /// Network connection made
    NetworkConnection,
    /// Environment modified
    EnvironmentModified,
    /// Other side effect
    Other,
}
