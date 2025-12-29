//! Configuration system for OISP Sensor
//!
//! Provides:
//! - Config file discovery (CLI flag, env var, standard paths)
//! - TOML parsing with serde
//! - Environment variable overrides
//! - Sink configuration schema
//! - Hot-reload capability

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Config file not found: {0}")]
    NotFound(PathBuf),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Complete sensor configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorConfig {
    /// Sensor settings
    pub sensor: SensorSettings,

    /// Capture settings
    pub capture: CaptureSettings,

    /// Redaction settings
    pub redaction: RedactionSettings,

    /// Policy engine settings
    pub policy: PolicySettings,

    /// Export settings
    pub export: ExportSettings,

    /// Web UI settings
    pub web: WebSettings,

    /// Correlation settings
    pub correlation: CorrelationSettings,
}

/// Sensor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSettings {
    /// Log level: trace, debug, info, warn, error
    pub log_level: String,
}

impl Default for SensorSettings {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
        }
    }
}

/// Capture settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureSettings {
    /// Enable SSL/TLS capture
    pub ssl: bool,

    /// Enable process capture
    pub process: bool,

    /// Enable file capture
    pub file: bool,

    /// Enable network capture
    pub network: bool,

    /// Additional binary paths for SSL library detection
    pub ssl_binary_paths: Vec<String>,

    /// Process name filter (capture only these processes, empty = all)
    pub process_filter: Vec<String>,

    /// PID filter (capture only these PIDs, empty = all)
    pub pid_filter: Vec<u32>,

    /// Path to eBPF bytecode file (Linux only)
    pub ebpf_path: Option<String>,

    /// Path to libssl.so for SSL interception
    pub libssl_path: Option<String>,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            ssl: true,
            process: true,
            file: true,
            network: true,
            ssl_binary_paths: Vec::new(),
            process_filter: Vec::new(),
            pid_filter: Vec::new(),
            ebpf_path: None,
            libssl_path: None,
        }
    }
}

/// Redaction settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RedactionSettings {
    /// Mode: safe, full, minimal
    pub mode: String,

    /// Redact API keys
    pub redact_api_keys: bool,

    /// Redact email addresses
    pub redact_emails: bool,

    /// Redact credit card numbers
    pub redact_credit_cards: bool,

    /// Redact social security numbers
    pub redact_ssn: bool,

    /// Redact phone numbers
    pub redact_phone_numbers: bool,

    /// Custom regex patterns to redact
    pub custom_patterns: Vec<String>,
}

impl Default for RedactionSettings {
    fn default() -> Self {
        Self {
            mode: "safe".to_string(),
            redact_api_keys: true,
            redact_emails: true,
            redact_credit_cards: true,
            redact_ssn: true,
            redact_phone_numbers: false,
            custom_patterns: Vec::new(),
        }
    }
}

/// Policy engine settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicySettings {
    /// Enable policy engine
    pub enabled: bool,

    /// Path to policy file (YAML)
    pub policy_file: String,

    /// Enable hot-reload of policy file
    pub hot_reload: bool,

    /// Default action when no policy matches: allow, block, log
    pub default_action: String,

    /// Enable audit logging
    pub audit_enabled: bool,

    /// Path to audit log file (JSONL format)
    pub audit_file: Option<String>,

    /// Minimum severity to audit: info, warning, alert, critical
    pub audit_min_severity: String,

    /// Webhook URL for alerts
    pub alert_webhook_url: Option<String>,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            policy_file: default_policy_path_string(),
            hot_reload: true,
            default_action: "allow".to_string(),
            audit_enabled: false,
            audit_file: None,
            audit_min_severity: "info".to_string(),
            alert_webhook_url: None,
        }
    }
}

impl PolicySettings {
    /// Convert to PolicyConfig for use by the policy module
    pub fn to_policy_config(&self) -> crate::policy::PolicyConfig {
        use crate::policy::DefaultAction;

        let default_action = match self.default_action.to_lowercase().as_str() {
            "block" => DefaultAction::Block,
            "log" => DefaultAction::Log,
            _ => DefaultAction::Allow,
        };

        crate::policy::PolicyConfig {
            policy_file: PathBuf::from(&self.policy_file),
            hot_reload: self.hot_reload,
            audit_enabled: self.audit_enabled,
            audit_file: self.audit_file.as_ref().map(PathBuf::from),
            default_action,
            alert_webhook_url: self.alert_webhook_url.clone(),
        }
    }
}

/// Get the default policy file path as a string
fn default_policy_path_string() -> String {
    crate::policy::default_policy_path()
        .to_string_lossy()
        .to_string()
}

/// Export settings container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    /// JSONL file output
    pub jsonl: JsonlExportConfig,

    /// WebSocket for UI
    pub websocket: WebSocketExportConfig,

    /// OTLP export
    pub otlp: OtlpExportConfig,

    /// Kafka export
    pub kafka: KafkaExportConfig,

    /// Webhook export
    pub webhook: WebhookExportConfig,

    /// Oximy Cloud export
    pub oximy: OximyExportConfig,
}

/// JSONL export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JsonlExportConfig {
    /// Enable JSONL export
    pub enabled: bool,

    /// Output file path
    pub path: String,

    /// Append to existing file
    pub append: bool,

    /// Flush after each event
    pub flush_each: bool,

    /// Pretty print JSON
    pub pretty: bool,
}

impl Default for JsonlExportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/var/lib/oisp-sensor/events.jsonl".to_string(),
            append: true,
            flush_each: true,
            pretty: false,
        }
    }
}

/// WebSocket export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketExportConfig {
    /// Enable WebSocket export
    pub enabled: bool,

    /// Host to bind
    pub host: String,

    /// Port to bind
    pub port: u16,

    /// Buffer size for messages
    pub buffer_size: usize,
}

impl Default for WebSocketExportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 7777,
            buffer_size: 1000,
        }
    }
}

/// OTLP export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OtlpExportConfig {
    /// Enable OTLP export
    pub enabled: bool,

    /// OTLP endpoint URL
    pub endpoint: String,

    /// Protocol: grpc, http-proto, http-json
    pub protocol: String,

    /// Use insecure connection
    pub insecure: bool,

    /// Enable compression (gzip)
    pub compression: bool,

    /// Custom headers
    pub headers: HashMap<String, String>,

    /// API key (convenience - added to headers)
    pub api_key: Option<String>,

    /// Bearer token (convenience - added to Authorization header)
    pub bearer_token: Option<String>,

    /// Batch size
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
}

impl Default for OtlpExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            insecure: true,
            compression: true,
            headers: HashMap::new(),
            api_key: None,
            bearer_token: None,
            batch_size: 100,
            flush_interval_ms: 5000,
        }
    }
}

/// Kafka export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KafkaExportConfig {
    /// Enable Kafka export
    pub enabled: bool,

    /// Bootstrap servers (comma-separated)
    pub brokers: String,

    /// Topic name
    pub topic: String,

    /// SASL mechanism: plain, scram-sha-256, scram-sha-512
    pub sasl_mechanism: Option<String>,

    /// SASL username
    pub sasl_username: Option<String>,

    /// SASL password
    pub sasl_password: Option<String>,

    /// Enable TLS
    pub tls: bool,

    /// Compression: none, gzip, snappy, lz4, zstd
    pub compression: String,

    /// Batch size
    pub batch_size: usize,

    /// Linger time in milliseconds
    pub linger_ms: u64,

    /// Message key mode: event_id, host_pid, none
    pub key_mode: String,
}

impl Default for KafkaExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            brokers: "localhost:9092".to_string(),
            topic: "oisp-events".to_string(),
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            tls: false,
            compression: "gzip".to_string(),
            batch_size: 100,
            linger_ms: 100,
            key_mode: "event_id".to_string(),
        }
    }
}

/// Webhook export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookExportConfig {
    /// Enable Webhook export
    pub enabled: bool,

    /// Webhook endpoint URL
    pub url: String,

    /// HTTP method: POST, PUT, PATCH
    pub method: String,

    /// Custom headers
    pub headers: HashMap<String, String>,

    /// Authentication type: none, api_key, bearer, basic
    pub auth_type: String,

    /// API key (for api_key auth)
    pub api_key: Option<String>,

    /// API key header name
    pub api_key_header: String,

    /// Bearer token (for bearer auth)
    pub bearer_token: Option<String>,

    /// Basic auth username
    pub basic_username: Option<String>,

    /// Basic auth password
    pub basic_password: Option<String>,

    /// Batch mode
    pub batch_mode: bool,

    /// Batch size
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Max retries
    pub max_retries: u32,

    /// Initial retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for WebhookExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://localhost:8080/events".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            auth_type: "none".to_string(),
            api_key: None,
            api_key_header: "X-API-Key".to_string(),
            bearer_token: None,
            basic_username: None,
            basic_password: None,
            batch_mode: false,
            batch_size: 100,
            flush_interval_ms: 5000,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Oximy Cloud export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OximyExportConfig {
    /// Enable Oximy export
    pub enabled: bool,

    /// Oximy API key
    pub api_key: Option<String>,

    /// Oximy endpoint (default: https://api.oximy.com)
    pub endpoint: String,

    /// Device ID (set after registration)
    pub device_id: Option<String>,

    /// Batch size
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
}

impl Default for OximyExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            endpoint: "https://api.oximy.com".to_string(),
            device_id: None,
            batch_size: 100,
            flush_interval_ms: 5000,
        }
    }
}

/// Web UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSettings {
    /// Enable Web UI
    pub enabled: bool,

    /// Host to bind
    pub host: String,

    /// Port to bind
    pub port: u16,
}

impl Default for WebSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 7777,
        }
    }
}

/// Correlation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorrelationSettings {
    /// Time window for correlating events (ms)
    pub time_window_ms: u64,

    /// Maximum trace duration before auto-complete (ms)
    pub max_trace_duration_ms: u64,

    /// Maximum traces to keep in memory
    pub max_traces: usize,
}

impl Default for CorrelationSettings {
    fn default() -> Self {
        Self {
            time_window_ms: 5000,
            max_trace_duration_ms: 300000,
            max_traces: 100,
        }
    }
}

/// Configuration loader
pub struct ConfigLoader {
    /// Path to config file (if specified via CLI)
    cli_path: Option<PathBuf>,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self { cli_path: None }
    }

    /// Set the config path from CLI argument
    pub fn with_cli_path(mut self, path: Option<PathBuf>) -> Self {
        self.cli_path = path;
        self
    }

    /// Load configuration with the following precedence:
    /// 1. CLI --config flag
    /// 2. OISP_CONFIG environment variable
    /// 3. ~/.config/oisp-sensor/config.toml
    /// 4. /etc/oisp-sensor/config.toml
    /// 5. Default values
    pub fn load(&self) -> ConfigResult<SensorConfig> {
        // Try to find config file
        let config_path = self.find_config_file();

        let mut config = if let Some(path) = config_path {
            info!("Loading configuration from: {}", path.display());
            self.load_from_file(&path)?
        } else {
            debug!("No config file found, using defaults");
            SensorConfig::default()
        };

        // Apply environment variable overrides
        self.apply_env_overrides(&mut config);

        // Validate configuration
        self.validate(&config)?;

        Ok(config)
    }

    /// Find the config file to use
    pub fn find_config_file(&self) -> Option<PathBuf> {
        // 1. CLI --config flag
        if let Some(path) = &self.cli_path {
            if path.exists() {
                return Some(path.clone());
            }
            warn!("CLI config path does not exist: {}", path.display());
        }

        // 2. OISP_CONFIG environment variable
        if let Ok(env_path) = std::env::var("OISP_CONFIG") {
            let path = PathBuf::from(&env_path);
            if path.exists() {
                return Some(path);
            }
            warn!("OISP_CONFIG path does not exist: {}", env_path);
        }

        // 3. ~/.config/oisp-sensor/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("oisp-sensor").join("config.toml");
            if path.exists() {
                return Some(path);
            }
        }

        // 4. /etc/oisp-sensor/config.toml (Unix only)
        #[cfg(unix)]
        {
            let path = PathBuf::from("/etc/oisp-sensor/config.toml");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Load configuration from a TOML file
    fn load_from_file(&self, path: &Path) -> ConfigResult<SensorConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: SensorConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&self, config: &mut SensorConfig) {
        // Sensor settings
        if let Ok(val) = std::env::var("OISP_LOG_LEVEL") {
            config.sensor.log_level = val;
        }

        // Web settings
        if let Ok(val) = std::env::var("OISP_WEB_PORT") {
            if let Ok(port) = val.parse() {
                config.web.port = port;
            }
        }
        if let Ok(val) = std::env::var("OISP_WEB_HOST") {
            config.web.host = val;
        }
        if let Ok(val) = std::env::var("OISP_WEB_ENABLED") {
            config.web.enabled = val.parse().unwrap_or(config.web.enabled);
        }

        // Capture settings
        if let Ok(val) = std::env::var("OISP_CAPTURE_SSL") {
            config.capture.ssl = val.parse().unwrap_or(config.capture.ssl);
        }
        if let Ok(val) = std::env::var("OISP_CAPTURE_PROCESS") {
            config.capture.process = val.parse().unwrap_or(config.capture.process);
        }
        if let Ok(val) = std::env::var("OISP_CAPTURE_FILE") {
            config.capture.file = val.parse().unwrap_or(config.capture.file);
        }
        if let Ok(val) = std::env::var("OISP_CAPTURE_NETWORK") {
            config.capture.network = val.parse().unwrap_or(config.capture.network);
        }

        // Redaction settings
        if let Ok(val) = std::env::var("OISP_REDACTION_MODE") {
            config.redaction.mode = val;
        }

        // Oximy settings
        if let Ok(val) = std::env::var("OISP_OXIMY_API_KEY") {
            config.export.oximy.api_key = Some(val);
            config.export.oximy.enabled = true;
        }
        if let Ok(val) = std::env::var("OISP_OXIMY_ENDPOINT") {
            config.export.oximy.endpoint = val;
        }

        // OTLP settings
        if let Ok(val) = std::env::var("OISP_OTLP_ENDPOINT") {
            config.export.otlp.endpoint = val;
            config.export.otlp.enabled = true;
        }
        if let Ok(val) = std::env::var("OISP_OTLP_ENABLED") {
            config.export.otlp.enabled = val.parse().unwrap_or(config.export.otlp.enabled);
        }

        // Kafka settings
        if let Ok(val) = std::env::var("OISP_KAFKA_BROKERS") {
            config.export.kafka.brokers = val;
            config.export.kafka.enabled = true;
        }
        if let Ok(val) = std::env::var("OISP_KAFKA_TOPIC") {
            config.export.kafka.topic = val;
        }
        if let Ok(val) = std::env::var("OISP_KAFKA_ENABLED") {
            config.export.kafka.enabled = val.parse().unwrap_or(config.export.kafka.enabled);
        }

        // Webhook settings
        if let Ok(val) = std::env::var("OISP_WEBHOOK_URL") {
            config.export.webhook.url = val;
            config.export.webhook.enabled = true;
        }
        if let Ok(val) = std::env::var("OISP_WEBHOOK_ENABLED") {
            config.export.webhook.enabled = val.parse().unwrap_or(config.export.webhook.enabled);
        }

        // JSONL settings
        if let Ok(val) = std::env::var("OISP_JSONL_PATH") {
            config.export.jsonl.path = val;
        }
        if let Ok(val) = std::env::var("OISP_JSONL_ENABLED") {
            config.export.jsonl.enabled = val.parse().unwrap_or(config.export.jsonl.enabled);
        }

        // Policy settings
        if let Ok(val) = std::env::var("OISP_POLICY_ENABLED") {
            config.policy.enabled = val.parse().unwrap_or(config.policy.enabled);
        }
        if let Ok(val) = std::env::var("OISP_POLICY_FILE") {
            config.policy.policy_file = val;
        }
        if let Ok(val) = std::env::var("OISP_POLICY_HOT_RELOAD") {
            config.policy.hot_reload = val.parse().unwrap_or(config.policy.hot_reload);
        }
        if let Ok(val) = std::env::var("OISP_POLICY_DEFAULT_ACTION") {
            config.policy.default_action = val;
        }
        if let Ok(val) = std::env::var("OISP_POLICY_AUDIT_ENABLED") {
            config.policy.audit_enabled = val.parse().unwrap_or(config.policy.audit_enabled);
        }
        if let Ok(val) = std::env::var("OISP_POLICY_AUDIT_FILE") {
            config.policy.audit_file = Some(val);
        }
        if let Ok(val) = std::env::var("OISP_POLICY_AUDIT_MIN_SEVERITY") {
            config.policy.audit_min_severity = val;
        }
        if let Ok(val) = std::env::var("OISP_POLICY_ALERT_WEBHOOK_URL") {
            config.policy.alert_webhook_url = Some(val);
        }
    }

    /// Validate configuration
    fn validate(&self, config: &SensorConfig) -> ConfigResult<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.sensor.log_level.to_lowercase().as_str()) {
            return Err(ConfigError::ValidationError(format!(
                "Invalid log level: {}. Must be one of: {:?}",
                config.sensor.log_level, valid_levels
            )));
        }

        // Validate redaction mode
        let valid_modes = ["safe", "full", "minimal"];
        if !valid_modes.contains(&config.redaction.mode.to_lowercase().as_str()) {
            return Err(ConfigError::ValidationError(format!(
                "Invalid redaction mode: {}. Must be one of: {:?}",
                config.redaction.mode, valid_modes
            )));
        }

        // Validate OTLP protocol
        if config.export.otlp.enabled {
            let valid_protocols = ["grpc", "http-proto", "http-json"];
            if !valid_protocols.contains(&config.export.otlp.protocol.to_lowercase().as_str()) {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid OTLP protocol: {}. Must be one of: {:?}",
                    config.export.otlp.protocol, valid_protocols
                )));
            }
        }

        // Validate webhook method
        if config.export.webhook.enabled {
            let valid_methods = ["POST", "PUT", "PATCH"];
            if !valid_methods.contains(&config.export.webhook.method.to_uppercase().as_str()) {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid webhook method: {}. Must be one of: {:?}",
                    config.export.webhook.method, valid_methods
                )));
            }
        }

        // Validate ports
        if config.web.port == 0 {
            return Err(ConfigError::ValidationError(
                "Web port cannot be 0".to_string(),
            ));
        }

        // Validate policy settings
        if config.policy.enabled {
            let valid_actions = ["allow", "block", "log"];
            if !valid_actions.contains(&config.policy.default_action.to_lowercase().as_str()) {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid policy default_action: {}. Must be one of: {:?}",
                    config.policy.default_action, valid_actions
                )));
            }

            let valid_severities = ["info", "warning", "alert", "critical"];
            if !valid_severities.contains(&config.policy.audit_min_severity.to_lowercase().as_str())
            {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid policy audit_min_severity: {}. Must be one of: {:?}",
                    config.policy.audit_min_severity, valid_severities
                )));
            }
        }

        Ok(())
    }

    /// Save configuration to a file
    pub fn save(&self, config: &SensorConfig, path: &Path) -> ConfigResult<()> {
        let content = toml::to_string_pretty(config).map_err(|e| {
            ConfigError::ValidationError(format!("Failed to serialize config: {}", e))
        })?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        info!("Configuration saved to: {}", path.display());
        Ok(())
    }

    /// Get the default config file path for the current platform
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("oisp-sensor").join("config.toml"))
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared configuration that can be updated at runtime
///
/// This type wraps a `SensorConfig` in an `Arc<parking_lot::RwLock<>>` to allow
/// for safe concurrent access and runtime updates (hot-reload).
#[derive(Clone)]
pub struct SharedConfig {
    inner: std::sync::Arc<parking_lot::RwLock<SensorConfig>>,
    /// Path to the config file being watched (if any)
    config_path: std::sync::Arc<parking_lot::RwLock<Option<PathBuf>>>,
}

impl SharedConfig {
    /// Create a new shared config from a SensorConfig
    pub fn new(config: SensorConfig) -> Self {
        Self {
            inner: std::sync::Arc::new(parking_lot::RwLock::new(config)),
            config_path: std::sync::Arc::new(parking_lot::RwLock::new(None)),
        }
    }

    /// Create a shared config from the loader
    pub fn from_loader(loader: &ConfigLoader) -> ConfigResult<Self> {
        let config = loader.load()?;
        let config_path = loader.find_config_file();

        let shared = Self::new(config);
        *shared.config_path.write() = config_path;

        Ok(shared)
    }

    /// Get a read lock on the configuration
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, SensorConfig> {
        self.inner.read()
    }

    /// Get a clone of the current configuration
    pub fn get(&self) -> SensorConfig {
        self.inner.read().clone()
    }

    /// Update the configuration
    pub fn update(&self, config: SensorConfig) {
        *self.inner.write() = config;
        info!("Configuration updated");
    }

    /// Reload configuration from disk
    ///
    /// This re-reads the config file and applies environment variable overrides.
    /// Returns Ok(true) if the config was reloaded, Ok(false) if no config file exists.
    pub fn reload(&self) -> ConfigResult<bool> {
        let config_path = self.config_path.read().clone();

        if let Some(path) = config_path {
            if path.exists() {
                let loader = ConfigLoader::new().with_cli_path(Some(path.clone()));
                let new_config = loader.load()?;
                self.update(new_config);
                info!("Configuration reloaded from: {}", path.display());
                return Ok(true);
            }
        }

        debug!("No config file to reload");
        Ok(false)
    }

    /// Set the config file path to watch
    pub fn set_config_path(&self, path: Option<PathBuf>) {
        *self.config_path.write() = path;
    }

    /// Get the current config file path
    pub fn config_path(&self) -> Option<PathBuf> {
        self.config_path.read().clone()
    }
}

impl Default for SharedConfig {
    fn default() -> Self {
        Self::new(SensorConfig::default())
    }
}

/// Setup SIGHUP handler for config reload (Unix only)
///
/// When SIGHUP is received, the provided callback will be invoked.
/// This is typically used to trigger a config reload.
#[cfg(unix)]
pub async fn setup_sighup_handler<F>(mut callback: F)
where
    F: FnMut() + Send + 'static,
{
    use tokio::signal::unix::{signal, SignalKind};

    let mut sighup = match signal(SignalKind::hangup()) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to setup SIGHUP handler: {}", e);
            return;
        }
    };

    tokio::spawn(async move {
        loop {
            sighup.recv().await;
            info!("Received SIGHUP, triggering config reload");
            callback();
        }
    });
}

/// Setup SIGHUP handler that reloads a SharedConfig (Unix only)
#[cfg(unix)]
pub fn spawn_sighup_reload_handler(config: SharedConfig) {
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sighup = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to setup SIGHUP handler: {}", e);
                return;
            }
        };

        loop {
            sighup.recv().await;
            info!("Received SIGHUP, reloading configuration");
            match config.reload() {
                Ok(true) => info!("Configuration reloaded successfully"),
                Ok(false) => info!("No configuration file to reload"),
                Err(e) => warn!("Failed to reload configuration: {}", e),
            }
        }
    });
}

/// No-op SIGHUP handler for non-Unix platforms
#[cfg(not(unix))]
pub async fn setup_sighup_handler<F>(_callback: F)
where
    F: FnMut() + Send + 'static,
{
    debug!("SIGHUP handler not available on this platform");
}

/// No-op SIGHUP reload handler for non-Unix platforms
#[cfg(not(unix))]
pub fn spawn_sighup_reload_handler(_config: SharedConfig) {
    debug!("SIGHUP handler not available on this platform");
}

/// Helper module for platform-specific directories
mod dirs {
    use std::path::PathBuf;

    /// Get the user's config directory
    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".config"))
                })
        }

        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SensorConfig::default();
        assert_eq!(config.sensor.log_level, "info");
        assert!(config.capture.ssl);
        assert_eq!(config.redaction.mode, "safe");
        assert!(config.web.enabled);
        assert_eq!(config.web.port, 7777);
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
            [sensor]
            log_level = "debug"
        "#;
        let config: SensorConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sensor.log_level, "debug");
        // Other fields should be default
        assert!(config.capture.ssl);
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
            [sensor]
            log_level = "trace"

            [capture]
            ssl = false
            process = true
            process_filter = ["node", "python"]

            [redaction]
            mode = "full"
            redact_api_keys = false

            [export.jsonl]
            enabled = true
            path = "/tmp/events.jsonl"

            [export.otlp]
            enabled = true
            endpoint = "http://otel:4317"
            protocol = "grpc"

            [web]
            enabled = true
            port = 8080
        "#;

        let config: SensorConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sensor.log_level, "trace");
        assert!(!config.capture.ssl);
        assert_eq!(config.capture.process_filter, vec!["node", "python"]);
        assert_eq!(config.redaction.mode, "full");
        assert!(!config.redaction.redact_api_keys);
        assert!(config.export.jsonl.enabled);
        assert_eq!(config.export.jsonl.path, "/tmp/events.jsonl");
        assert!(config.export.otlp.enabled);
        assert_eq!(config.export.otlp.endpoint, "http://otel:4317");
        assert_eq!(config.web.port, 8080);
    }

    #[test]
    fn test_validation_invalid_log_level() {
        let config = SensorConfig {
            sensor: SensorSettings {
                log_level: "invalid".to_string(),
            },
            ..Default::default()
        };
        let loader = ConfigLoader::new();
        let result = loader.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_redaction_mode() {
        let config = SensorConfig {
            redaction: RedactionSettings {
                mode: "invalid".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let loader = ConfigLoader::new();
        let result = loader.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_config() {
        let config = SensorConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[sensor]"));
        assert!(toml_str.contains("log_level"));
    }
}
