//! Plugin traits for all pipeline stages
//!
//! OISP Sensor is built on a plugin architecture. Every pipeline stage
//! is defined as a trait, enabling extensibility and custom implementations.

use crate::events::OispEvent;
use async_trait::async_trait;
use std::any::Any;
use thiserror::Error;
use tokio::sync::mpsc;

/// Plugin error type
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin operation failed: {0}")]
    OperationFailed(String),

    #[error("Plugin not supported on this platform")]
    NotSupported,

    #[error("Plugin configuration error: {0}")]
    ConfigurationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;

/// Basic plugin information
pub trait PluginInfo {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin version
    fn version(&self) -> &str;

    /// Plugin description
    fn description(&self) -> &str {
        ""
    }

    /// Whether plugin is available on this platform
    fn is_available(&self) -> bool {
        true
    }
}

/// Base plugin trait - all plugins implement this
pub trait Plugin: PluginInfo + Send + Sync {
    /// Initialize the plugin with configuration
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        let _ = config;
        Ok(())
    }

    /// Shutdown the plugin
    fn shutdown(&mut self) -> PluginResult<()> {
        Ok(())
    }

    /// Get plugin as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get plugin as mutable Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin configuration
#[derive(Debug, Clone, Default)]
pub struct PluginConfig {
    /// Raw configuration values
    pub values: std::collections::HashMap<String, serde_json::Value>,
}

impl PluginConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.values
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: serde::Serialize>(&mut self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.values.insert(key.to_string(), v);
        }
    }
}

// =============================================================================
// CAPTURE PLUGINS
// =============================================================================

/// Raw captured data before decoding
#[derive(Debug, Clone)]
pub struct RawCaptureEvent {
    /// Unique ID for this raw event
    pub id: String,

    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,

    /// Event kind
    pub kind: RawEventKind,

    /// Process ID
    pub pid: u32,

    /// Thread ID
    pub tid: Option<u32>,

    /// Raw data bytes
    pub data: Vec<u8>,

    /// Metadata
    pub metadata: RawEventMetadata,
}

/// Kind of raw event
#[derive(Debug, Clone)]
pub enum RawEventKind {
    /// SSL/TLS write (outgoing data)
    SslWrite,
    /// SSL/TLS read (incoming data)
    SslRead,
    /// Process execution
    ProcessExec,
    /// Process exit
    ProcessExit,
    /// Process fork
    ProcessFork,
    /// File open
    FileOpen,
    /// File read
    FileRead,
    /// File write
    FileWrite,
    /// File close
    FileClose,
    /// Network connect
    NetworkConnect,
    /// Network accept
    NetworkAccept,
    /// Network send
    NetworkSend,
    /// Network receive
    NetworkRecv,
    /// DNS query
    DnsQuery,
    /// Other/custom
    Other(String),
}

/// Metadata for raw events
#[derive(Debug, Clone, Default)]
pub struct RawEventMetadata {
    /// Executable path
    pub exe: Option<String>,
    /// Process name
    pub comm: Option<String>,
    /// Parent PID
    pub ppid: Option<u32>,
    /// User ID
    pub uid: Option<u32>,
    /// File descriptor
    pub fd: Option<i32>,
    /// File path
    pub path: Option<String>,
    /// Remote address
    pub remote_addr: Option<String>,
    /// Remote port
    pub remote_port: Option<u16>,
    /// Local address
    pub local_addr: Option<String>,
    /// Local port
    pub local_port: Option<u16>,
    /// macOS bundle identifier (e.g., "com.todesktop.230313mzl4w4u92")
    pub bundle_id: Option<String>,
    /// Additional data
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Capture plugin - produces raw events from system capture
#[async_trait]
pub trait CapturePlugin: Plugin {
    /// Start capturing events
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()>;

    /// Stop capturing
    async fn stop(&mut self) -> PluginResult<()>;

    /// Check if capture is running
    fn is_running(&self) -> bool;

    /// Get capture statistics
    fn stats(&self) -> CaptureStats {
        CaptureStats::default()
    }
}

/// Capture statistics
#[derive(Debug, Clone, Default)]
pub struct CaptureStats {
    /// Total events captured
    pub events_captured: u64,
    /// Events dropped (buffer full, etc.)
    pub events_dropped: u64,
    /// Bytes captured
    pub bytes_captured: u64,
    /// Errors encountered
    pub errors: u64,
}

// =============================================================================
// DECODE PLUGINS
// =============================================================================

/// Decode plugin - transforms raw events into structured OISP events
#[async_trait]
pub trait DecodePlugin: Plugin {
    /// Check if this decoder can handle the raw event
    fn can_decode(&self, raw: &RawCaptureEvent) -> bool;

    /// Decode raw event into OISP event(s)
    /// May return multiple events (e.g., HTTP request + AI request)
    async fn decode(&self, raw: RawCaptureEvent) -> PluginResult<Vec<OispEvent>>;

    /// Priority for decoder selection (higher = tried first)
    fn priority(&self) -> i32 {
        0
    }
}

// =============================================================================
// ENRICH PLUGINS
// =============================================================================

/// Enrich plugin - adds context to events
#[async_trait]
pub trait EnrichPlugin: Plugin {
    /// Enrich an event with additional context
    async fn enrich(&self, event: &mut OispEvent) -> PluginResult<()>;

    /// Whether this enricher applies to the event
    fn applies_to(&self, event: &OispEvent) -> bool {
        let _ = event;
        true
    }
}

// =============================================================================
// ACTION PLUGINS
// =============================================================================

/// Action to take on an event
#[derive(Debug, Clone)]
pub enum EventAction {
    /// Pass the event through unchanged
    Pass,
    /// Pass with modifications (event was mutated)
    Modified,
    /// Drop the event
    Drop,
    /// Replace with different event(s)
    Replace(Vec<OispEvent>),
}

/// Action plugin - filters, transforms, or redacts events
#[async_trait]
pub trait ActionPlugin: Plugin {
    /// Process an event and decide what to do with it
    async fn process(&self, event: OispEvent) -> PluginResult<(OispEvent, EventAction)>;

    /// Whether this action applies to the event
    fn applies_to(&self, event: &OispEvent) -> bool {
        let _ = event;
        true
    }
}

// =============================================================================
// EXPORT PLUGINS
// =============================================================================

/// Export plugin - sends events to destinations
#[async_trait]
pub trait ExportPlugin: Plugin {
    /// Export an event
    async fn export(&self, event: &OispEvent) -> PluginResult<()>;

    /// Export multiple events (batch)
    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()> {
        for event in events {
            self.export(event).await?;
        }
        Ok(())
    }

    /// Flush any buffered events
    async fn flush(&self) -> PluginResult<()> {
        Ok(())
    }
}

// =============================================================================
// PLUGIN REGISTRY
// =============================================================================

/// Registry of all available plugins
#[derive(Default)]
pub struct PluginRegistry {
    pub capture: Vec<Box<dyn CapturePlugin>>,
    pub decode: Vec<Box<dyn DecodePlugin>>,
    pub enrich: Vec<Box<dyn EnrichPlugin>>,
    pub action: Vec<Box<dyn ActionPlugin>>,
    pub export: Vec<Box<dyn ExportPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_capture(&mut self, plugin: Box<dyn CapturePlugin>) {
        self.capture.push(plugin);
    }

    pub fn register_decode(&mut self, plugin: Box<dyn DecodePlugin>) {
        self.decode.push(plugin);
    }

    pub fn register_enrich(&mut self, plugin: Box<dyn EnrichPlugin>) {
        self.enrich.push(plugin);
    }

    pub fn register_action(&mut self, plugin: Box<dyn ActionPlugin>) {
        self.action.push(plugin);
    }

    pub fn register_export(&mut self, plugin: Box<dyn ExportPlugin>) {
        self.export.push(plugin);
    }
}
