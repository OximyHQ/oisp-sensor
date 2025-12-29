//! OISP Core - Event types, plugin traits, and pipeline orchestration
//!
//! This crate provides the foundational types and abstractions for the OISP Sensor:
//!
//! - **Events**: OISP-spec compliant event types
//! - **Plugins**: Trait definitions for all pipeline stages
//! - **Pipeline**: Event routing and orchestration
//! - **Providers**: AI provider detection and metadata
//! - **Config**: Configuration loading and management
//! - **Enrichers**: Built-in enrichment plugins (host, process tree)
//! - **Actions**: Built-in action plugins (redaction)
//! - **Trace**: Event correlation and trace building

pub mod actions;
pub mod config;
pub mod enrichers;
pub mod events;
pub mod metrics;
pub mod pipeline;
pub mod plugins;
pub mod providers;
pub mod redaction;
pub mod replay;
pub mod spec;
pub mod trace;

// Re-export commonly used types
pub use actions::RedactionPlugin;
pub use config::{
    spawn_sighup_reload_handler, CaptureSettings, ConfigError, ConfigLoader, ConfigResult,
    CorrelationSettings, ExportSettings, JsonlExportConfig, KafkaExportConfig, OtlpExportConfig,
    OximyExportConfig, RedactionSettings, SensorConfig, SensorSettings, SharedConfig, WebSettings,
    WebSocketExportConfig, WebhookExportConfig,
};
pub use enrichers::{HostEnricher, ProcessTreeEnricher};
pub use events::{
    Actor, Confidence, EventEnvelope, EventType, Host, OispEvent, ProcessInfo, Source,
};
pub use metrics::{create_metrics, MetricsCollector, SharedMetrics};
pub use pipeline::{Pipeline, PipelineConfig};
pub use plugins::{
    ActionPlugin, CapturePlugin, DecodePlugin, EnrichPlugin, ExportPlugin, Plugin, PluginInfo,
};
pub use providers::{Provider, ProviderRegistry};
pub use replay::{EventReplay, ReplayConfig};
pub use spec::{
    bundle_refresh_interval, bundle_url, DynamicProviderRegistry, OispSpecBundle, SpecLoader,
    DEFAULT_BUNDLE_URL,
};
pub use trace::{AgentTrace, CorrelationConfig, Span, SpanKind};

use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global spec loader for getting version and other spec-driven values
static GLOBAL_SPEC_LOADER: Lazy<SpecLoader> = Lazy::new(SpecLoader::new);

/// Cached version string from spec bundle (avoids lifetime issues)
static CACHED_SPEC_VERSION: Lazy<String> =
    Lazy::new(|| GLOBAL_SPEC_LOADER.bundle().version.clone());

/// Get the OISP specification version from the loaded spec bundle
/// Falls back to embedded bundle version if network fetch fails
pub fn oisp_version() -> &'static str {
    CACHED_SPEC_VERSION.as_str()
}

/// Get the global spec loader (for sharing across components)
pub fn global_spec_loader() -> &'static SpecLoader {
    &GLOBAL_SPEC_LOADER
}

/// Get the global spec bundle
pub fn global_spec_bundle() -> Arc<OispSpecBundle> {
    GLOBAL_SPEC_LOADER.bundle()
}

/// OISP specification version this crate implements (constant for backward compatibility)
/// Prefer using `oisp_version()` which reads from spec bundle
pub const OISP_VERSION: &str = "0.1";

/// Sensor version
pub const SENSOR_VERSION: &str = env!("CARGO_PKG_VERSION");
