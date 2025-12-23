//! Windows capture for OISP Sensor
//!
//! Uses Event Tracing for Windows (ETW) for event capture.
//!
//! Note: Full capture requires running as Administrator and
//! may require service installation.

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent,
};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// Windows capture configuration
#[derive(Debug, Clone)]
pub struct WindowsCaptureConfig {
    /// Enable process capture
    pub process: bool,

    /// Enable file capture
    pub file: bool,

    /// Enable network capture
    pub network: bool,

    /// Use ETW (requires elevation)
    pub use_etw: bool,
}

impl Default for WindowsCaptureConfig {
    fn default() -> Self {
        Self {
            process: true,
            file: true,
            network: true,
            use_etw: true,
        }
    }
}

/// Windows capture plugin
pub struct WindowsCapture {
    config: WindowsCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
}

struct CaptureStatsInner {
    events_captured: AtomicU64,
    events_dropped: AtomicU64,
    bytes_captured: AtomicU64,
    errors: AtomicU64,
}

impl WindowsCapture {
    pub fn new() -> Self {
        Self::with_config(WindowsCaptureConfig::default())
    }

    pub fn with_config(config: WindowsCaptureConfig) -> Self {
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

    /// Check if running with Administrator privileges
    #[cfg(target_os = "windows")]
    pub fn is_elevated(&self) -> bool {
        // TODO: Check if running as Administrator
        false
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_elevated(&self) -> bool {
        false
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
        "Windows capture using Event Tracing for Windows (ETW)"
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "windows")
    }
}

impl Plugin for WindowsCapture {
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
impl CapturePlugin for WindowsCapture {
    async fn start(&mut self, _tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        #[cfg(not(target_os = "windows"))]
        {
            return Err(PluginError::NotSupported);
        }

        #[cfg(target_os = "windows")]
        {
            if self.running.load(Ordering::SeqCst) {
                return Err(PluginError::OperationFailed("Already running".into()));
            }

            self.running.store(true, Ordering::SeqCst);

            if self.config.use_etw {
                if self.is_elevated() {
                    info!("Starting Windows capture with ETW");
                    // TODO: Initialize ETW providers
                } else {
                    warn!("Not running as Administrator, ETW capture limited");
                    // TODO: Use WMI and other non-elevated APIs
                }
            } else {
                info!("Starting Windows basic capture (metadata only)");
                // TODO: Use WMI, netstat, etc.
            }

            Ok(())
        }
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping Windows capture...");
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
