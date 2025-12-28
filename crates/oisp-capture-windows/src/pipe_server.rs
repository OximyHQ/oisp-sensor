//! Named Pipe server for receiving events from the Windows Redirector
//!
//! The redirector sends events as newline-delimited JSON over a Named Pipe.
//! This is the Windows equivalent of the macOS Unix socket server.

use base64::prelude::*;
use oisp_core::plugins::{RawCaptureEvent, RawEventKind, RawEventMetadata};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;
#[cfg(target_os = "windows")]
use tracing::{debug, error, info};

/// Default named pipe path
pub const DEFAULT_PIPE_PATH: &str = r"\\.\pipe\oisp-capture";

/// Event received from the redirector (JSON format)
#[derive(Debug, Deserialize)]
pub struct RedirectorEvent {
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,

    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,

    /// Event data
    pub data: RedirectorEventData,
}

/// Event data variants
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RedirectorEventData {
    /// SSL data event
    SslData(SslDataEvent),

    /// Connection event
    Connection(ConnectionEvent),

    /// Status event
    Status(StatusEvent),
}

/// SSL data event (decrypted traffic from MITM proxy)
#[derive(Debug, Deserialize)]
pub struct SslDataEvent {
    /// Unique ID
    pub id: String,

    /// Direction: "read" or "write"
    pub direction: String,

    /// Process ID
    pub pid: u32,

    /// Remote host
    pub remote_host: String,

    /// Remote port
    pub remote_port: u16,

    /// Data (base64 encoded)
    pub data: String,

    /// Process metadata
    pub metadata: ProcessMetadata,
}

/// Connection event
#[derive(Debug, Deserialize)]
pub struct ConnectionEvent {
    /// Local address
    pub local_addr: String,

    /// Local port
    pub local_port: u16,

    /// Remote address
    pub remote_addr: String,

    /// Remote port
    pub remote_port: u16,

    /// Process ID
    pub pid: Option<u32>,

    /// Process name
    pub process_name: Option<String>,

    /// Connection state
    pub state: String,
}

/// Status event
#[derive(Debug, Deserialize)]
pub struct StatusEvent {
    /// Status message
    pub status: String,

    /// Packets captured
    pub packets_captured: Option<u64>,

    /// Active connections
    pub active_connections: Option<u64>,
}

/// Process metadata
#[derive(Debug, Deserialize)]
pub struct ProcessMetadata {
    /// Process name
    pub comm: String,

    /// Executable path
    pub exe: String,

    /// User ID (always 0 on Windows)
    pub uid: u32,
}

impl RedirectorEvent {
    /// Convert to RawCaptureEvent if applicable
    pub fn into_raw_event(self) -> Option<RawCaptureEvent> {
        match self.data {
            RedirectorEventData::SslData(ssl) => {
                // Decode base64 data
                let data = match BASE64_STANDARD.decode(&ssl.data) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to decode base64 data: {}", e);
                        return None;
                    }
                };

                // Determine event kind
                let kind = match ssl.direction.as_str() {
                    "read" => RawEventKind::SslRead,
                    "write" => RawEventKind::SslWrite,
                    other => {
                        warn!("Unknown SSL direction: {}", other);
                        return None;
                    }
                };

                Some(RawCaptureEvent {
                    id: ssl.id,
                    timestamp_ns: self.timestamp_ns,
                    kind,
                    pid: ssl.pid,
                    tid: None,
                    data,
                    metadata: RawEventMetadata {
                        comm: Some(ssl.metadata.comm),
                        exe: Some(ssl.metadata.exe),
                        uid: Some(ssl.metadata.uid),
                        ppid: None,
                        fd: None,
                        path: None,
                        remote_addr: Some(ssl.remote_host),
                        remote_port: Some(ssl.remote_port),
                        local_addr: None,
                        local_port: None,
                        extra: Default::default(),
                    },
                })
            }
            RedirectorEventData::Connection(_) => {
                // Connection events are logged but not converted to raw events
                // They're used for tracking/debugging
                None
            }
            RedirectorEventData::Status(_) => {
                // Status events are logged but not converted to raw events
                None
            }
        }
    }
}

/// Statistics for the pipe server
pub struct PipeServerStats {
    pub events_received: AtomicU64,
    pub bytes_received: AtomicU64,
    pub parse_errors: AtomicU64,
    pub connections: AtomicU64,
}

impl Default for PipeServerStats {
    fn default() -> Self {
        Self {
            events_received: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            parse_errors: AtomicU64::new(0),
            connections: AtomicU64::new(0),
        }
    }
}

/// Named Pipe server for Windows Redirector
pub struct PipeServer {
    pipe_path: String,
    running: Arc<AtomicBool>,
    stats: Arc<PipeServerStats>,
}

impl PipeServer {
    /// Create a new pipe server
    pub fn new(pipe_path: impl Into<String>) -> Self {
        Self {
            pipe_path: pipe_path.into(),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(PipeServerStats::default()),
        }
    }

    /// Create with default pipe path
    pub fn with_default_path() -> Self {
        Self::new(DEFAULT_PIPE_PATH)
    }

    /// Get the pipe path
    pub fn pipe_path(&self) -> &str {
        &self.pipe_path
    }

    /// Get statistics
    pub fn stats(&self) -> &PipeServerStats {
        &self.stats
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the pipe server
    #[cfg(target_os = "windows")]
    pub async fn start(
        &self,
        tx: mpsc::Sender<RawCaptureEvent>,
    ) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
        if self.running.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Server already running",
            ));
        }

        self.running.store(true, Ordering::SeqCst);
        info!("Named Pipe server starting on {}", self.pipe_path);

        let running = self.running.clone();
        let stats = self.stats.clone();
        let pipe_path = self.pipe_path.clone();

        let handle = tokio::spawn(async move {
            run_pipe_server(pipe_path, tx, stats, running).await;
        });

        Ok(handle)
    }

    /// Start the pipe server (non-Windows stub)
    #[cfg(not(target_os = "windows"))]
    pub async fn start(
        &self,
        _tx: mpsc::Sender<RawCaptureEvent>,
    ) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Named Pipes are only available on Windows",
        ))
    }

    /// Stop the pipe server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Windows implementation of the pipe server
#[cfg(target_os = "windows")]
async fn run_pipe_server(
    pipe_path: String,
    tx: mpsc::Sender<RawCaptureEvent>,
    stats: Arc<PipeServerStats>,
    running: Arc<AtomicBool>,
) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
        PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    // Convert pipe path to wide string
    let pipe_path_wide: Vec<u16> = OsStr::new(&pipe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    while running.load(Ordering::SeqCst) {
        // Create the named pipe
        let pipe_handle = unsafe {
            CreateNamedPipeW(
                PCWSTR::from_raw(pipe_path_wide.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                65536, // Output buffer size
                65536, // Input buffer size
                0,     // Default timeout
                None,  // Default security
            )
        };

        if pipe_handle == INVALID_HANDLE_VALUE {
            error!("Failed to create named pipe");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }

        info!("Waiting for redirector connection on {}", pipe_path);

        // Wait for client connection
        let connected = unsafe { ConnectNamedPipe(pipe_handle, None) };
        if connected.is_err()
            && unsafe { windows::Win32::Foundation::GetLastError() }
                != windows::Win32::Foundation::ERROR_PIPE_CONNECTED
        {
            warn!("ConnectNamedPipe failed, retrying...");
            unsafe { CloseHandle(pipe_handle) };
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        stats.connections.fetch_add(1, Ordering::Relaxed);
        info!("Redirector connected!");

        // Handle the connection
        handle_pipe_connection(pipe_handle, tx.clone(), stats.clone(), running.clone()).await;

        // Cleanup
        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }

        if running.load(Ordering::SeqCst) {
            info!("Redirector disconnected, waiting for reconnection...");
        }
    }

    info!("Named Pipe server stopped");
}

/// Handle a single pipe connection
#[cfg(target_os = "windows")]
async fn handle_pipe_connection(
    pipe_handle: windows::Win32::Foundation::HANDLE,
    tx: mpsc::Sender<RawCaptureEvent>,
    stats: Arc<PipeServerStats>,
    running: Arc<AtomicBool>,
) {
    use windows::Win32::Storage::FileSystem::ReadFile;

    let mut buffer = vec![0u8; 65536];
    let mut line_buffer = String::new();

    while running.load(Ordering::SeqCst) {
        // Read from pipe
        let mut bytes_read = 0u32;
        let result =
            unsafe { ReadFile(pipe_handle, Some(&mut buffer), Some(&mut bytes_read), None) };

        if result.is_err() || bytes_read == 0 {
            // Pipe closed or error
            break;
        }

        stats
            .bytes_received
            .fetch_add(bytes_read as u64, Ordering::Relaxed);

        // Convert to string and append to buffer
        if let Ok(s) = std::str::from_utf8(&buffer[..bytes_read as usize]) {
            line_buffer.push_str(s);

            // Process complete lines
            while let Some(newline_pos) = line_buffer.find('\n') {
                let line = line_buffer[..newline_pos].to_string();
                line_buffer = line_buffer[newline_pos + 1..].to_string();

                if line.trim().is_empty() {
                    continue;
                }

                // Parse JSON event
                match serde_json::from_str::<RedirectorEvent>(&line) {
                    Ok(event) => {
                        debug!("Received event type: {}", event.event_type);

                        // Convert to RawCaptureEvent if applicable
                        if let Some(raw_event) = event.into_raw_event() {
                            stats.events_received.fetch_add(1, Ordering::Relaxed);

                            if let Err(e) = tx.send(raw_event).await {
                                warn!("Failed to send event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        stats.parse_errors.fetch_add(1, Ordering::Relaxed);
                        warn!(
                            "Failed to parse event: {} - line: {}...",
                            e,
                            &line[..line.len().min(50)]
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssl_event() {
        let json = r#"{
            "type": "ssl_write",
            "timestamp_ns": 1703680000000000000,
            "data": {
                "id": "test-id",
                "direction": "write",
                "pid": 12345,
                "remote_host": "api.openai.com",
                "remote_port": 443,
                "data": "SGVsbG8gV29ybGQ=",
                "metadata": {
                    "comm": "python.exe",
                    "exe": "C:\\Python311\\python.exe",
                    "uid": 0
                }
            }
        }"#;

        let event: RedirectorEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "ssl_write");

        let raw = event.into_raw_event().unwrap();
        assert!(matches!(raw.kind, RawEventKind::SslWrite));
        assert_eq!(raw.pid, 12345);
        assert_eq!(raw.data, b"Hello World");
        assert_eq!(raw.metadata.remote_addr, Some("api.openai.com".to_string()));
    }

    #[test]
    fn test_parse_connection_event() {
        let json = r#"{
            "type": "connection",
            "timestamp_ns": 1703680000000000000,
            "data": {
                "local_addr": "192.168.1.100",
                "local_port": 12345,
                "remote_addr": "104.18.7.192",
                "remote_port": 443,
                "pid": 1234,
                "process_name": "python.exe",
                "state": "Established"
            }
        }"#;

        let event: RedirectorEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "connection");

        // Connection events don't convert to raw events
        assert!(event.into_raw_event().is_none());
    }
}
