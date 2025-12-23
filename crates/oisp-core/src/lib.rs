//! OISP Core - Event types, plugin traits, and pipeline orchestration
//!
//! This crate provides the foundational types and abstractions for the OISP Sensor:
//!
//! - **Events**: OISP-spec compliant event types
//! - **Plugins**: Trait definitions for all pipeline stages
//! - **Pipeline**: Event routing and orchestration
//! - **Providers**: AI provider detection and metadata

pub mod events;
pub mod pipeline;
pub mod plugins;
pub mod providers;
pub mod redaction;
pub mod trace;

// Re-export commonly used types
pub use events::{
    Actor, Confidence, EventEnvelope, EventType, Host, OispEvent, ProcessInfo, Source,
};
pub use pipeline::{Pipeline, PipelineConfig};
pub use plugins::{
    ActionPlugin, CapturePlugin, DecodePlugin, EnrichPlugin, ExportPlugin, Plugin, PluginInfo,
};
pub use providers::{Provider, ProviderRegistry};
pub use trace::{AgentTrace, Span, SpanKind};

/// OISP specification version this crate implements
pub const OISP_VERSION: &str = "0.1";

/// Sensor version
pub const SENSOR_VERSION: &str = env!("CARGO_PKG_VERSION");
