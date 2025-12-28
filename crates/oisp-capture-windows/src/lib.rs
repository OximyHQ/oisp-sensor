//! Windows capture for OISP Sensor
//!
//! Receives decrypted SSL/TLS traffic from the Windows Redirector via Named Pipes.
//! The redirector runs as Administrator and performs WinDivert packet capture + TLS MITM.
//! This sensor plugin runs as a normal user and processes the decrypted events.
//!
//! Architecture:
//! - `oisp-redirector.exe` (Admin) -> Named Pipe -> `oisp-sensor.exe` (User)
//!
//! Note: This is the receiving side. The redirector handles the privileged capture.

pub mod pipe_server;

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent,
};
use pipe_server::{PipeServer, DEFAULT_PIPE_PATH};
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// Windows capture configuration
#[derive(Debug, Clone)]
pub struct WindowsCaptureConfig {
    /// Named Pipe path for receiving events from redirector
    pub pipe_path: String,
}

impl Default for WindowsCaptureConfig {
    fn default() -> Self {
        Self {
            pipe_path: DEFAULT_PIPE_PATH.to_string(),
        }
    }
}

/// Windows capture plugin
///
/// Receives events from the Windows Redirector via Named Pipes.
/// The redirector captures and decrypts TLS traffic using WinDivert + MITM proxy.
pub struct WindowsCapture {
    config: WindowsCaptureConfig,
    pipe_server: Option<PipeServer>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    stats: Arc<CaptureStatsInner>,
}

struct CaptureStatsInner {
    /// Events dropped (e.g., when channel is full)
    events_dropped: AtomicU64,
}

impl WindowsCapture {
    pub fn new() -> Self {
        Self::with_config(WindowsCaptureConfig::default())
    }

    pub fn with_config(config: WindowsCaptureConfig) -> Self {
        let pipe_server = Some(PipeServer::new(&config.pipe_path));
        Self {
            config,
            pipe_server,
            server_handle: None,
            stats: Arc::new(CaptureStatsInner {
                events_dropped: AtomicU64::new(0),
            }),
        }
    }

    /// Get the pipe path being used
    pub fn pipe_path(&self) -> &str {
        &self.config.pipe_path
    }
}

impl Default for WindowsCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for WindowsCapture {
    fn name(&self) -> &str {
        "windows-capture"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Windows capture via Named Pipe from OISP Redirector"
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "windows")
    }
}

impl Plugin for WindowsCapture {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(pipe_path) = config.get::<String>("pipe_path") {
            self.config.pipe_path = pipe_path;
            // Recreate pipe server with new path
            self.pipe_server = Some(PipeServer::new(&self.config.pipe_path));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        // Stop the pipe server
        if let Some(ref server) = self.pipe_server {
            server.stop();
        }
        // Abort the server handle if running
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
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
impl CapturePlugin for WindowsCapture {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = tx;
            return Err(PluginError::NotSupported);
        }

        #[cfg(target_os = "windows")]
        {
            // Check if already running
            if let Some(ref server) = self.pipe_server {
                if server.is_running() {
                    return Err(PluginError::OperationFailed("Already running".into()));
                }
            }

            // Take ownership of the pipe server to start it
            let server = self.pipe_server.take().ok_or_else(|| {
                PluginError::OperationFailed("Pipe server not initialized".into())
            })?;

            info!(
                "Starting Windows capture, listening on pipe: {}",
                server.pipe_path()
            );

            // Start the pipe server
            match server.start(tx).await {
                Ok(handle) => {
                    self.server_handle = Some(handle);
                    // Put the server back
                    self.pipe_server = Some(server);
                    info!("Windows capture started, waiting for redirector connection...");
                    Ok(())
                }
                Err(e) => {
                    // Put the server back even on error
                    self.pipe_server = Some(server);
                    Err(PluginError::OperationFailed(format!(
                        "Failed to start pipe server: {}",
                        e
                    )))
                }
            }
        }
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping Windows capture...");

        // Stop the pipe server
        if let Some(ref server) = self.pipe_server {
            server.stop();
        }

        // Wait for server handle to complete
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }

        info!("Windows capture stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.pipe_server
            .as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false)
    }

    fn stats(&self) -> CaptureStats {
        // Get stats from pipe server if available
        let (events, bytes, errors) = if let Some(ref server) = self.pipe_server {
            let stats = server.stats();
            (
                stats.events_received.load(Ordering::Relaxed),
                stats.bytes_received.load(Ordering::Relaxed),
                stats.parse_errors.load(Ordering::Relaxed),
            )
        } else {
            (0, 0, 0)
        };

        CaptureStats {
            events_captured: events,
            events_dropped: self.stats.events_dropped.load(Ordering::Relaxed),
            bytes_captured: bytes,
            errors,
        }
    }
}
