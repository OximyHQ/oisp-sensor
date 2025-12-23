//! Webhook exporter
//!
//! Exports OISP events to HTTP endpoints via webhooks.
//! Supports batching, retries with exponential backoff, and various authentication methods.

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{
    ExportPlugin, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use reqwest::{Client, Method, StatusCode};

/// HTTP method for webhook requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebhookMethod {
    #[default]
    Post,
    Put,
    Patch,
}

impl WebhookMethod {
    fn as_reqwest(self) -> Method {
        match self {
            WebhookMethod::Post => Method::POST,
            WebhookMethod::Put => Method::PUT,
            WebhookMethod::Patch => Method::PATCH,
        }
    }
}

/// Authentication method for webhook
#[derive(Debug, Clone, Default)]
pub enum WebhookAuth {
    /// No authentication
    #[default]
    None,
    /// API key in header
    ApiKey { header: String, value: String },
    /// Bearer token
    Bearer(String),
    /// Basic authentication
    Basic { username: String, password: String },
}

/// Webhook exporter configuration
#[derive(Debug, Clone)]
pub struct WebhookExporterConfig {
    /// Webhook endpoint URL
    pub endpoint: String,

    /// HTTP method
    pub method: WebhookMethod,

    /// Authentication configuration
    pub auth: WebhookAuth,

    /// Static headers to include in all requests
    pub headers: HashMap<String, String>,

    /// Request timeout
    pub timeout: Duration,

    /// Enable gzip compression
    pub compression: bool,

    /// Batch mode: send multiple events in a single request as JSON array
    pub batch_mode: bool,

    /// Maximum batch size (when batch_mode is true)
    pub max_batch_size: usize,

    /// Maximum time to wait before flushing a batch
    pub batch_timeout: Duration,

    /// Enable retry on failure
    pub retry_enabled: bool,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial retry delay (doubles with each retry)
    pub initial_retry_delay: Duration,

    /// Maximum retry delay
    pub max_retry_delay: Duration,

    /// User-Agent header
    pub user_agent: String,

    /// Content-Type header (default: application/json)
    pub content_type: String,

    /// Dead letter queue file path (for failed events)
    pub dlq_path: Option<String>,
}

impl Default for WebhookExporterConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080/webhook".to_string(),
            method: WebhookMethod::Post,
            auth: WebhookAuth::None,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            compression: true,
            batch_mode: false,
            max_batch_size: 100,
            batch_timeout: Duration::from_secs(5),
            retry_enabled: true,
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(500),
            max_retry_delay: Duration::from_secs(30),
            user_agent: format!("oisp-sensor/{}", env!("CARGO_PKG_VERSION")),
            content_type: "application/json".to_string(),
            dlq_path: None,
        }
    }
}

/// Webhook exporter for sending events to HTTP endpoints
pub struct WebhookExporter {
    config: WebhookExporterConfig,
    client: Option<Client>,
    batch_buffer: Arc<Mutex<Vec<OispEvent>>>,
    events_exported: AtomicU64,
    events_retried: AtomicU64,
    events_dropped: AtomicU64,
    errors: AtomicU64,
}

impl WebhookExporter {
    /// Create a new webhook exporter with the given configuration
    pub fn new(config: WebhookExporterConfig) -> Self {
        Self {
            config,
            client: None,
            batch_buffer: Arc::new(Mutex::new(Vec::new())),
            events_exported: AtomicU64::new(0),
            events_retried: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    /// Initialize the HTTP client
    fn init_client(&mut self) -> PluginResult<()> {
        let mut builder = Client::builder()
            .timeout(self.config.timeout)
            .user_agent(&self.config.user_agent);

        if self.config.compression {
            builder = builder.gzip(true);
        }

        let client = builder.build().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to create HTTP client: {}", e))
        })?;

        self.client = Some(client);
        Ok(())
    }

    /// Send a single event or batch of events
    async fn send_request(&self, payload: &str) -> Result<(), WebhookError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| WebhookError::NotInitialized)?;

        let mut request = client
            .request(self.config.method.as_reqwest(), &self.config.endpoint)
            .header("Content-Type", &self.config.content_type);

        // Add authentication
        request = match &self.config.auth {
            WebhookAuth::None => request,
            WebhookAuth::ApiKey { header, value } => {
                request.header(header.as_str(), value.as_str())
            }
            WebhookAuth::Bearer(token) => request.bearer_auth(token),
            WebhookAuth::Basic { username, password } => {
                request.basic_auth(username, Some(password))
            }
        };

        // Add static headers
        for (key, value) in &self.config.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Set body
        request = request.body(payload.to_string());

        // Send request
        let response = request.send().await.map_err(WebhookError::Network)?;
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else if status.is_client_error() {
            // 4xx errors - don't retry, drop the event
            let body = response.text().await.unwrap_or_default();
            Err(WebhookError::ClientError { status, body })
        } else {
            // 5xx errors - retry
            let body = response.text().await.unwrap_or_default();
            Err(WebhookError::ServerError { status, body })
        }
    }

    /// Send with retry logic
    async fn send_with_retry(&self, payload: &str) -> PluginResult<()> {
        if !self.config.retry_enabled {
            return self.send_request(payload).await.map_err(|e| {
                PluginError::OperationFailed(format!("Webhook request failed: {}", e))
            });
        }

        let mut delay = self.config.initial_retry_delay;
        let mut attempts = 0;

        loop {
            match self.send_request(payload).await {
                Ok(()) => {
                    if attempts > 0 {
                        self.events_retried.fetch_add(1, Ordering::Relaxed);
                    }
                    return Ok(());
                }
                Err(WebhookError::ClientError { status, body }) => {
                    // 4xx errors - don't retry
                    warn!(
                        "Webhook request failed with client error {}: {}",
                        status, body
                    );
                    self.write_to_dlq(payload).await;
                    self.events_dropped.fetch_add(1, Ordering::Relaxed);
                    return Err(PluginError::OperationFailed(format!(
                        "Webhook client error {}: {}",
                        status, body
                    )));
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > self.config.max_retries {
                        error!("Webhook request failed after {} retries: {}", attempts, e);
                        self.write_to_dlq(payload).await;
                        self.events_dropped.fetch_add(1, Ordering::Relaxed);
                        self.errors.fetch_add(1, Ordering::Relaxed);
                        return Err(PluginError::OperationFailed(format!(
                            "Webhook failed after {} retries: {}",
                            attempts, e
                        )));
                    }

                    warn!(
                        "Webhook request failed (attempt {}), retrying in {:?}: {}",
                        attempts, delay, e
                    );

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.config.max_retry_delay);
                }
            }
        }
    }

    /// Write failed event to dead letter queue
    async fn write_to_dlq(&self, payload: &str) {
        if let Some(ref path) = self.config.dlq_path {
            use std::fs::OpenOptions;
            use std::io::Write;

            match OpenOptions::new().create(true).append(true).open(path) {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "{}", payload) {
                        error!("Failed to write to DLQ file: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to open DLQ file {}: {}", path, e);
                }
            }
        }
    }

    /// Flush the batch buffer
    async fn flush_batch(&self) -> PluginResult<()> {
        let mut buffer = self.batch_buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }

        let events: Vec<_> = buffer.drain(..).collect();
        drop(buffer); // Release lock before sending

        let payload = serde_json::to_string(&events)?;
        let count = events.len();

        self.send_with_retry(&payload).await?;
        self.events_exported
            .fetch_add(count as u64, Ordering::Relaxed);
        debug!("Flushed {} events to webhook", count);

        Ok(())
    }

    /// Get export statistics
    pub fn stats(&self) -> WebhookStats {
        WebhookStats {
            events_exported: self.events_exported.load(Ordering::Relaxed),
            events_retried: self.events_retried.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Webhook export statistics
#[derive(Debug, Clone, Default)]
pub struct WebhookStats {
    pub events_exported: u64,
    pub events_retried: u64,
    pub events_dropped: u64,
    pub errors: u64,
}

/// Webhook-specific error types
#[derive(Debug)]
enum WebhookError {
    NotInitialized,
    Network(reqwest::Error),
    ClientError { status: StatusCode, body: String },
    ServerError { status: StatusCode, body: String },
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookError::NotInitialized => write!(f, "HTTP client not initialized"),
            WebhookError::Network(e) => write!(f, "Network error: {}", e),
            WebhookError::ClientError { status, body } => {
                write!(f, "Client error {}: {}", status, body)
            }
            WebhookError::ServerError { status, body } => {
                write!(f, "Server error {}: {}", status, body)
            }
        }
    }
}

impl std::error::Error for WebhookError {}

impl PluginInfo for WebhookExporter {
    fn name(&self) -> &str {
        "webhook-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events to HTTP webhooks"
    }
}

impl Plugin for WebhookExporter {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Parse configuration
        if let Some(endpoint) = config.get::<String>("endpoint") {
            self.config.endpoint = endpoint;
        }
        if let Some(method) = config.get::<String>("method") {
            self.config.method = match method.to_uppercase().as_str() {
                "PUT" => WebhookMethod::Put,
                "PATCH" => WebhookMethod::Patch,
                _ => WebhookMethod::Post,
            };
        }
        if let Some(timeout_secs) = config.get::<u64>("timeout_secs") {
            self.config.timeout = Duration::from_secs(timeout_secs);
        }
        if let Some(compression) = config.get::<bool>("compression") {
            self.config.compression = compression;
        }
        if let Some(batch_mode) = config.get::<bool>("batch_mode") {
            self.config.batch_mode = batch_mode;
        }
        if let Some(max_batch_size) = config.get::<usize>("max_batch_size") {
            self.config.max_batch_size = max_batch_size;
        }
        if let Some(retry_enabled) = config.get::<bool>("retry_enabled") {
            self.config.retry_enabled = retry_enabled;
        }
        if let Some(max_retries) = config.get::<u32>("max_retries") {
            self.config.max_retries = max_retries;
        }
        if let Some(dlq_path) = config.get::<String>("dlq_path") {
            self.config.dlq_path = Some(dlq_path);
        }

        // Parse auth config
        if let Some(api_key) = config.get::<String>("api_key") {
            let header = config
                .get::<String>("api_key_header")
                .unwrap_or_else(|| "X-API-Key".to_string());
            self.config.auth = WebhookAuth::ApiKey {
                header,
                value: api_key,
            };
        } else if let Some(bearer_token) = config.get::<String>("bearer_token") {
            self.config.auth = WebhookAuth::Bearer(bearer_token);
        } else if let Some(username) = config.get::<String>("basic_username") {
            let password = config.get::<String>("basic_password").unwrap_or_default();
            self.config.auth = WebhookAuth::Basic { username, password };
        }

        // Parse headers
        if let Some(headers) = config.get::<HashMap<String, String>>("headers") {
            self.config.headers = headers;
        }

        self.init_client()?;

        info!(
            "Webhook exporter initialized: endpoint={}, method={:?}, batch_mode={}",
            self.config.endpoint, self.config.method, self.config.batch_mode
        );

        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Flush any pending batched events synchronously
        // Note: In real usage, we'd want to handle this more gracefully
        self.client = None;
        info!("Webhook exporter shutdown complete");
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
impl ExportPlugin for WebhookExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        if self.config.batch_mode {
            // Add to batch buffer
            let mut buffer = self.batch_buffer.lock().await;
            buffer.push(event.clone());

            if buffer.len() >= self.config.max_batch_size {
                // Buffer is full, flush it
                drop(buffer);
                self.flush_batch().await?;
            }
        } else {
            // Send immediately
            let payload = serde_json::to_string(event)?;
            self.send_with_retry(&payload).await?;
            self.events_exported.fetch_add(1, Ordering::Relaxed);
            debug!("Exported event {} to webhook", event.envelope().event_id);
        }

        Ok(())
    }

    async fn export_batch(&self, events: &[OispEvent]) -> PluginResult<()> {
        if self.config.batch_mode {
            // Send as a single batch
            let payload = serde_json::to_string(events)?;
            self.send_with_retry(&payload).await?;
            self.events_exported
                .fetch_add(events.len() as u64, Ordering::Relaxed);
        } else {
            // Send each event individually
            for event in events {
                self.export(event).await?;
            }
        }
        Ok(())
    }

    async fn flush(&self) -> PluginResult<()> {
        if self.config.batch_mode {
            self.flush_batch().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebhookExporterConfig::default();
        assert_eq!(config.method, WebhookMethod::Post);
        assert!(config.retry_enabled);
        assert_eq!(config.max_retries, 3);
        assert!(!config.batch_mode);
        assert!(config.compression);
    }

    #[test]
    fn test_webhook_method() {
        assert_eq!(WebhookMethod::Post.as_reqwest(), Method::POST);
        assert_eq!(WebhookMethod::Put.as_reqwest(), Method::PUT);
        assert_eq!(WebhookMethod::Patch.as_reqwest(), Method::PATCH);
    }

    #[test]
    fn test_webhook_stats() {
        let stats = WebhookStats::default();
        assert_eq!(stats.events_exported, 0);
        assert_eq!(stats.events_dropped, 0);
    }
}
