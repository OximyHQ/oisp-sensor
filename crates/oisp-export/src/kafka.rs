//! Kafka exporter
//!
//! Exports OISP events to Apache Kafka topics.
//! Supports SASL authentication, TLS, and batching.

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ExportPlugin, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;

/// SASL authentication mechanism
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaslMechanism {
    /// No authentication
    #[default]
    None,
    /// PLAIN authentication
    Plain,
    /// SCRAM-SHA-256
    ScramSha256,
    /// SCRAM-SHA-512
    ScramSha512,
}

impl SaslMechanism {
    fn as_str(&self) -> Option<&'static str> {
        match self {
            SaslMechanism::None => None,
            SaslMechanism::Plain => Some("PLAIN"),
            SaslMechanism::ScramSha256 => Some("SCRAM-SHA-256"),
            SaslMechanism::ScramSha512 => Some("SCRAM-SHA-512"),
        }
    }
}

/// Kafka exporter configuration
#[derive(Debug, Clone)]
pub struct KafkaExporterConfig {
    /// Kafka bootstrap servers (comma-separated)
    pub bootstrap_servers: String,

    /// Topic name to publish to
    pub topic: String,

    /// SASL mechanism for authentication
    pub sasl_mechanism: SaslMechanism,

    /// SASL username
    pub sasl_username: Option<String>,

    /// SASL password
    pub sasl_password: Option<String>,

    /// Enable TLS/SSL
    pub tls: bool,

    /// TLS CA certificate path
    pub tls_ca_path: Option<String>,

    /// TLS client certificate path
    pub tls_cert_path: Option<String>,

    /// TLS client key path
    pub tls_key_path: Option<String>,

    /// Message compression codec
    pub compression: KafkaCompression,

    /// Producer acks (-1, 0, 1, or "all")
    pub acks: String,

    /// Linger time in milliseconds (for batching)
    pub linger_ms: u64,

    /// Batch size in bytes
    pub batch_size: usize,

    /// Buffer memory in bytes
    pub buffer_memory: usize,

    /// Delivery timeout in milliseconds
    pub delivery_timeout_ms: u64,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,

    /// Use event_id as message key (default: true)
    /// Alternative: use "{host}:{pid}" as key for partition affinity
    pub key_by_event_id: bool,

    /// Client ID for Kafka
    pub client_id: String,
}

/// Kafka compression codec
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KafkaCompression {
    #[default]
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

impl KafkaCompression {
    fn as_str(&self) -> &'static str {
        match self {
            KafkaCompression::None => "none",
            KafkaCompression::Gzip => "gzip",
            KafkaCompression::Snappy => "snappy",
            KafkaCompression::Lz4 => "lz4",
            KafkaCompression::Zstd => "zstd",
        }
    }
}

impl Default for KafkaExporterConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            topic: "oisp-events".to_string(),
            sasl_mechanism: SaslMechanism::None,
            sasl_username: None,
            sasl_password: None,
            tls: false,
            tls_ca_path: None,
            tls_cert_path: None,
            tls_key_path: None,
            compression: KafkaCompression::Lz4,
            acks: "all".to_string(),
            linger_ms: 5,
            batch_size: 16384,
            buffer_memory: 33554432, // 32MB
            delivery_timeout_ms: 30000,
            request_timeout_ms: 10000,
            key_by_event_id: true,
            client_id: "oisp-sensor".to_string(),
        }
    }
}

/// Kafka exporter for publishing events to Kafka topics
pub struct KafkaExporter {
    config: KafkaExporterConfig,
    producer: Option<FutureProducer>,
    events_exported: std::sync::atomic::AtomicU64,
    errors: std::sync::atomic::AtomicU64,
}

impl KafkaExporter {
    /// Create a new Kafka exporter with the given configuration
    pub fn new(config: KafkaExporterConfig) -> Self {
        Self {
            config,
            producer: None,
            events_exported: std::sync::atomic::AtomicU64::new(0),
            errors: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Initialize the Kafka producer
    fn init_producer(&mut self) -> PluginResult<()> {
        let mut client_config = ClientConfig::new();

        // Basic configuration
        client_config
            .set("bootstrap.servers", &self.config.bootstrap_servers)
            .set("client.id", &self.config.client_id)
            .set("acks", &self.config.acks)
            .set("compression.type", self.config.compression.as_str())
            .set("linger.ms", self.config.linger_ms.to_string())
            .set("batch.size", self.config.batch_size.to_string())
            .set("buffer.memory", self.config.buffer_memory.to_string())
            .set(
                "delivery.timeout.ms",
                self.config.delivery_timeout_ms.to_string(),
            )
            .set(
                "request.timeout.ms",
                self.config.request_timeout_ms.to_string(),
            );

        // TLS configuration
        if self.config.tls {
            client_config.set(
                "security.protocol",
                if self.config.sasl_mechanism != SaslMechanism::None {
                    "SASL_SSL"
                } else {
                    "SSL"
                },
            );

            if let Some(ref ca_path) = self.config.tls_ca_path {
                client_config.set("ssl.ca.location", ca_path);
            }
            if let Some(ref cert_path) = self.config.tls_cert_path {
                client_config.set("ssl.certificate.location", cert_path);
            }
            if let Some(ref key_path) = self.config.tls_key_path {
                client_config.set("ssl.key.location", key_path);
            }
        } else if self.config.sasl_mechanism != SaslMechanism::None {
            client_config.set("security.protocol", "SASL_PLAINTEXT");
        }

        // SASL configuration
        if let Some(mechanism) = self.config.sasl_mechanism.as_str() {
            client_config.set("sasl.mechanism", mechanism);

            if let Some(ref username) = self.config.sasl_username {
                client_config.set("sasl.username", username);
            }
            if let Some(ref password) = self.config.sasl_password {
                client_config.set("sasl.password", password);
            }
        }

        // Create the producer
        let producer: FutureProducer = client_config.create().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to create Kafka producer: {}", e))
        })?;

        self.producer = Some(producer);
        Ok(())
    }

    /// Generate message key for an event
    fn message_key(&self, event: &OispEvent) -> String {
        if self.config.key_by_event_id {
            event.envelope().event_id.clone()
        } else {
            // Use host:pid for partition affinity
            let envelope = event.envelope();
            let host = envelope
                .host
                .as_ref()
                .map(|h| h.hostname.as_str())
                .unwrap_or("unknown");
            let pid = envelope.process.as_ref().map(|p| p.pid).unwrap_or(0);
            format!("{}:{}", host, pid)
        }
    }

    /// Get export statistics
    pub fn stats(&self) -> (u64, u64) {
        (
            self.events_exported
                .load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

impl PluginInfo for KafkaExporter {
    fn name(&self) -> &str {
        "kafka-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events to Apache Kafka topics"
    }
}

impl Plugin for KafkaExporter {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Parse configuration
        if let Some(bootstrap_servers) = config.get::<String>("bootstrap_servers") {
            self.config.bootstrap_servers = bootstrap_servers;
        }
        if let Some(topic) = config.get::<String>("topic") {
            self.config.topic = topic;
        }
        if let Some(sasl_mechanism) = config.get::<String>("sasl_mechanism") {
            self.config.sasl_mechanism = match sasl_mechanism.to_uppercase().as_str() {
                "PLAIN" => SaslMechanism::Plain,
                "SCRAM-SHA-256" => SaslMechanism::ScramSha256,
                "SCRAM-SHA-512" => SaslMechanism::ScramSha512,
                _ => SaslMechanism::None,
            };
        }
        if let Some(sasl_username) = config.get::<String>("sasl_username") {
            self.config.sasl_username = Some(sasl_username);
        }
        if let Some(sasl_password) = config.get::<String>("sasl_password") {
            self.config.sasl_password = Some(sasl_password);
        }
        if let Some(tls) = config.get::<bool>("tls") {
            self.config.tls = tls;
        }
        if let Some(tls_ca_path) = config.get::<String>("tls_ca_path") {
            self.config.tls_ca_path = Some(tls_ca_path);
        }
        if let Some(compression) = config.get::<String>("compression") {
            self.config.compression = match compression.to_lowercase().as_str() {
                "gzip" => KafkaCompression::Gzip,
                "snappy" => KafkaCompression::Snappy,
                "lz4" => KafkaCompression::Lz4,
                "zstd" => KafkaCompression::Zstd,
                _ => KafkaCompression::None,
            };
        }
        if let Some(linger_ms) = config.get::<u64>("linger_ms") {
            self.config.linger_ms = linger_ms;
        }
        if let Some(batch_size) = config.get::<usize>("batch_size") {
            self.config.batch_size = batch_size;
        }
        if let Some(key_by_event_id) = config.get::<bool>("key_by_event_id") {
            self.config.key_by_event_id = key_by_event_id;
        }

        self.init_producer()?;

        info!(
            "Kafka exporter initialized: servers={}, topic={}",
            self.config.bootstrap_servers, self.config.topic
        );

        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Flush any pending messages
        if let Some(ref producer) = self.producer {
            producer.flush(Timeout::After(Duration::from_secs(5)));
        }
        self.producer = None;
        info!("Kafka exporter shutdown complete");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ExportPlugin for KafkaExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        let producer = self.producer.as_ref().ok_or_else(|| {
            PluginError::OperationFailed("Kafka producer not initialized".to_string())
        })?;

        // Serialize event to JSON
        let payload = serde_json::to_string(event)?;
        let key = self.message_key(event);
        let event_type = event.event_type();
        let timestamp = event.envelope().ts.timestamp_millis();

        // Build the record with headers
        let record = FutureRecord::to(&self.config.topic)
            .key(&key)
            .payload(&payload)
            .timestamp(timestamp)
            .headers(
                rdkafka::message::OwnedHeaders::new()
                    .insert(rdkafka::message::Header {
                        key: "event_type",
                        value: Some(event_type.as_bytes()),
                    })
                    .insert(rdkafka::message::Header {
                        key: "oisp_version",
                        value: Some(event.envelope().oisp_version.as_bytes()),
                    }),
            );

        // Send the message
        match producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
        {
            Ok((_partition, _offset)) => {
                self.events_exported
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Exported event {} to Kafka", event.envelope().event_id);
                Ok(())
            }
            Err((err, _)) => {
                self.errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                error!("Failed to send event to Kafka: {}", err);
                Err(PluginError::OperationFailed(format!(
                    "Kafka send failed: {}",
                    err
                )))
            }
        }
    }

    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()> {
        // Kafka producer handles batching internally via linger.ms and batch.size
        // We just send all events and let rdkafka batch them
        for event in events {
            self.export(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> PluginResult<()> {
        if let Some(ref producer) = self.producer {
            producer.flush(Timeout::After(Duration::from_secs(10)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KafkaExporterConfig::default();
        assert_eq!(config.bootstrap_servers, "localhost:9092");
        assert_eq!(config.topic, "oisp-events");
        assert_eq!(config.compression, KafkaCompression::Lz4);
        assert_eq!(config.sasl_mechanism, SaslMechanism::None);
        assert!(config.key_by_event_id);
    }

    #[test]
    fn test_sasl_mechanism() {
        assert_eq!(SaslMechanism::Plain.as_str(), Some("PLAIN"));
        assert_eq!(SaslMechanism::ScramSha256.as_str(), Some("SCRAM-SHA-256"));
        assert_eq!(SaslMechanism::ScramSha512.as_str(), Some("SCRAM-SHA-512"));
        assert_eq!(SaslMechanism::None.as_str(), None);
    }

    #[test]
    fn test_compression_codec() {
        assert_eq!(KafkaCompression::None.as_str(), "none");
        assert_eq!(KafkaCompression::Gzip.as_str(), "gzip");
        assert_eq!(KafkaCompression::Lz4.as_str(), "lz4");
        assert_eq!(KafkaCompression::Zstd.as_str(), "zstd");
    }
}
