//! Main eBPF capture implementation

#![cfg(target_os = "linux")]

use crate::ssl::find_ssl_libraries;
use crate::types::{SslEvent, SslEventType, MAX_DATA_LEN};

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent, RawEventKind, RawEventMetadata,
};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

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

    /// Binary paths for SSL library detection (auto-detected if empty)
    pub ssl_binary_paths: Vec<String>,

    /// Process name filter (empty = all)
    pub comm_filter: Vec<String>,

    /// PID filter (None = all)
    pub pid_filter: Option<u32>,

    /// Path to pre-built eBPF bytecode (required)
    pub ebpf_bytecode_path: Option<String>,
}

impl Default for EbpfCaptureConfig {
    fn default() -> Self {
        Self {
            ssl: true,
            process: true,
            file: true,
            network: true,
            ssl_binary_paths: Vec::new(),
            comm_filter: Vec::new(),
            pid_filter: None,
            ebpf_bytecode_path: None,
        }
    }
}

/// eBPF capture plugin
pub struct EbpfCapture {
    config: EbpfCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
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
        }
    }

    /// Bump memlock rlimit for eBPF (required on older kernels)
    fn bump_memlock_rlimit() -> bool {
        let rlim = libc::rlimit {
            rlim_cur: libc::RLIM_INFINITY,
            rlim_max: libc::RLIM_INFINITY,
        };
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
        if ret != 0 {
            warn!("Failed to remove limit on locked memory (ret={})", ret);
            false
        } else {
            true
        }
    }

    /// Convert SslEvent to RawCaptureEvent
    fn ssl_event_to_raw(event: &SslEvent) -> RawCaptureEvent {
        let kind = match event.event_type {
            SslEventType::Write => RawEventKind::SslWrite,
            SslEventType::Read => RawEventKind::SslRead,
        };

        let captured_len = (event.captured_len as usize).min(MAX_DATA_LEN);
        let data = event.data[..captured_len].to_vec();

        RawCaptureEvent {
            id: Ulid::new().to_string(),
            timestamp_ns: event.timestamp_ns,
            kind,
            pid: event.pid,
            tid: Some(event.tid),
            data,
            metadata: RawEventMetadata {
                comm: Some(event.comm_str()),
                uid: Some(event.uid),
                ..Default::default()
            },
        }
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
        if let Some(path) = config.get::<String>("ebpf_bytecode_path") {
            self.config.ebpf_bytecode_path = Some(path);
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
        use aya::maps::RingBuf;
        use aya::programs::UProbe;
        use aya::Ebpf;

        if self.running.load(Ordering::SeqCst) {
            return Err(PluginError::OperationFailed("Already running".into()));
        }

        self.running.store(true, Ordering::SeqCst);
        info!("Starting eBPF capture...");

        // Bump memlock rlimit
        Self::bump_memlock_rlimit();

        // Find SSL library
        let ssl_libs = if self.config.ssl_binary_paths.is_empty() {
            find_ssl_libraries()
        } else {
            self.config.ssl_binary_paths.clone()
        };

        if ssl_libs.is_empty() {
            return Err(PluginError::InitializationFailed(
                "No SSL libraries found for uprobe attachment".into(),
            ));
        }

        let libssl_path = ssl_libs.first().unwrap().clone();
        info!("Found SSL library: {}", libssl_path);

        // Load eBPF bytecode
        // For now, we expect the bytecode to be provided via config or embedded
        // This is a placeholder - in production, the bytecode would be embedded or loaded from a file
        let bytecode_path = self.config.ebpf_bytecode_path.clone().ok_or_else(|| {
            PluginError::ConfigurationError(
                "eBPF bytecode path not configured. Set 'ebpf_bytecode_path' in config.".into(),
            )
        })?;

        let bytecode = std::fs::read(&bytecode_path).map_err(|e| {
            PluginError::InitializationFailed(format!(
                "Failed to read eBPF bytecode from '{}': {}",
                bytecode_path, e
            ))
        })?;

        info!("Loading eBPF program ({} bytes)...", bytecode.len());

        // Load eBPF program
        let mut ebpf = Ebpf::load(&bytecode).map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to load eBPF program: {}", e))
        })?;

        // Attach SSL_write probes
        let ssl_write: &mut UProbe = ebpf
            .program_mut("ssl_write")
            .ok_or_else(|| PluginError::InitializationFailed("ssl_write program not found".into()))?
            .try_into()
            .map_err(|e| {
                PluginError::InitializationFailed(format!(
                    "ssl_write is not a uprobe program: {}",
                    e
                ))
            })?;
        ssl_write.load().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to load ssl_write: {}", e))
        })?;
        // UProbe::attach signature: (fn_name, target, pid, offset)
        let libssl_target = libssl_path.as_str();
        ssl_write
            .attach("SSL_write", libssl_target, self.config.pid_filter, None)
            .map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to attach ssl_write: {}", e))
            })?;
        info!("Attached uprobe to SSL_write");

        let ssl_write_ret: &mut UProbe = ebpf
            .program_mut("ssl_write_ret")
            .ok_or_else(|| {
                PluginError::InitializationFailed("ssl_write_ret program not found".into())
            })?
            .try_into()
            .map_err(|e| {
                PluginError::InitializationFailed(format!(
                    "ssl_write_ret is not a uretprobe program: {}",
                    e
                ))
            })?;
        ssl_write_ret.load().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to load ssl_write_ret: {}", e))
        })?;
        ssl_write_ret
            .attach("SSL_write", libssl_target, self.config.pid_filter, None)
            .map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to attach ssl_write_ret: {}", e))
            })?;
        info!("Attached uretprobe to SSL_write");

        // Attach SSL_read probes
        let ssl_read: &mut UProbe = ebpf
            .program_mut("ssl_read")
            .ok_or_else(|| PluginError::InitializationFailed("ssl_read program not found".into()))?
            .try_into()
            .map_err(|e| {
                PluginError::InitializationFailed(format!(
                    "ssl_read is not a uprobe program: {}",
                    e
                ))
            })?;
        ssl_read.load().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to load ssl_read: {}", e))
        })?;
        ssl_read
            .attach("SSL_read", libssl_target, self.config.pid_filter, None)
            .map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to attach ssl_read: {}", e))
            })?;
        info!("Attached uprobe to SSL_read");

        let ssl_read_ret: &mut UProbe = ebpf
            .program_mut("ssl_read_ret")
            .ok_or_else(|| {
                PluginError::InitializationFailed("ssl_read_ret program not found".into())
            })?
            .try_into()
            .map_err(|e| {
                PluginError::InitializationFailed(format!(
                    "ssl_read_ret is not a uretprobe program: {}",
                    e
                ))
            })?;
        ssl_read_ret.load().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to load ssl_read_ret: {}", e))
        })?;
        ssl_read_ret
            .attach("SSL_read", libssl_target, self.config.pid_filter, None)
            .map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to attach ssl_read_ret: {}", e))
            })?;
        info!("Attached uretprobe to SSL_read");

        info!("eBPF capture started, polling ring buffer...");

        // Spawn background task to poll ring buffer
        let running = self.running.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            // Move ebpf ownership into the task so it stays alive
            // and create ring buffer inside the task
            let mut ebpf = ebpf;

            // Get ring buffer map (inside the spawned task to avoid lifetime issues)
            let ring_buf = match ebpf.map_mut("SSL_EVENTS") {
                Some(map) => match RingBuf::try_from(map) {
                    Ok(rb) => rb,
                    Err(e) => {
                        error!("Failed to create ring buffer: {}", e);
                        return;
                    }
                },
                None => {
                    error!("SSL_EVENTS map not found");
                    return;
                }
            };
            let mut ring_buf = ring_buf;

            while running.load(Ordering::SeqCst) {
                // Poll ring buffer (non-blocking)
                while let Some(item) = ring_buf.next() {
                    let data = item.as_ref();
                    if data.len() >= std::mem::size_of::<SslEvent>() {
                        let event: &SslEvent = unsafe { &*(data.as_ptr() as *const SslEvent) };

                        // Sanity check: data_len should be reasonable
                        if event.data_len > 1_000_000 {
                            debug!(
                                "Skipping event with unreasonable data_len: {}",
                                event.data_len
                            );
                            continue;
                        }

                        // Convert to RawCaptureEvent
                        let raw_event = Self::ssl_event_to_raw(event);

                        // Update stats
                        stats.events_captured.fetch_add(1, Ordering::Relaxed);
                        stats
                            .bytes_captured
                            .fetch_add(event.captured_len as u64, Ordering::Relaxed);

                        // Send to pipeline
                        if let Err(e) = tx.send(raw_event).await {
                            error!("Failed to send event to pipeline: {}", e);
                            stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }

                // Sleep briefly to avoid busy-waiting
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            // Keep ebpf alive by explicitly referencing it
            drop(ebpf);
            info!("eBPF ring buffer polling stopped");
        });

        Ok(())
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping eBPF capture...");
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
