//! OpenTelemetry Protocol (OTLP) exporter
//!
//! Exports OISP events as OpenTelemetry logs using OTLP.
//! Supports both gRPC and HTTP transports.

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ExportPlugin, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

use opentelemetry::logs::{
    AnyValue, LogRecord as OtelLogRecord, Logger, LoggerProvider as _, Severity,
};
use opentelemetry::{Key, KeyValue};
use opentelemetry_otlp::{
    LogExporter, Protocol, WithExportConfig, WithHttpConfig, WithTonicConfig,
};
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::Resource;
use tonic::metadata::MetadataMap;

/// OTLP transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OtlpTransport {
    /// gRPC transport (default, recommended)
    #[default]
    Grpc,
    /// HTTP/protobuf transport
    HttpProto,
    /// HTTP/JSON transport
    HttpJson,
}

/// OTLP exporter configuration
#[derive(Debug, Clone)]
pub struct OtlpExporterConfig {
    /// OTLP endpoint URL
    /// For gRPC: http://localhost:4317
    /// For HTTP: http://localhost:4318/v1/logs
    pub endpoint: String,

    /// Transport protocol
    pub transport: OtlpTransport,

    /// Request timeout
    pub timeout: Duration,

    /// Whether to use TLS
    pub tls: bool,

    /// TLS certificate path (optional)
    pub tls_cert_path: Option<String>,

    /// Authentication headers
    pub headers: HashMap<String, String>,

    /// API key for authentication (added as Authorization header)
    pub api_key: Option<String>,

    /// Bearer token for authentication
    pub bearer_token: Option<String>,

    /// Service name for resource attributes
    pub service_name: String,

    /// Service version
    pub service_version: Option<String>,

    /// Additional resource attributes
    pub resource_attributes: HashMap<String, String>,

    /// Enable gzip compression
    pub compression: bool,

    /// Batch size for export
    pub batch_size: usize,

    /// Flush interval
    pub flush_interval: Duration,
}

impl Default for OtlpExporterConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            transport: OtlpTransport::Grpc,
            timeout: Duration::from_secs(10),
            tls: false,
            tls_cert_path: None,
            headers: HashMap::new(),
            api_key: None,
            bearer_token: None,
            service_name: "oisp-sensor".to_string(),
            service_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            resource_attributes: HashMap::new(),
            compression: true,
            batch_size: 512,
            flush_interval: Duration::from_secs(5),
        }
    }
}

/// OpenTelemetry semantic conventions for AI
/// Based on OpenTelemetry GenAI semantic conventions
mod semconv {
    // GenAI attributes (https://opentelemetry.io/docs/specs/semconv/gen-ai/)
    pub const GEN_AI_SYSTEM: &str = "gen_ai.system";
    pub const GEN_AI_REQUEST_MODEL: &str = "gen_ai.request.model";
    pub const GEN_AI_RESPONSE_MODEL: &str = "gen_ai.response.model";
    pub const GEN_AI_REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";
    pub const GEN_AI_REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";
    pub const GEN_AI_REQUEST_TOP_P: &str = "gen_ai.request.top_p";
    pub const GEN_AI_RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";
    pub const GEN_AI_USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
    pub const GEN_AI_USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
    pub const GEN_AI_OPERATION_NAME: &str = "gen_ai.operation.name";

    // Process attributes
    pub const PROCESS_PID: &str = "process.pid";
    pub const PROCESS_PARENT_PID: &str = "process.parent_pid";
    pub const PROCESS_EXECUTABLE_NAME: &str = "process.executable.name";
    pub const PROCESS_EXECUTABLE_PATH: &str = "process.executable.path";
    pub const PROCESS_COMMAND_LINE: &str = "process.command_line";
    pub const PROCESS_COMMAND_ARGS: &str = "process.command_args";

    // Host attributes
    pub const HOST_NAME: &str = "host.name";
    pub const HOST_ID: &str = "host.id";
    pub const HOST_ARCH: &str = "host.arch";
    pub const OS_TYPE: &str = "os.type";
    pub const OS_VERSION: &str = "os.version";

    // Service attributes
    pub const SERVICE_NAME: &str = "service.name";
    pub const SERVICE_VERSION: &str = "service.version";

    // OISP-specific attributes
    pub const OISP_EVENT_ID: &str = "oisp.event_id";
    pub const OISP_EVENT_TYPE: &str = "oisp.event_type";
    pub const OISP_VERSION: &str = "oisp.version";
    pub const OISP_REQUEST_ID: &str = "oisp.request_id";
    pub const OISP_LATENCY_MS: &str = "oisp.latency_ms";
    pub const OISP_SUCCESS: &str = "oisp.success";
    pub const OISP_STATUS_CODE: &str = "oisp.status_code";
}

/// OTLP exporter for sending events to OpenTelemetry collectors
pub struct OtlpExporter {
    config: OtlpExporterConfig,
    logger_provider: Option<LoggerProvider>,
    events_exported: std::sync::atomic::AtomicU64,
    errors: std::sync::atomic::AtomicU64,
}

impl OtlpExporter {
    /// Create a new OTLP exporter with the given configuration
    pub fn new(config: OtlpExporterConfig) -> Self {
        Self {
            config,
            logger_provider: None,
            events_exported: std::sync::atomic::AtomicU64::new(0),
            errors: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Initialize the OpenTelemetry logger provider
    fn init_logger_provider(&mut self) -> PluginResult<()> {
        // Build resource attributes
        let mut resource_attrs = vec![KeyValue::new(
            semconv::SERVICE_NAME,
            self.config.service_name.clone(),
        )];

        if let Some(ref version) = self.config.service_version {
            resource_attrs.push(KeyValue::new(semconv::SERVICE_VERSION, version.clone()));
        }

        for (key, value) in &self.config.resource_attributes {
            resource_attrs.push(KeyValue::new(key.clone(), value.clone()));
        }

        let resource = Resource::new(resource_attrs);

        // Build the exporter based on transport
        let exporter = self.build_exporter()?;

        // Create batch processor
        let processor = opentelemetry_sdk::logs::BatchLogProcessor::builder(
            exporter,
            opentelemetry_sdk::runtime::Tokio,
        )
        .with_batch_config(
            opentelemetry_sdk::logs::BatchConfigBuilder::default()
                .with_max_export_batch_size(self.config.batch_size)
                .with_scheduled_delay(self.config.flush_interval)
                .build(),
        )
        .build();

        // Create logger provider
        let provider = LoggerProvider::builder()
            .with_resource(resource)
            .with_log_processor(processor)
            .build();

        self.logger_provider = Some(provider);
        Ok(())
    }

    /// Build the OTLP exporter based on configuration
    fn build_exporter(&self) -> PluginResult<LogExporter> {
        let mut headers = self.config.headers.clone();

        // Add authentication headers
        if let Some(ref api_key) = self.config.api_key {
            headers.insert("x-api-key".to_string(), api_key.clone());
        }
        if let Some(ref token) = self.config.bearer_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }

        match self.config.transport {
            OtlpTransport::Grpc => {
                let mut builder = opentelemetry_otlp::LogExporter::builder()
                    .with_tonic()
                    .with_endpoint(&self.config.endpoint)
                    .with_timeout(self.config.timeout);

                // Add headers as metadata
                if !headers.is_empty() {
                    let mut metadata = MetadataMap::new();
                    for (key, value) in headers {
                        if let (Ok(key), Ok(value)) = (
                            key.parse::<tonic::metadata::MetadataKey<tonic::metadata::Ascii>>(),
                            value.parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>(),
                        ) {
                            metadata.insert(key, value);
                        }
                    }
                    builder = builder.with_metadata(metadata);
                }

                if self.config.compression {
                    builder = builder.with_compression(opentelemetry_otlp::Compression::Gzip);
                }

                builder.build().map_err(|e| {
                    PluginError::InitializationFailed(format!(
                        "Failed to create gRPC exporter: {}",
                        e
                    ))
                })
            }
            OtlpTransport::HttpProto => {
                let mut builder = opentelemetry_otlp::LogExporter::builder()
                    .with_http()
                    .with_endpoint(&self.config.endpoint)
                    .with_timeout(self.config.timeout)
                    .with_protocol(Protocol::HttpBinary);

                if !headers.is_empty() {
                    builder = builder.with_headers(headers);
                }

                builder.build().map_err(|e| {
                    PluginError::InitializationFailed(format!(
                        "Failed to create HTTP/proto exporter: {}",
                        e
                    ))
                })
            }
            OtlpTransport::HttpJson => {
                let mut builder = opentelemetry_otlp::LogExporter::builder()
                    .with_http()
                    .with_endpoint(&self.config.endpoint)
                    .with_timeout(self.config.timeout)
                    .with_protocol(Protocol::HttpJson);

                if !headers.is_empty() {
                    builder = builder.with_headers(headers);
                }

                builder.build().map_err(|e| {
                    PluginError::InitializationFailed(format!(
                        "Failed to create HTTP/JSON exporter: {}",
                        e
                    ))
                })
            }
        }
    }

    /// Map an OISP event to OpenTelemetry log record attributes
    fn event_to_attributes(&self, event: &OispEvent) -> Vec<(Key, AnyValue)> {
        let mut attrs: Vec<(Key, AnyValue)> = Vec::new();
        let envelope = event.envelope();

        // OISP envelope attributes
        attrs.push((
            Key::new(semconv::OISP_EVENT_ID),
            AnyValue::String(envelope.event_id.clone().into()),
        ));
        attrs.push((
            Key::new(semconv::OISP_EVENT_TYPE),
            AnyValue::String(event.event_type().into()),
        ));
        attrs.push((
            Key::new(semconv::OISP_VERSION),
            AnyValue::String(envelope.oisp_version.clone().into()),
        ));

        // Process attributes
        if let Some(ref process) = envelope.process {
            attrs.push((
                Key::new(semconv::PROCESS_PID),
                AnyValue::Int(process.pid as i64),
            ));
            if let Some(ppid) = process.ppid {
                attrs.push((
                    Key::new(semconv::PROCESS_PARENT_PID),
                    AnyValue::Int(ppid as i64),
                ));
            }
            if let Some(ref name) = process.name {
                attrs.push((
                    Key::new(semconv::PROCESS_EXECUTABLE_NAME),
                    AnyValue::String(name.clone().into()),
                ));
            }
            if let Some(ref exe) = process.exe {
                attrs.push((
                    Key::new(semconv::PROCESS_EXECUTABLE_PATH),
                    AnyValue::String(exe.clone().into()),
                ));
            }
            if let Some(ref cmdline) = process.cmdline {
                attrs.push((
                    Key::new(semconv::PROCESS_COMMAND_LINE),
                    AnyValue::String(cmdline.clone().into()),
                ));
            }
        }

        // Host attributes
        if let Some(ref host) = envelope.host {
            attrs.push((
                Key::new(semconv::HOST_NAME),
                AnyValue::String(host.hostname.clone().into()),
            ));
            if let Some(ref device_id) = host.device_id {
                attrs.push((
                    Key::new(semconv::HOST_ID),
                    AnyValue::String(device_id.clone().into()),
                ));
            }
            if let Some(ref arch) = host.arch {
                attrs.push((
                    Key::new(semconv::HOST_ARCH),
                    AnyValue::String(arch.clone().into()),
                ));
            }
            if let Some(ref os) = host.os {
                attrs.push((
                    Key::new(semconv::OS_TYPE),
                    AnyValue::String(os.clone().into()),
                ));
            }
            if let Some(ref os_version) = host.os_version {
                attrs.push((
                    Key::new(semconv::OS_VERSION),
                    AnyValue::String(os_version.clone().into()),
                ));
            }
        }

        // Event-specific attributes
        match event {
            OispEvent::AiRequest(e) => {
                attrs.push((
                    Key::new(semconv::GEN_AI_OPERATION_NAME),
                    AnyValue::String("chat".into()),
                ));
                attrs.push((
                    Key::new(semconv::OISP_REQUEST_ID),
                    AnyValue::String(e.data.request_id.clone().into()),
                ));

                if let Some(ref provider) = e.data.provider {
                    attrs.push((
                        Key::new(semconv::GEN_AI_SYSTEM),
                        AnyValue::String(provider.name.clone().into()),
                    ));
                }
                if let Some(ref model) = e.data.model {
                    attrs.push((
                        Key::new(semconv::GEN_AI_REQUEST_MODEL),
                        AnyValue::String(model.id.clone().into()),
                    ));
                }
                if let Some(ref params) = e.data.parameters {
                    if let Some(temp) = params.temperature {
                        attrs.push((
                            Key::new(semconv::GEN_AI_REQUEST_TEMPERATURE),
                            AnyValue::Double(temp),
                        ));
                    }
                    if let Some(top_p) = params.top_p {
                        attrs.push((
                            Key::new(semconv::GEN_AI_REQUEST_TOP_P),
                            AnyValue::Double(top_p),
                        ));
                    }
                    if let Some(max_tokens) = params.max_tokens {
                        attrs.push((
                            Key::new(semconv::GEN_AI_REQUEST_MAX_TOKENS),
                            AnyValue::Int(max_tokens as i64),
                        ));
                    }
                }
                if let Some(estimated) = e.data.estimated_tokens {
                    attrs.push((
                        Key::new(semconv::GEN_AI_USAGE_INPUT_TOKENS),
                        AnyValue::Int(estimated as i64),
                    ));
                }
            }
            OispEvent::AiResponse(e) => {
                attrs.push((
                    Key::new(semconv::GEN_AI_OPERATION_NAME),
                    AnyValue::String("chat".into()),
                ));
                attrs.push((
                    Key::new(semconv::OISP_REQUEST_ID),
                    AnyValue::String(e.data.request_id.clone().into()),
                ));

                if let Some(ref provider) = e.data.provider {
                    attrs.push((
                        Key::new(semconv::GEN_AI_SYSTEM),
                        AnyValue::String(provider.name.clone().into()),
                    ));
                }
                if let Some(ref model) = e.data.model {
                    attrs.push((
                        Key::new(semconv::GEN_AI_RESPONSE_MODEL),
                        AnyValue::String(model.id.clone().into()),
                    ));
                }
                if let Some(ref usage) = e.data.usage {
                    if let Some(input) = usage.prompt_tokens {
                        attrs.push((
                            Key::new(semconv::GEN_AI_USAGE_INPUT_TOKENS),
                            AnyValue::Int(input as i64),
                        ));
                    }
                    if let Some(output) = usage.completion_tokens {
                        attrs.push((
                            Key::new(semconv::GEN_AI_USAGE_OUTPUT_TOKENS),
                            AnyValue::Int(output as i64),
                        ));
                    }
                }
                if let Some(latency) = e.data.latency_ms {
                    attrs.push((
                        Key::new(semconv::OISP_LATENCY_MS),
                        AnyValue::Int(latency as i64),
                    ));
                }
                if let Some(success) = e.data.success {
                    attrs.push((Key::new(semconv::OISP_SUCCESS), AnyValue::Boolean(success)));
                }
                if let Some(status) = e.data.status_code {
                    attrs.push((
                        Key::new(semconv::OISP_STATUS_CODE),
                        AnyValue::Int(status as i64),
                    ));
                }
                if let Some(ref finish_reason) = e.data.finish_reason {
                    attrs.push((
                        Key::new(semconv::GEN_AI_RESPONSE_FINISH_REASONS),
                        AnyValue::String(format!("{:?}", finish_reason).into()),
                    ));
                }
            }
            OispEvent::AiEmbedding(e) => {
                attrs.push((
                    Key::new(semconv::GEN_AI_OPERATION_NAME),
                    AnyValue::String("embedding".into()),
                ));

                if let Some(ref provider) = e.data.provider {
                    attrs.push((
                        Key::new(semconv::GEN_AI_SYSTEM),
                        AnyValue::String(provider.name.clone().into()),
                    ));
                }
                if let Some(ref model) = e.data.model {
                    attrs.push((
                        Key::new(semconv::GEN_AI_REQUEST_MODEL),
                        AnyValue::String(model.id.clone().into()),
                    ));
                }
            }
            OispEvent::ProcessExec(e) => {
                attrs.push((
                    Key::new(semconv::PROCESS_EXECUTABLE_PATH),
                    AnyValue::String(e.data.exe.clone().into()),
                ));
                if !e.data.args.is_empty() {
                    attrs.push((
                        Key::new(semconv::PROCESS_COMMAND_ARGS),
                        AnyValue::String(
                            serde_json::to_string(&e.data.args)
                                .unwrap_or_default()
                                .into(),
                        ),
                    ));
                }
                if let Some(ref cwd) = e.data.cwd {
                    attrs.push((
                        Key::new("process.cwd"),
                        AnyValue::String(cwd.clone().into()),
                    ));
                }
            }
            OispEvent::ProcessExit(e) => {
                attrs.push((
                    Key::new("process.exit_code"),
                    AnyValue::Int(e.data.exit_code as i64),
                ));
                if let Some(signal) = e.data.signal {
                    attrs.push((
                        Key::new("process.exit_signal"),
                        AnyValue::Int(signal as i64),
                    ));
                }
            }
            OispEvent::NetworkConnect(e) => {
                if let Some(ref ip) = e.data.dest.ip {
                    attrs.push((
                        Key::new("network.peer.address"),
                        AnyValue::String(ip.clone().into()),
                    ));
                }
                if let Some(port) = e.data.dest.port {
                    attrs.push((Key::new("network.peer.port"), AnyValue::Int(port as i64)));
                }
                if let Some(ref src) = e.data.src {
                    if let Some(ref local_ip) = src.ip {
                        attrs.push((
                            Key::new("network.local.address"),
                            AnyValue::String(local_ip.clone().into()),
                        ));
                    }
                    if let Some(local_port) = src.port {
                        attrs.push((
                            Key::new("network.local.port"),
                            AnyValue::Int(local_port as i64),
                        ));
                    }
                }
            }
            OispEvent::FileWrite(e) => {
                attrs.push((
                    Key::new("file.path"),
                    AnyValue::String(e.data.path.clone().into()),
                ));
                if let Some(bytes) = e.data.bytes_written {
                    attrs.push((Key::new("file.size"), AnyValue::Int(bytes as i64)));
                }
            }
            // Add more event types as needed
            _ => {}
        }

        attrs
    }

    /// Get severity level based on event type
    fn event_severity(&self, event: &OispEvent) -> Severity {
        match event {
            OispEvent::AiResponse(e) if e.data.success == Some(false) => Severity::Error,
            OispEvent::AiResponse(e) if e.data.error.is_some() => Severity::Error,
            OispEvent::ProcessExit(e) if e.data.exit_code != 0 => Severity::Warn,
            OispEvent::AiRequest(_) | OispEvent::AiResponse(_) => Severity::Info,
            OispEvent::ProcessExec(_) | OispEvent::ProcessExit(_) => Severity::Info,
            OispEvent::NetworkConnect(_) => Severity::Debug,
            OispEvent::FileWrite(_) | OispEvent::FileRead(_) => Severity::Debug,
            _ => Severity::Info,
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

impl PluginInfo for OtlpExporter {
    fn name(&self) -> &str {
        "otlp-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events to OpenTelemetry collectors via OTLP"
    }
}

impl Plugin for OtlpExporter {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Parse configuration from PluginConfig
        if let Some(endpoint) = config.get::<String>("endpoint") {
            self.config.endpoint = endpoint;
        }
        if let Some(transport) = config.get::<String>("transport") {
            self.config.transport = match transport.to_lowercase().as_str() {
                "grpc" => OtlpTransport::Grpc,
                "http-proto" | "http_proto" => OtlpTransport::HttpProto,
                "http-json" | "http_json" => OtlpTransport::HttpJson,
                _ => OtlpTransport::Grpc,
            };
        }
        if let Some(timeout_secs) = config.get::<u64>("timeout_secs") {
            self.config.timeout = Duration::from_secs(timeout_secs);
        }
        if let Some(api_key) = config.get::<String>("api_key") {
            self.config.api_key = Some(api_key);
        }
        if let Some(bearer_token) = config.get::<String>("bearer_token") {
            self.config.bearer_token = Some(bearer_token);
        }
        if let Some(service_name) = config.get::<String>("service_name") {
            self.config.service_name = service_name;
        }
        if let Some(compression) = config.get::<bool>("compression") {
            self.config.compression = compression;
        }
        if let Some(batch_size) = config.get::<usize>("batch_size") {
            self.config.batch_size = batch_size;
        }
        if let Some(headers) = config.get::<HashMap<String, String>>("headers") {
            self.config.headers = headers;
        }

        self.init_logger_provider()?;

        info!(
            "OTLP exporter initialized: endpoint={}, transport={:?}",
            self.config.endpoint, self.config.transport
        );

        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        if let Some(ref provider) = self.logger_provider {
            if let Err(e) = provider.shutdown() {
                warn!("Error shutting down OTLP logger provider: {:?}", e);
            }
        }
        self.logger_provider = None;
        info!("OTLP exporter shutdown complete");
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
impl ExportPlugin for OtlpExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        let provider = self.logger_provider.as_ref().ok_or_else(|| {
            PluginError::OperationFailed("OTLP logger provider not initialized".to_string())
        })?;

        let logger = provider.logger("oisp-sensor");
        let envelope = event.envelope();

        // Create log record
        let mut record = logger.create_log_record();

        // Set timestamp
        let timestamp = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_nanos(envelope.ts.timestamp_nanos_opt().unwrap_or(0) as u64);
        record.set_timestamp(timestamp);
        record.set_observed_timestamp(std::time::SystemTime::now());

        // Set severity
        record.set_severity_number(self.event_severity(event));
        record.set_severity_text(event.event_type());

        // Set body as JSON of the event
        let body = serde_json::to_string(event).unwrap_or_default();
        record.set_body(opentelemetry::logs::AnyValue::String(body.into()));

        // Set attributes
        for (key, value) in self.event_to_attributes(event) {
            record.add_attribute(key, value);
        }

        // Emit the log record
        logger.emit(record);

        self.events_exported
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!("Exported event {} to OTLP", envelope.event_id);

        Ok(())
    }

    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()> {
        for event in events {
            self.export(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> PluginResult<()> {
        if let Some(ref provider) = self.logger_provider {
            let results = provider.force_flush();
            for result in results {
                if let Err(e) = result {
                    warn!("Error flushing OTLP exporter: {:?}", e);
                    self.errors
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OtlpExporterConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.transport, OtlpTransport::Grpc);
        assert_eq!(config.service_name, "oisp-sensor");
        assert!(config.compression);
    }

    #[test]
    fn test_transport_variants() {
        assert_eq!(OtlpTransport::default(), OtlpTransport::Grpc);
    }
}
