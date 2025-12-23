//! Main eBPF capture implementation

#![cfg(target_os = "linux")]

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent,
};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// eBPF capture configuration
#[derive(Debug, Clone)]
pub struct EbpfCaptureConfig {
    /// Enable SSL/TLS capture
    pub ssl: bool,

    /// Enable process capture
    pub process: bool,

    /// Enable file capture
    pub file: bool,

    /// Enable network capture
    pub network: bool,

    /// Binary paths for SSL library detection
    pub ssl_binary_paths: Vec<String>,

    /// Process name filter
    pub comm_filter: Vec<String>,
}

impl Default for EbpfCaptureConfig {
    fn default() -> Self {
        Self {
            ssl: true,
            process: true,
            file: true,
            network: true,
            ssl_binary_paths: vec![
                "/usr/lib/x86_64-linux-gnu/libssl.so.3".into(),
                "/usr/lib/x86_64-linux-gnu/libssl.so.1.1".into(),
                "/lib/x86_64-linux-gnu/libssl.so.3".into(),
                "/lib/x86_64-linux-gnu/libssl.so.1.1".into(),
            ],
            comm_filter: Vec::new(),
        }
    }
}

/// eBPF capture plugin
pub struct EbpfCapture {
    config: EbpfCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
    tx: Option<mpsc::Sender<RawCaptureEvent>>,
}

struct CaptureStatsInner {
    events_captured: AtomicU64,
    events_dropped: AtomicU64,
    bytes_captured: AtomicU64,
    errors: AtomicU64,
}

impl EbpfCapture {
    pub fn new() -> Self {
        Self::with_config(EbpfCaptureConfig::default())
    }

    pub fn with_config(config: EbpfCaptureConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(CaptureStatsInner {
                events_captured: AtomicU64::new(0),
                events_dropped: AtomicU64::new(0),
                bytes_captured: AtomicU64::new(0),
                errors: AtomicU64::new(0),
            }),
            tx: None,
        }
    }

    /// Find SSL library paths on the system
    fn find_ssl_libraries(&self) -> Vec<String> {
        let mut libs = Vec::new();

        // Standard locations
        let search_paths = [
            "/usr/lib/x86_64-linux-gnu",
            "/lib/x86_64-linux-gnu",
            "/usr/lib64",
            "/lib64",
            "/usr/lib",
            "/lib",
        ];

        let lib_names = ["libssl.so.3", "libssl.so.1.1", "libssl.so"];

        for path in search_paths {
            for lib in lib_names {
                let full_path = format!("{}/{}", path, lib);
                if std::path::Path::new(&full_path).exists() {
                    libs.push(full_path);
                }
            }
        }

        // Add user-specified paths
        for path in &self.config.ssl_binary_paths {
            if std::path::Path::new(path).exists() && !libs.contains(path) {
                libs.push(path.clone());
            }
        }

        libs
    }
}

impl Default for EbpfCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginInfo for EbpfCapture {
    fn name(&self) -> &str {
        "ebpf-capture"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Linux eBPF capture for SSL/TLS, process, file, and network events"
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "linux")
    }
}

impl Plugin for EbpfCapture {
    fn init(&mut self, config: &PluginConfig) -> PluginResult<()> {
        // Parse configuration
        if let Some(ssl) = config.get::<bool>("ssl") {
            self.config.ssl = ssl;
        }
        if let Some(process) = config.get::<bool>("process") {
            self.config.process = process;
        }
        if let Some(file) = config.get::<bool>("file") {
            self.config.file = file;
        }
        if let Some(network) = config.get::<bool>("network") {
            self.config.network = network;
        }
        if let Some(paths) = config.get::<Vec<String>>("ssl_binary_paths") {
            self.config.ssl_binary_paths = paths;
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
impl CapturePlugin for EbpfCapture {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(PluginError::OperationFailed("Already running".into()));
        }

        self.running.store(true, Ordering::SeqCst);
        self.tx = Some(tx.clone());

        info!("Starting eBPF capture...");

        // Find SSL libraries
        if self.config.ssl {
            let ssl_libs = self.find_ssl_libraries();
            if ssl_libs.is_empty() {
                warn!("No SSL libraries found for uprobe attachment");
            } else {
                info!("Found SSL libraries: {:?}", ssl_libs);
            }
        }

        // TODO: Load and attach eBPF programs
        // This is where we would:
        // 1. Load the compiled eBPF programs (from bpf/ directory)
        // 2. Attach uprobes to SSL_read/SSL_write
        // 3. Attach tracepoints for syscalls
        // 4. Set up ring buffer/perf buffer for event delivery

        // For now, we'll just log that we're ready
        info!(
            "eBPF capture started (SSL: {}, Process: {}, File: {}, Network: {})",
            self.config.ssl, self.config.process, self.config.file, self.config.network
        );

        Ok(())
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping eBPF capture...");
        self.running.store(false, Ordering::SeqCst);
        self.tx = None;
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
