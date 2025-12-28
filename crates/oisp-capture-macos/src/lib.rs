//! macOS capture for OISP Sensor
//!
//! Uses Endpoint Security Framework for process/file events
//! and Network Extension for network capture.
//!
//! The architecture on macOS is:
//! 1. Swift Network Extension (System Extension) captures SSL traffic
//! 2. Swift sends events over Unix domain socket to this Rust crate
//! 3. This crate implements the CapturePlugin trait for oisp-sensor
//!
//! Note: Full capture requires a System Extension which must be:
//! - Signed with an Apple Developer ID
//! - Notarized by Apple
//! - Approved by the user in System Preferences

#[cfg(target_os = "macos")]
pub mod socket_server;

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent,
};
#[cfg(target_os = "macos")]
use socket_server::{SocketServer, DEFAULT_SOCKET_PATH};
#[cfg(not(target_os = "macos"))]
const DEFAULT_SOCKET_PATH: &str = "/tmp/oisp.sock";
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
#[cfg(target_os = "macos")]
use tokio::task::JoinHandle;
use tracing::info;
#[cfg(target_os = "macos")]
use tracing::{error, warn};

/// macOS capture configuration
#[derive(Debug, Clone)]
pub struct MacOSCaptureConfig {
    /// Enable process capture (via ESF or libproc)
    pub process: bool,

    /// Enable file capture (via ESF or FSEvents)
    pub file: bool,

    /// Enable network capture (via Network Extension or lsof)
    pub network: bool,

    /// Use System Extension for full capture (requires approval)
    pub use_system_extension: bool,

    /// Unix socket path for receiving events from Swift extension
    pub socket_path: String,
}

impl Default for MacOSCaptureConfig {
    fn default() -> Self {
        Self {
            process: true,
            file: true,
            network: true,
            use_system_extension: true,
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
        }
    }
}

/// macOS capture plugin
pub struct MacOSCapture {
    config: MacOSCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
    #[cfg(target_os = "macos")]
    socket_server: Option<SocketServer>,
    #[cfg(target_os = "macos")]
    server_handle: Option<JoinHandle<()>>,
}

struct CaptureStatsInner {
    events_captured: AtomicU64,
    events_dropped: AtomicU64,
    bytes_captured: AtomicU64,
    errors: AtomicU64,
}

impl MacOSCapture {
    pub fn new() -> Self {
        Self::with_config(MacOSCaptureConfig::default())
    }

    pub fn with_config(config: MacOSCaptureConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(CaptureStatsInner {
                events_captured: AtomicU64::new(0),
                events_dropped: AtomicU64::new(0),
                bytes_captured: AtomicU64::new(0),
                errors: AtomicU64::new(0),
            }),
            #[cfg(target_os = "macos")]
            socket_server: None,
            #[cfg(target_os = "macos")]
            server_handle: None,
        }
    }

    /// Check if System Extension is installed and approved
    #[cfg(target_os = "macos")]
    pub fn is_system_extension_available(&self) -> bool {
        // Check if the extension's socket is responding or check via systemextensionctl
        std::path::Path::new(&self.config.socket_path).exists()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn is_system_extension_available(&self) -> bool {
        false
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &str {
        &self.config.socket_path
    }
}

impl Default for MacOSCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for MacOSCapture {
    fn name(&self) -> &str {
        "macos-capture"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "macOS capture using Endpoint Security Framework and Network Extension"
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "macos")
    }
}

impl Plugin for MacOSCapture {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        if let Some(process) = config.get::<bool>("process") {
            self.config.process = process;
        }
        if let Some(file) = config.get::<bool>("file") {
            self.config.file = file;
        }
        if let Some(network) = config.get::<bool>("network") {
            self.config.network = network;
        }
        if let Some(socket_path) = config.get::<String>("socket_path") {
            self.config.socket_path = socket_path;
        }
        if let Some(use_sysext) = config.get::<bool>("use_system_extension") {
            self.config.use_system_extension = use_sysext;
        }
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        self.running.store(false, Ordering::SeqCst);

        // Stop the socket server
        #[cfg(target_os = "macos")]
        if let Some(server) = &self.socket_server {
            server.stop();
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
impl CapturePlugin for MacOSCapture {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = tx;
            return Err(PluginError::NotSupported);
        }

        #[cfg(target_os = "macos")]
        {
            if self.running.load(Ordering::SeqCst) {
                return Err(PluginError::OperationFailed("Already running".into()));
            }

            self.running.store(true, Ordering::SeqCst);

            if self.config.use_system_extension && self.config.network {
                info!(
                    "Starting macOS capture with Network Extension (socket: {})",
                    self.config.socket_path
                );

                // Create and start the socket server
                let server = SocketServer::new(&self.config.socket_path);
                let stats = self.stats.clone();

                // Create a wrapper channel that updates our stats
                let (internal_tx, mut internal_rx) = mpsc::channel::<RawCaptureEvent>(1000);

                // Forward events from internal channel to external channel, updating stats
                let external_tx = tx.clone();
                let running = self.running.clone();
                tokio::spawn(async move {
                    while running.load(Ordering::SeqCst) {
                        match internal_rx.recv().await {
                            Some(event) => {
                                let bytes = event.data.len() as u64;
                                stats.events_captured.fetch_add(1, Ordering::Relaxed);
                                stats.bytes_captured.fetch_add(bytes, Ordering::Relaxed);

                                if external_tx.send(event).await.is_err() {
                                    stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                                    warn!("Failed to forward event - channel closed");
                                    break;
                                }
                            }
                            None => {
                                info!("Internal event channel closed");
                                break;
                            }
                        }
                    }
                });

                // Start the socket server
                match server.start(internal_tx).await {
                    Ok(handle) => {
                        self.socket_server = Some(server);
                        self.server_handle = Some(handle);
                        info!("Socket server started successfully");
                    }
                    Err(e) => {
                        error!("Failed to start socket server: {}", e);
                        self.stats.errors.fetch_add(1, Ordering::Relaxed);

                        // Fall back to basic capture
                        warn!("Falling back to basic capture (metadata only)");
                        return self.start_basic_capture(tx).await;
                    }
                }
            } else {
                info!("Starting macOS basic capture (metadata only)");
                return self.start_basic_capture(tx).await;
            }

            Ok(())
        }
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping macOS capture...");
        self.running.store(false, Ordering::SeqCst);

        #[cfg(target_os = "macos")]
        {
            // Stop the socket server
            if let Some(server) = &self.socket_server {
                server.stop();
            }

            // Wait for the server task to finish
            if let Some(handle) = self.server_handle.take() {
                let _ = handle.await;
            }

            self.socket_server = None;
        }

        info!("macOS capture stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn stats(&self) -> CaptureStats {
        CaptureStats {
            events_captured: self.stats.events_captured.load(Ordering::Relaxed),
            events_dropped: self.stats.events_dropped.load(Ordering::Relaxed),
            bytes_captured: self.stats.bytes_captured.load(Ordering::Relaxed),
            errors: self.stats.errors.load(Ordering::Relaxed),
        }
    }
}

// macOS-specific implementation
#[cfg(target_os = "macos")]
impl MacOSCapture {
    /// Start basic capture using libproc, lsof, FSEvents
    /// This doesn't capture SSL content, only metadata
    async fn start_basic_capture(
        &mut self,
        _tx: mpsc::Sender<RawCaptureEvent>,
    ) -> PluginResult<()> {
        warn!("Basic capture mode: Only process metadata will be captured");
        warn!("For full SSL capture, install and enable the OISP System Extension");

        // TODO: Implement basic capture using:
        // - libproc for process info (already have this in Swift)
        // - lsof for network connections
        // - FSEvents for file changes

        Ok(())
    }
}

// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
impl MacOSCapture {
    #[allow(dead_code)]
    async fn start_basic_capture(
        &mut self,
        _tx: mpsc::Sender<RawCaptureEvent>,
    ) -> PluginResult<()> {
        Err(PluginError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info() {
        let capture = MacOSCapture::new();
        assert_eq!(capture.name(), "macos-capture");
        assert!(!capture.version().is_empty());

        #[cfg(target_os = "macos")]
        assert!(capture.is_available());

        #[cfg(not(target_os = "macos"))]
        assert!(!capture.is_available());
    }

    #[test]
    fn test_config() {
        let config = MacOSCaptureConfig::default();
        assert!(config.process);
        assert!(config.network);
        assert!(config.use_system_extension);
        assert_eq!(config.socket_path, DEFAULT_SOCKET_PATH);
    }

    #[test]
    fn test_custom_socket_path() {
        let config = MacOSCaptureConfig {
            socket_path: "/custom/path.sock".to_string(),
            ..Default::default()
        };
        let capture = MacOSCapture::with_config(config);
        assert_eq!(capture.socket_path(), "/custom/path.sock");
    }
}
