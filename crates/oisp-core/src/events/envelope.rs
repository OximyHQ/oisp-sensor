//! Event envelope - the common wrapper for all OISP events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The canonical envelope for all OISP events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// OISP specification version
    pub oisp_version: String,

    /// Unique event identifier (ULID recommended)
    pub event_id: String,

    /// Event type (e.g., "ai.request", "process.exec")
    pub event_type: String,

    /// Event timestamp
    pub ts: DateTime<Utc>,

    /// Monotonic timestamp in nanoseconds (for precise ordering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_mono: Option<u64>,

    /// Host/device context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<Host>,

    /// User/identity context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Actor>,

    /// Process context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessInfo>,

    /// Capture source/provenance
    pub source: Source,

    /// Confidence and completeness metadata
    pub confidence: Confidence,

    /// Additional attributes
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, serde_json::Value>,

    /// Namespaced extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub ext: HashMap<String, serde_json::Value>,

    /// Related event IDs for correlation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_events: Vec<RelatedEvent>,

    /// OpenTelemetry trace context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_context: Option<TraceContext>,
}

impl EventEnvelope {
    /// Create a new envelope with minimal required fields
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            oisp_version: crate::OISP_VERSION.to_string(),
            event_id: ulid::Ulid::new().to_string(),
            event_type: event_type.into(),
            ts: Utc::now(),
            ts_mono: None,
            host: None,
            actor: None,
            process: None,
            source: Source::default(),
            confidence: Confidence::default(),
            attrs: HashMap::new(),
            ext: HashMap::new(),
            related_events: Vec::new(),
            trace_context: None,
        }
    }

    /// Set the host context
    pub fn with_host(mut self, host: Host) -> Self {
        self.host = Some(host);
        self
    }

    /// Set the actor context
    pub fn with_actor(mut self, actor: Actor) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set the process context
    pub fn with_process(mut self, process: ProcessInfo) -> Self {
        self.process = Some(process);
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    /// Add a related event
    pub fn with_related(mut self, event_id: String, relationship: Relationship) -> Self {
        self.related_events.push(RelatedEvent {
            event_id,
            relationship,
        });
        self
    }
}

/// Host/device context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    /// Hostname
    pub hostname: String,

    /// Unique device identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// Operating system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,

    /// OS version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,

    /// Architecture
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl Host {
    /// Create host info from current system
    pub fn current() -> Self {
        Self {
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            device_id: None,
            os: Some(std::env::consts::OS.to_string()),
            os_version: None,
            arch: Some(std::env::consts::ARCH.to_string()),
        }
    }
}

/// User/identity context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Unix UID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,

    /// Username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Group ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,

    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Identity from IdP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
}

/// Identity from identity provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// IdP name (e.g., "okta", "azure_ad")
    pub provider: String,

    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User ID from IdP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Process context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,

    /// Parent process ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppid: Option<u32>,

    /// Executable path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,

    /// Process name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Full command line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,

    /// Current working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Thread ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<u32>,

    /// Container ID (if in container)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,

    /// Binary hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,

    /// Code signing information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_signature: Option<CodeSignature>,
}

/// Code signing information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeSignature {
    /// Whether the binary is signed
    pub signed: bool,

    /// Signer identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,

    /// Team ID (macOS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,

    /// Whether signature is valid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

/// Capture source/provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Collector name
    pub collector: String,

    /// Collector version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collector_version: Option<String>,

    /// Capture method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_method: Option<CaptureMethod>,

    /// Specific capture point
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_point: Option<String>,

    /// Sensor host if different from event host
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensor_host: Option<String>,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            collector: "oisp-sensor".to_string(),
            collector_version: Some(crate::SENSOR_VERSION.to_string()),
            capture_method: None,
            capture_point: None,
            sensor_host: None,
        }
    }
}

/// How the event was captured
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMethod {
    /// eBPF tracepoint
    EbpfTracepoint,
    /// eBPF kprobe
    EbpfKprobe,
    /// eBPF uprobe
    EbpfUprobe,
    /// DTrace (macOS/Solaris)
    Dtrace,
    /// Event Tracing for Windows
    Etw,
    /// Syscall interception
    SyscallIntercept,
    /// TLS boundary capture
    TlsBoundary,
    /// MITM proxy
    MitmProxy,
    /// Browser extension
    BrowserExtension,
    /// SDK instrumentation
    SdkInstrumentation,
    /// Vendor API
    VendorApi,
    /// Vendor audit log
    VendorAuditLog,
    /// Log parsing
    LogParsing,
    /// macOS Endpoint Security Framework
    EndpointSecurity,
    /// macOS Network Extension
    NetworkExtension,
    /// Other
    Other,
}

/// Confidence and completeness metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confidence {
    /// Confidence level in the data
    pub level: ConfidenceLevel,

    /// How complete is the captured data
    pub completeness: Completeness,

    /// Reasons for the confidence/completeness assessment
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,

    /// Source of content (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_source: Option<String>,

    /// How AI activity was detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_detection_method: Option<String>,
}

impl Default for Confidence {
    fn default() -> Self {
        Self {
            level: ConfidenceLevel::Medium,
            completeness: Completeness::Partial,
            reasons: Vec::new(),
            content_source: None,
            ai_detection_method: None,
        }
    }
}

/// Confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    /// Low confidence - inferred or heuristic
    Low,
    /// Medium confidence - partial data
    Medium,
    /// High confidence - complete data from reliable source
    High,
}

/// Data completeness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// Only metadata available
    MetadataOnly,
    /// Partial content captured
    Partial,
    /// Full content captured
    Full,
}

/// Related event reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEvent {
    /// Related event ID
    pub event_id: String,
    /// Relationship type
    pub relationship: Relationship,
}

/// Relationship between events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relationship {
    Parent,
    Child,
    CausedBy,
    Causes,
    Related,
}

/// OpenTelemetry trace context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// W3C Trace ID (32 hex chars)
    pub trace_id: String,
    /// W3C Span ID (16 hex chars)
    pub span_id: String,
    /// Trace flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_flags: Option<u8>,
}

/// Event type enumeration for type-safe handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    // AI
    AiRequest,
    AiResponse,
    AiStreamingChunk,
    AiEmbedding,
    // Agent
    AgentToolCall,
    AgentToolResult,
    // Process
    ProcessExec,
    ProcessExit,
    ProcessFork,
    // File
    FileOpen,
    FileRead,
    FileWrite,
    FileClose,
    // Network
    NetworkConnect,
    NetworkAccept,
    NetworkFlow,
    NetworkDns,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::AiRequest => "ai.request",
            EventType::AiResponse => "ai.response",
            EventType::AiStreamingChunk => "ai.streaming_chunk",
            EventType::AiEmbedding => "ai.embedding",
            EventType::AgentToolCall => "agent.tool_call",
            EventType::AgentToolResult => "agent.tool_result",
            EventType::ProcessExec => "process.exec",
            EventType::ProcessExit => "process.exit",
            EventType::ProcessFork => "process.fork",
            EventType::FileOpen => "file.open",
            EventType::FileRead => "file.read",
            EventType::FileWrite => "file.write",
            EventType::FileClose => "file.close",
            EventType::NetworkConnect => "network.connect",
            EventType::NetworkAccept => "network.accept",
            EventType::NetworkFlow => "network.flow",
            EventType::NetworkDns => "network.dns",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ai.request" => Some(EventType::AiRequest),
            "ai.response" => Some(EventType::AiResponse),
            "ai.streaming_chunk" => Some(EventType::AiStreamingChunk),
            "ai.embedding" => Some(EventType::AiEmbedding),
            "agent.tool_call" => Some(EventType::AgentToolCall),
            "agent.tool_result" => Some(EventType::AgentToolResult),
            "process.exec" => Some(EventType::ProcessExec),
            "process.exit" => Some(EventType::ProcessExit),
            "process.fork" => Some(EventType::ProcessFork),
            "file.open" => Some(EventType::FileOpen),
            "file.read" => Some(EventType::FileRead),
            "file.write" => Some(EventType::FileWrite),
            "file.close" => Some(EventType::FileClose),
            "network.connect" => Some(EventType::NetworkConnect),
            "network.accept" => Some(EventType::NetworkAccept),
            "network.flow" => Some(EventType::NetworkFlow),
            "network.dns" => Some(EventType::NetworkDns),
            _ => None,
        }
    }
}
