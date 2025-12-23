//! Main eBPF capture implementation

#![cfg(target_os = "linux")]

use crate::ssl::find_ssl_libraries;
use crate::types::{
    FileOpenEvent, NetworkConnectEvent, ProcessExecEvent, ProcessExitEvent, SslEvent, SslEventType,
    MAX_DATA_LEN,
};

use async_trait::async_trait;
use oisp_core::plugins::{
    CapturePlugin, CaptureStats, Plugin, PluginConfig, PluginError, PluginInfo, PluginResult,
    RawCaptureEvent, RawEventKind, RawEventMetadata,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

/// Socket info cached in userspace for correlation
#[derive(Debug, Clone)]
pub struct SocketCacheEntry {
    /// Remote address (IPv4 or IPv6 string)
    pub remote_addr: String,
    /// Remote port
    pub remote_port: u16,
    /// Timestamp when connection was established (ns)
    pub connect_time_ns: u64,
}

/// Cache of socket connections per PID for SSL correlation
/// Key: PID, Value: Most recent connection info
///
/// This is a simplification - in practice, a process may have multiple connections.
/// For now, we use the most recent connection which works well for single-threaded
/// AI agent processes making sequential API calls.
type SocketCache = Arc<RwLock<HashMap<u32, SocketCacheEntry>>>;

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

/// Filter configuration that can be sent to the eBPF program
#[derive(Debug, Clone, Default)]
pub struct EbpfFilterConfig {
    /// PIDs to trace (empty = all)
    pub pids: Vec<u32>,
    /// Process names (comm) to trace (empty = all)
    pub comms: Vec<String>,
}

// Config flag constants (must match eBPF side)
const CONFIG_KEY_FLAGS: u32 = 0;
const FLAG_PID_FILTER_ENABLED: u32 = 1 << 0;
const FLAG_COMM_FILTER_ENABLED: u32 = 1 << 1;
const COMM_LEN: usize = 16;

/// eBPF capture plugin
pub struct EbpfCapture {
    config: EbpfCaptureConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
    /// Socket cache for correlating SSL events with network connections
    socket_cache: SocketCache,
    /// Current filter configuration
    filter_config: Arc<RwLock<EbpfFilterConfig>>,
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
            socket_cache: Arc::new(RwLock::new(HashMap::new())),
            filter_config: Arc::new(RwLock::new(EbpfFilterConfig::default())),
        }
    }

    /// Get the current filter configuration
    pub fn get_filter_config(&self) -> EbpfFilterConfig {
        self.filter_config.read().unwrap().clone()
    }

    /// Set PIDs to filter (empty = all PIDs)
    ///
    /// Note: This only takes effect after the eBPF program is loaded.
    /// For dynamic updates, use update_pid_filter() after start().
    pub fn set_pid_filter(&mut self, pids: Vec<u32>) {
        if let Ok(mut config) = self.filter_config.write() {
            config.pids = pids;
        }
    }

    /// Set process names to filter (empty = all processes)
    ///
    /// Note: This only takes effect after the eBPF program is loaded.
    /// For dynamic updates, use update_comm_filter() after start().
    pub fn set_comm_filter(&mut self, comms: Vec<String>) {
        if let Ok(mut config) = self.filter_config.write() {
            config.comms = comms;
        }
    }

    /// Apply filter configuration to eBPF maps
    fn apply_filters_to_ebpf(&self, ebpf: &mut aya::Ebpf) -> PluginResult<()> {
        use aya::maps::HashMap as AyaHashMap;

        let filter_config = self.filter_config.read().unwrap();
        let mut flags: u32 = 0;

        // Apply PID filter if configured
        if !filter_config.pids.is_empty() {
            flags |= FLAG_PID_FILTER_ENABLED;

            if let Some(map) = ebpf.map_mut("TARGET_PIDS") {
                let mut target_pids: AyaHashMap<_, u32, u8> = map.try_into().map_err(|e| {
                    PluginError::InitializationFailed(format!(
                        "Failed to get TARGET_PIDS map: {}",
                        e
                    ))
                })?;

                for &pid in &filter_config.pids {
                    target_pids.insert(pid, 1, 0).map_err(|e| {
                        PluginError::InitializationFailed(format!(
                            "Failed to insert PID {} into filter: {}",
                            pid, e
                        ))
                    })?;
                }
                info!("Applied PID filter: {:?}", filter_config.pids);
            } else {
                debug!("TARGET_PIDS map not found - PID filtering unavailable");
            }
        }

        // Apply comm filter if configured
        if !filter_config.comms.is_empty() {
            flags |= FLAG_COMM_FILTER_ENABLED;

            if let Some(map) = ebpf.map_mut("TARGET_COMMS") {
                let mut target_comms: AyaHashMap<_, [u8; COMM_LEN], u8> =
                    map.try_into().map_err(|e| {
                        PluginError::InitializationFailed(format!(
                            "Failed to get TARGET_COMMS map: {}",
                            e
                        ))
                    })?;

                for comm in &filter_config.comms {
                    let mut comm_bytes = [0u8; COMM_LEN];
                    let bytes = comm.as_bytes();
                    let len = bytes.len().min(COMM_LEN - 1); // Leave room for null terminator
                    comm_bytes[..len].copy_from_slice(&bytes[..len]);

                    target_comms.insert(comm_bytes, 1, 0).map_err(|e| {
                        PluginError::InitializationFailed(format!(
                            "Failed to insert comm '{}' into filter: {}",
                            comm, e
                        ))
                    })?;
                }
                info!("Applied comm filter: {:?}", filter_config.comms);
            } else {
                debug!("TARGET_COMMS map not found - comm filtering unavailable");
            }
        }

        // Set config flags
        if flags != 0 {
            if let Some(map) = ebpf.map_mut("CONFIG_FLAGS") {
                let mut config_flags: AyaHashMap<_, u32, u32> = map.try_into().map_err(|e| {
                    PluginError::InitializationFailed(format!(
                        "Failed to get CONFIG_FLAGS map: {}",
                        e
                    ))
                })?;

                config_flags
                    .insert(CONFIG_KEY_FLAGS, flags, 0)
                    .map_err(|e| {
                        PluginError::InitializationFailed(format!(
                            "Failed to set config flags: {}",
                            e
                        ))
                    })?;
                info!("Applied config flags: {:#x}", flags);
            } else {
                debug!("CONFIG_FLAGS map not found - filtering unavailable");
            }
        }

        Ok(())
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

    /// Convert SslEvent to RawCaptureEvent, enriching with socket info if available
    fn ssl_event_to_raw(event: &SslEvent, socket_cache: &SocketCache) -> RawCaptureEvent {
        let kind = match event.event_type {
            SslEventType::Write => RawEventKind::SslWrite,
            SslEventType::Read => RawEventKind::SslRead,
        };

        let captured_len = (event.captured_len as usize).min(MAX_DATA_LEN);
        let data = event.data[..captured_len].to_vec();

        // Look up socket info from cache
        let (remote_addr, remote_port) = {
            if let Ok(cache) = socket_cache.read() {
                if let Some(entry) = cache.get(&event.pid) {
                    (Some(entry.remote_addr.clone()), Some(entry.remote_port))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        };

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
                remote_addr,
                remote_port,
                ..Default::default()
            },
        }
    }

    /// Convert NetworkConnectEvent to RawCaptureEvent
    fn network_event_to_raw(event: &NetworkConnectEvent) -> RawCaptureEvent {
        RawCaptureEvent {
            id: Ulid::new().to_string(),
            timestamp_ns: event.timestamp_ns,
            kind: RawEventKind::NetworkConnect,
            pid: event.pid,
            tid: Some(event.tid),
            data: Vec::new(), // Network events don't carry payload data
            metadata: RawEventMetadata {
                comm: Some(event.comm_str()),
                uid: Some(event.uid),
                fd: Some(event.fd),
                remote_addr: Some(event.addr_str()),
                remote_port: Some(event.port),
                ..Default::default()
            },
        }
    }

    /// Convert ProcessExecEvent to RawCaptureEvent
    fn process_exec_event_to_raw(event: &ProcessExecEvent) -> RawCaptureEvent {
        RawCaptureEvent {
            id: Ulid::new().to_string(),
            timestamp_ns: event.timestamp_ns,
            kind: RawEventKind::ProcessExec,
            pid: event.pid,
            tid: Some(event.tid),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some(event.comm_str()),
                ppid: Some(event.ppid),
                uid: Some(event.uid),
                exe: Some(event.filename_str()),
                ..Default::default()
            },
        }
    }

    /// Convert ProcessExitEvent to RawCaptureEvent
    fn process_exit_event_to_raw(event: &ProcessExitEvent) -> RawCaptureEvent {
        let mut extra = std::collections::HashMap::new();
        extra.insert("exit_code".to_string(), serde_json::json!(event.exit_code));

        RawCaptureEvent {
            id: Ulid::new().to_string(),
            timestamp_ns: event.timestamp_ns,
            kind: RawEventKind::ProcessExit,
            pid: event.pid,
            tid: Some(event.tid),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some(event.comm_str()),
                ppid: Some(event.ppid),
                extra,
                ..Default::default()
            },
        }
    }

    /// Convert FileOpenEvent to RawCaptureEvent
    fn file_open_event_to_raw(event: &FileOpenEvent) -> RawCaptureEvent {
        let mut extra = std::collections::HashMap::new();
        extra.insert("flags".to_string(), serde_json::json!(event.flags));
        extra.insert("mode".to_string(), serde_json::json!(event.mode));

        RawCaptureEvent {
            id: Ulid::new().to_string(),
            timestamp_ns: event.timestamp_ns,
            kind: RawEventKind::FileOpen,
            pid: event.pid,
            tid: Some(event.tid),
            data: Vec::new(),
            metadata: RawEventMetadata {
                comm: Some(event.comm_str()),
                ppid: Some(event.ppid),
                uid: Some(event.uid),
                path: Some(event.path_str()),
                extra,
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

        // Apply filter configuration to eBPF maps
        self.apply_filters_to_ebpf(&mut ebpf)?;

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
        let socket_cache = self.socket_cache.clone();

        tokio::spawn(async move {
            // Move ebpf ownership into the task so it stays alive
            // and create ring buffers inside the task
            let mut ebpf = ebpf;

            // Get SSL ring buffer map (inside the spawned task to avoid lifetime issues)
            let ssl_ring_buf = match ebpf.map_mut("SSL_EVENTS") {
                Some(map) => match RingBuf::try_from(map) {
                    Ok(rb) => rb,
                    Err(e) => {
                        error!("Failed to create SSL ring buffer: {}", e);
                        return;
                    }
                },
                None => {
                    error!("SSL_EVENTS map not found");
                    return;
                }
            };
            let mut ssl_ring_buf = ssl_ring_buf;

            // Get Network ring buffer map (optional - may not exist in older eBPF programs)
            let network_ring_buf = match ebpf.map_mut("NETWORK_EVENTS") {
                Some(map) => match RingBuf::try_from(map) {
                    Ok(rb) => {
                        info!("NETWORK_EVENTS ring buffer initialized");
                        Some(rb)
                    }
                    Err(e) => {
                        warn!("Failed to create network ring buffer: {}", e);
                        None
                    }
                },
                None => {
                    debug!("NETWORK_EVENTS map not found - network capture disabled");
                    None
                }
            };
            let mut network_ring_buf = network_ring_buf;

            // Get Process ring buffer map (optional)
            let process_ring_buf = match ebpf.map_mut("PROCESS_EVENTS") {
                Some(map) => match RingBuf::try_from(map) {
                    Ok(rb) => {
                        info!("PROCESS_EVENTS ring buffer initialized");
                        Some(rb)
                    }
                    Err(e) => {
                        warn!("Failed to create process ring buffer: {}", e);
                        None
                    }
                },
                None => {
                    debug!("PROCESS_EVENTS map not found - process capture disabled");
                    None
                }
            };
            let mut process_ring_buf = process_ring_buf;

            // Get File ring buffer map (optional)
            let file_ring_buf = match ebpf.map_mut("FILE_EVENTS") {
                Some(map) => match RingBuf::try_from(map) {
                    Ok(rb) => {
                        info!("FILE_EVENTS ring buffer initialized");
                        Some(rb)
                    }
                    Err(e) => {
                        warn!("Failed to create file ring buffer: {}", e);
                        None
                    }
                },
                None => {
                    debug!("FILE_EVENTS map not found - file capture disabled");
                    None
                }
            };
            let mut file_ring_buf = file_ring_buf;

            while running.load(Ordering::SeqCst) {
                // Poll SSL ring buffer (non-blocking)
                while let Some(item) = ssl_ring_buf.next() {
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

                        // Convert to RawCaptureEvent (with socket correlation)
                        let raw_event = Self::ssl_event_to_raw(event, &socket_cache);

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

                // Poll Network ring buffer (non-blocking)
                if let Some(ref mut net_rb) = network_ring_buf {
                    while let Some(item) = net_rb.next() {
                        let data = item.as_ref();
                        if data.len() >= std::mem::size_of::<NetworkConnectEvent>() {
                            let event: &NetworkConnectEvent =
                                unsafe { &*(data.as_ptr() as *const NetworkConnectEvent) };

                            // Update socket cache for SSL correlation (only for successful connections)
                            if event.is_success() {
                                if let Ok(mut cache) = socket_cache.write() {
                                    cache.insert(
                                        event.pid,
                                        SocketCacheEntry {
                                            remote_addr: event.addr_str(),
                                            remote_port: event.port,
                                            connect_time_ns: event.timestamp_ns,
                                        },
                                    );
                                    // Limit cache size to prevent unbounded growth
                                    if cache.len() > 10000 {
                                        // Remove oldest entries (simple LRU approximation)
                                        let keys_to_remove: Vec<u32> =
                                            cache.iter().take(1000).map(|(k, _)| *k).collect();
                                        for key in keys_to_remove {
                                            cache.remove(&key);
                                        }
                                    }
                                }
                            }

                            // Convert to RawCaptureEvent
                            let raw_event = Self::network_event_to_raw(event);

                            // Update stats
                            stats.events_captured.fetch_add(1, Ordering::Relaxed);

                            // Send to pipeline
                            if let Err(e) = tx.send(raw_event).await {
                                error!("Failed to send network event to pipeline: {}", e);
                                stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }

                // Poll Process ring buffer (non-blocking)
                if let Some(ref mut proc_rb) = process_ring_buf {
                    while let Some(item) = proc_rb.next() {
                        let data = item.as_ref();

                        // Try to parse as ProcessExecEvent first (larger struct)
                        if data.len() >= std::mem::size_of::<ProcessExecEvent>() {
                            let event: &ProcessExecEvent =
                                unsafe { &*(data.as_ptr() as *const ProcessExecEvent) };

                            // Convert to RawCaptureEvent
                            let raw_event = Self::process_exec_event_to_raw(event);

                            // Update stats
                            stats.events_captured.fetch_add(1, Ordering::Relaxed);

                            // Send to pipeline
                            if let Err(e) = tx.send(raw_event).await {
                                error!("Failed to send process exec event to pipeline: {}", e);
                                stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                            }
                        } else if data.len() >= std::mem::size_of::<ProcessExitEvent>() {
                            // Try as ProcessExitEvent (smaller struct)
                            let event: &ProcessExitEvent =
                                unsafe { &*(data.as_ptr() as *const ProcessExitEvent) };

                            // Convert to RawCaptureEvent
                            let raw_event = Self::process_exit_event_to_raw(event);

                            // Update stats
                            stats.events_captured.fetch_add(1, Ordering::Relaxed);

                            // Send to pipeline
                            if let Err(e) = tx.send(raw_event).await {
                                error!("Failed to send process exit event to pipeline: {}", e);
                                stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }

                // Poll File ring buffer (non-blocking)
                if let Some(ref mut file_rb) = file_ring_buf {
                    while let Some(item) = file_rb.next() {
                        let data = item.as_ref();
                        if data.len() >= std::mem::size_of::<FileOpenEvent>() {
                            let event: &FileOpenEvent =
                                unsafe { &*(data.as_ptr() as *const FileOpenEvent) };

                            // Convert to RawCaptureEvent
                            let raw_event = Self::file_open_event_to_raw(event);

                            // Update stats
                            stats.events_captured.fetch_add(1, Ordering::Relaxed);

                            // Send to pipeline
                            if let Err(e) = tx.send(raw_event).await {
                                error!("Failed to send file event to pipeline: {}", e);
                                stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                            }
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
