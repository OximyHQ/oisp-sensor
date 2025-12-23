//! macOS capture for OISP Sensor
//!
//! Uses Endpoint Security Framework for process/file events
//! and Network Extension for network capture.
//!
//! Note: Full capture requires a System Extension which must be:
//! - Signed with an Apple Developer ID
//! - Notarized by Apple
//! - Approved by the user in System Preferences

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent,
};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
#[cfg(not(target_os = "macos"))]
use tracing::info;
#[cfg(target_os = "macos")]
use tracing::{info, warn};

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
}

impl Default for MacOSCaptureConfig {
    fn default() -> Self {
        Self {
            process: true,
            file: true,
            network: true,
            use_system_extension: false,
        }
    }
}

/// macOS capture plugin
pub struct MacOSCapture {
    config: MacOSCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
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
        }
    }

    /// Check if System Extension is installed and approved
    #[cfg(target_os = "macos")]
    pub fn is_system_extension_available(&self) -> bool {
        // TODO: Check if the system extension is loaded
        false
    }

    #[cfg(not(target_os = "macos"))]
    pub fn is_system_extension_available(&self) -> bool {
        false
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
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        self.running.store(false, Ordering::SeqCst);
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
    async fn start(&mut self, _tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        #[cfg(not(target_os = "macos"))]
        {
            return Err(PluginError::NotSupported);
        }

        #[cfg(target_os = "macos")]
        {
            if self.running.load(Ordering::SeqCst) {
                return Err(PluginError::OperationFailed("Already running".into()));
            }

            self.running.store(true, Ordering::SeqCst);

            if self.config.use_system_extension {
                if self.is_system_extension_available() {
                    info!("Starting macOS capture with System Extension");
                    // TODO: Initialize ESF and Network Extension
                } else {
                    warn!("System Extension not available, falling back to basic capture");
                    // TODO: Use libproc, lsof, FSEvents for basic capture
                }
            } else {
                info!("Starting macOS basic capture (metadata only)");
                // TODO: Implement basic capture using:
                // - libproc for process info
                // - lsof for network connections
                // - FSEvents for file changes
            }

            Ok(())
        }
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping macOS capture...");
        self.running.store(false, Ordering::SeqCst);
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
