//! sslsniff binary runner - extracts and runs embedded sslsniff binary
//!
//! This module handles:
//! 1. Extracting the embedded sslsniff binary (built from libbpf C code)
//! 2. Running it as a subprocess
//! 3. Parsing JSON events from its stdout
//! 4. Converting to OISP events

use oisp_core::plugins::{CapturePlugin, CaptureStats, PluginError, PluginResult, RawCaptureEvent};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Embedded sslsniff binary (built during Docker build)
/// This is included at compile time from the build output
/// The `embedded_sslsniff` cfg is set by build.rs when sslsniff is found
#[cfg(all(target_os = "linux", embedded_sslsniff))]
const EMBEDDED_SSLSNIFF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sslsniff"));

#[cfg(not(all(target_os = "linux", embedded_sslsniff)))]
const EMBEDDED_SSLSNIFF: &[u8] = &[];

/// Configuration for sslsniff runner
///
/// Compatible with the old EbpfCaptureConfig for easy migration
#[derive(Debug, Clone, Default)]
pub struct SslsniffConfig {
    /// Enable SSL capture
    pub ssl: bool,
    /// Enable process capture (not used by sslsniff, for compatibility)
    pub process: bool,
    /// Enable file capture (not used by sslsniff, for compatibility)
    pub file: bool,
    /// Enable network capture (not used by sslsniff, for compatibility)
    pub network: bool,
    /// Paths to SSL binaries (first one used for libssl path)
    pub ssl_binary_paths: Vec<String>,
    /// Filter by process name
    pub comm_filter: Vec<String>,
    /// Filter by PID
    pub pid_filter: Option<u32>,
    /// Path to eBPF bytecode (not used, for compatibility) or sslsniff binary
    pub ebpf_bytecode_path: Option<String>,
}

/// sslsniff-based SSL capture
pub struct SslsniffCapture {
    config: SslsniffConfig,
    running: Arc<AtomicBool>,
    stats: Arc<CaptureStatsInner>,
    child: Option<Child>,
    extracted_path: Option<PathBuf>,
}

struct CaptureStatsInner {
    events_captured: AtomicU64,
    events_dropped: AtomicU64,
    bytes_captured: AtomicU64,
    errors: AtomicU64,
}

impl SslsniffCapture {
    pub fn new() -> Self {
        Self::with_config(SslsniffConfig::default())
    }

    pub fn with_config(config: SslsniffConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(CaptureStatsInner {
                events_captured: AtomicU64::new(0),
                events_dropped: AtomicU64::new(0),
                bytes_captured: AtomicU64::new(0),
                errors: AtomicU64::new(0),
            }),
            child: None,
            extracted_path: None,
        }
    }

    /// Get the path to sslsniff binary - extract embedded if needed
    fn get_sslsniff_path(&mut self) -> PluginResult<PathBuf> {
        // If explicitly configured via ebpf_bytecode_path (which now points to sslsniff)
        if let Some(ref path_str) = self.config.ebpf_bytecode_path {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Ok(path);
            }
            // Not an error - fall through to other methods
            debug!(
                "Configured path not found: {}, trying other locations",
                path_str
            );
        }

        // Check if sslsniff is in PATH
        if let Ok(output) = Command::new("which").arg("sslsniff").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    info!("Found sslsniff in PATH: {}", path_str);
                    return Ok(PathBuf::from(path_str));
                }
            }
        }

        // Check common locations
        for path in &["/usr/local/bin/sslsniff", "/usr/bin/sslsniff", "./sslsniff"] {
            let p = PathBuf::from(path);
            if p.exists() {
                info!("Found sslsniff at: {}", path);
                return Ok(p);
            }
        }

        // Extract embedded binary
        if EMBEDDED_SSLSNIFF.is_empty() {
            return Err(PluginError::InitializationFailed(
                "No embedded sslsniff binary and none found in PATH. \
                 Build with embedded_sslsniff feature or install sslsniff."
                    .into(),
            ));
        }

        let extract_path = std::env::temp_dir().join("oisp-sslsniff");

        // Check if already extracted and has correct size
        if extract_path.exists() {
            if let Ok(meta) = std::fs::metadata(&extract_path) {
                if meta.len() == EMBEDDED_SSLSNIFF.len() as u64 {
                    info!("Using previously extracted sslsniff: {:?}", extract_path);
                    self.extracted_path = Some(extract_path.clone());
                    return Ok(extract_path);
                }
            }
        }

        // Extract the binary
        info!(
            "Extracting embedded sslsniff ({} bytes)...",
            EMBEDDED_SSLSNIFF.len()
        );
        std::fs::write(&extract_path, EMBEDDED_SSLSNIFF).map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to extract sslsniff: {}", e))
        })?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&extract_path)
                .map_err(|e| {
                    PluginError::InitializationFailed(format!("Failed to get metadata: {}", e))
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&extract_path, perms).map_err(|e| {
                PluginError::InitializationFailed(format!("Failed to set executable: {}", e))
            })?;
        }

        info!("Extracted sslsniff to: {:?}", extract_path);
        self.extracted_path = Some(extract_path.clone());
        Ok(extract_path)
    }

    /// Find libssl.so path
    fn find_libssl(&self) -> Option<String> {
        // Check configured paths first
        if let Some(path) = self.config.ssl_binary_paths.first() {
            if !path.is_empty() {
                return Some(path.clone());
            }
        }

        // Try ldconfig first
        if let Ok(output) = Command::new("ldconfig").args(["-p"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("libssl.so") {
                        if let Some(path) = line.split("=>").nth(1) {
                            let path = path.trim();
                            if !path.is_empty() {
                                return Some(path.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Check common paths
        for path in &[
            "/usr/lib/aarch64-linux-gnu/libssl.so.3",
            "/usr/lib/x86_64-linux-gnu/libssl.so.3",
            "/usr/lib/libssl.so.3",
            "/lib/aarch64-linux-gnu/libssl.so.3",
            "/lib/x86_64-linux-gnu/libssl.so.3",
        ] {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        None
    }

    /// Parse a JSON line from sslsniff into a RawCaptureEvent
    fn parse_sslsniff_event(json_line: &str) -> Option<RawCaptureEvent> {
        use oisp_core::plugins::{RawEventKind, RawEventMetadata};

        let value: serde_json::Value = serde_json::from_str(json_line).ok()?;

        let function = value.get("function")?.as_str()?;
        let kind = if function.contains("WRITE") || function.contains("SEND") {
            RawEventKind::SslWrite
        } else {
            RawEventKind::SslRead
        };

        let timestamp_ns = value.get("timestamp_ns")?.as_u64()?;
        let pid = value.get("pid")?.as_u64()? as u32;
        let tid = value.get("tid").and_then(|t| t.as_u64()).map(|t| t as u32);
        let comm = value.get("comm")?.as_str()?.to_string();
        let data_str = value.get("data").and_then(|d| d.as_str()).unwrap_or("");

        // CRITICAL: sslsniff encodes binary data as JSON string with escape sequences.
        // When serde_json decodes e.g. \u008b, it becomes Unicode codepoint U+008B.
        // But .as_bytes() would UTF-8 encode it as [0xC2, 0x8B] (2 bytes!), corrupting the data.
        // We must treat it as Latin-1: each char's codepoint IS the byte value.
        let data: Vec<u8> = data_str.chars().map(|c| c as u8).collect();

        Some(RawCaptureEvent {
            id: ulid::Ulid::new().to_string(),
            timestamp_ns,
            kind,
            pid,
            tid,
            data,
            metadata: RawEventMetadata {
                comm: Some(comm),
                ..Default::default()
            },
        })
    }
}

impl Default for SslsniffCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl oisp_core::plugins::PluginInfo for SslsniffCapture {
    fn name(&self) -> &str {
        "sslsniff-capture"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "SSL/TLS capture using libbpf-based sslsniff"
    }
}

impl oisp_core::plugins::Plugin for SslsniffCapture {
    fn init(&mut self, _config: &oisp_core::plugins::PluginConfig) -> PluginResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        if let Some(ref path) = self.extracted_path {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[async_trait::async_trait]
impl CapturePlugin for SslsniffCapture {
    async fn start(&mut self, tx: mpsc::Sender<RawCaptureEvent>) -> PluginResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(PluginError::OperationFailed("Already running".into()));
        }

        // Get sslsniff path
        let sslsniff_path = self.get_sslsniff_path()?;
        info!("Using sslsniff: {:?}", sslsniff_path);

        // Find libssl
        let libssl = self.find_libssl();
        if libssl.is_some() {
            info!("Found libssl: {:?}", libssl);
        }

        // Build command
        // Note: stderr goes to /dev/null to prevent buffer blocking
        // sslsniff outputs JSON events to stdout only
        let mut cmd = Command::new(&sslsniff_path);
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        // Add binary path for statically-linked SSL (e.g., Node.js with embedded OpenSSL)
        // This allows sslsniff to attach uprobes to the binary itself instead of libssl.so
        if let Some(binary_path) = self.config.ssl_binary_paths.first() {
            if !binary_path.is_empty() && std::path::Path::new(binary_path).exists() {
                info!("Attaching to binary with embedded SSL: {}", binary_path);
                cmd.args(["--binary-path", binary_path]);
            }
        }

        // Add PID filter if specified
        if let Some(pid) = self.config.pid_filter {
            cmd.args(["-p", &pid.to_string()]);
        }

        // Add comm filter if specified
        if let Some(comm) = self.config.comm_filter.first() {
            cmd.args(["-c", comm]);
        }

        // Start sslsniff
        info!("Starting sslsniff...");
        let mut child = cmd.spawn().map_err(|e| {
            PluginError::InitializationFailed(format!("Failed to start sslsniff: {}", e))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            PluginError::InitializationFailed("Failed to capture sslsniff stdout".into())
        })?;

        self.child = Some(child);
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let stats = self.stats.clone();

        // Spawn reader task
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match line {
                    Ok(line) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        // Debug log for every line from sslsniff
                        // Using warn! so it shows up without RUST_LOG=debug
                        // tracing::warn!("sslsniff raw line: {}", line);

                        match Self::parse_sslsniff_event(&line) {
                            Some(event) => {
                                stats.events_captured.fetch_add(1, Ordering::Relaxed);
                                stats
                                    .bytes_captured
                                    .fetch_add(event.data.len() as u64, Ordering::Relaxed);

                                // Send to pipeline (blocking)
                                if tx.blocking_send(event).is_err() {
                                    stats.events_dropped.fetch_add(1, Ordering::Relaxed);
                                    break;
                                }
                            }
                            None => {
                                tracing::warn!("Failed to parse sslsniff event: {}", line);
                                stats.errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error reading sslsniff output: {}", e);
                        stats.errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            info!("sslsniff reader stopped");
        });

        info!("sslsniff capture started");
        Ok(())
    }

    async fn stop(&mut self) -> PluginResult<()> {
        info!("Stopping sslsniff capture...");
        self.running.store(false, Ordering::SeqCst);

        if let Some(ref mut child) = self.child {
            // Send SIGINT for graceful shutdown
            #[cfg(unix)]
            {
                let pid = child.id();
                unsafe {
                    libc::kill(pid as i32, libc::SIGINT);
                }
            }

            // Wait briefly, then force kill if needed
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = child.kill();
            let _ = child.wait();
        }

        self.child = None;
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
