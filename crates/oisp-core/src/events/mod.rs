//! OISP Event types - conforming to oisp-spec v0.1
//!
//! All events share a common envelope structure with event-type-specific data.

pub mod envelope;
pub mod ai;
pub mod agent;
pub mod process;
pub mod file;
pub mod network;

pub use envelope::*;
pub use ai::*;
pub use agent::*;
pub use process::*;
pub use file::*;
pub use network::*;

use serde::{Deserialize, Serialize};

/// All possible OISP event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum OispEvent {
    // AI events
    #[serde(rename = "ai.request")]
    AiRequest(AiRequestEvent),
    
    #[serde(rename = "ai.response")]
    AiResponse(AiResponseEvent),
    
    #[serde(rename = "ai.streaming_chunk")]
    AiStreamingChunk(AiStreamingChunkEvent),
    
    #[serde(rename = "ai.embedding")]
    AiEmbedding(AiEmbeddingEvent),
    
    // Agent events
    #[serde(rename = "agent.tool_call")]
    AgentToolCall(AgentToolCallEvent),
    
    #[serde(rename = "agent.tool_result")]
    AgentToolResult(AgentToolResultEvent),
    
    // Process events
    #[serde(rename = "process.exec")]
    ProcessExec(ProcessExecEvent),
    
    #[serde(rename = "process.exit")]
    ProcessExit(ProcessExitEvent),
    
    #[serde(rename = "process.fork")]
    ProcessFork(ProcessForkEvent),
    
    // File events
    #[serde(rename = "file.open")]
    FileOpen(FileOpenEvent),
    
    #[serde(rename = "file.read")]
    FileRead(FileReadEvent),
    
    #[serde(rename = "file.write")]
    FileWrite(FileWriteEvent),
    
    #[serde(rename = "file.close")]
    FileClose(FileCloseEvent),
    
    // Network events
    #[serde(rename = "network.connect")]
    NetworkConnect(NetworkConnectEvent),
    
    #[serde(rename = "network.accept")]
    NetworkAccept(NetworkAcceptEvent),
    
    #[serde(rename = "network.flow")]
    NetworkFlow(NetworkFlowEvent),
    
    #[serde(rename = "network.dns")]
    NetworkDns(NetworkDnsEvent),
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
            _ => None,
        }
    }
}

