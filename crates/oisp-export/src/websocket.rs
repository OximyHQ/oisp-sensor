//! WebSocket exporter for real-time UI

use async_trait::async_trait;
use oisp_core::events::OispEvent;
use oisp_core::plugins::{ExportPlugin, Plugin, PluginConfig, PluginInfo, PluginResult};
use std::any::Any;
use tokio::sync::broadcast;
use tracing::info;

/// WebSocket exporter configuration
#[derive(Debug, Clone)]
pub struct WebSocketExporterConfig {
    /// Port to listen on
    pub port: u16,

    /// Host to bind to
    pub host: String,

    /// Channel buffer size
    pub buffer_size: usize,
}

impl Default for WebSocketExporterConfig {
    fn default() -> Self {
        Self {
            port: 7777,
            host: "127.0.0.1".to_string(),
            buffer_size: 1000,
        }
    }
}

/// WebSocket exporter for UI connections
pub struct WebSocketExporter {
    config: WebSocketExporterConfig,
    tx: broadcast::Sender<String>,
}

impl WebSocketExporter {
    pub fn new(config: WebSocketExporterConfig) -> Self {
        let (tx, _) = broadcast::channel(config.buffer_size);
        Self { config, tx }
    }

    /// Get a receiver for events
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// Get the broadcast sender for external use
    pub fn sender(&self) -> broadcast::Sender<String> {
        self.tx.clone()
    }
}

impl PluginInfo for WebSocketExporter {
    fn name(&self) -> &str {
        "websocket-exporter"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Exports events via WebSocket for real-time UI"
    }
}

impl Plugin for WebSocketExporter {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(port) = config.get::<u16>("port") {
            self.config.port = port;
        }
        if let Some(host) = config.get::<String>("host") {
            self.config.host = host;
        }

        info!(
            "WebSocket exporter ready on {}:{}",
            self.config.host, self.config.port
        );
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
impl ExportPlugin for WebSocketExporter {
    async fn export(&self, event: &OispEvent) -> PluginResult<()> {
        let json = serde_json::to_string(event)?;

        // Send to all connected clients
        // If no receivers, this is fine - the message is just dropped
        let _ = self.tx.send(json);

        Ok(())
    }
}
