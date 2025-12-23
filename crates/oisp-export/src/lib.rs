//! Export plugins for OISP Sensor
//!
//! This crate provides various export destinations for OISP events:
//!
//! - **JSONL** (default): Writes events to a local JSONL file
//! - **WebSocket** (default): Broadcasts events to WebSocket clients for real-time UI
//! - **OTLP** (optional): Exports to OpenTelemetry collectors via gRPC or HTTP
//! - **Kafka** (optional): Publishes events to Apache Kafka topics
//! - **Webhook** (optional): POSTs events to HTTP endpoints
//!
//! ## Feature Flags
//!
//! - `jsonl` - JSONL file export (default)
//! - `websocket` - WebSocket export (default)
//! - `otlp` - OpenTelemetry Protocol export
//! - `kafka` - Apache Kafka export
//! - `webhook` - HTTP webhook export

pub mod jsonl;
pub mod websocket;

#[cfg(feature = "otlp")]
pub mod otlp;

#[cfg(feature = "kafka")]
pub mod kafka;

#[cfg(feature = "webhook")]
pub mod webhook;

// Re-exports
pub use jsonl::{JsonlExporter, JsonlExporterConfig};
pub use websocket::{WebSocketExporter, WebSocketExporterConfig};

#[cfg(feature = "otlp")]
pub use otlp::{OtlpExporter, OtlpExporterConfig, OtlpTransport};

#[cfg(feature = "kafka")]
pub use kafka::{KafkaCompression, KafkaExporter, KafkaExporterConfig, SaslMechanism};

#[cfg(feature = "webhook")]
pub use webhook::{
    WebhookAuth, WebhookExporter, WebhookExporterConfig, WebhookMethod, WebhookStats,
};
