//! Unix socket server for receiving events from Swift Network Extension
//!
//! The Swift network extension sends events as newline-delimited JSON
//! over a Unix domain socket.

use base64::prelude::*;
use oisp_core::plugins::{RawCaptureEvent, RawEventKind, RawEventMetadata};
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default socket path
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/oisp.sock";

/// Event received from Swift (JSON format)
#[derive(Debug, Deserialize)]
pub struct SwiftCaptureEvent {
    /// Unique event ID
    pub id: String,

    /// Timestamp in nanoseconds
    #[serde(rename = "timestamp_ns")]
    pub timestamp_ns: u64,

    /// Event kind (SslRead or SslWrite)
    pub kind: String,

    /// Process ID
    pub pid: u32,

    /// Thread ID (optional)
    pub tid: Option<u32>,

    /// Data (base64 encoded)
    pub data: String,

    /// Metadata
    pub metadata: SwiftEventMetadata,

    /// Remote host
    #[serde(rename = "remote_host")]
    pub remote_host: Option<String>,

    /// Remote port
    #[serde(rename = "remote_port")]
    pub remote_port: Option<u16>,
}

/// Metadata from Swift
#[derive(Debug, Deserialize)]
pub struct SwiftEventMetadata {
    pub comm: String,
    pub exe: String,
    pub uid: u32,
    pub fd: Option<i32>,
    pub ppid: Option<u32>,
    #[serde(default)]
    pub bundle_id: Option<String>,
}

impl SwiftCaptureEvent {
    /// Convert to RawCaptureEvent
    pub fn into_raw_event(self) -> Result<RawCaptureEvent, String> {
        // Decode base64 data
        let data = BASE64_STANDARD
            .decode(&self.data)
            .map_err(|e| format!("Failed to decode base64 data: {}", e))?;

        // Parse kind
        let kind = match self.kind.as_str() {
            "SslRead" => RawEventKind::SslRead,
            "SslWrite" => RawEventKind::SslWrite,
            other => return Err(format!("Unknown event kind: {}", other)),
        };

        Ok(RawCaptureEvent {
            id: self.id,
            timestamp_ns: self.timestamp_ns,
            kind,
            pid: self.pid,
            tid: self.tid,
            data,
            metadata: RawEventMetadata {
                comm: Some(self.metadata.comm),
                exe: Some(self.metadata.exe),
                uid: Some(self.metadata.uid),
                ppid: self.metadata.ppid,
                fd: self.metadata.fd,
                remote_addr: self.remote_host,
                remote_port: self.remote_port,
                bundle_id: self.metadata.bundle_id,
                ..Default::default()
            },
        })
    }
}

/// Statistics for the socket server
pub struct SocketServerStats {
    pub events_received: AtomicU64,
    pub bytes_received: AtomicU64,
    pub parse_errors: AtomicU64,
    pub connections: AtomicU64,
}

impl Default for SocketServerStats {
    fn default() -> Self {
        Self {
            events_received: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            parse_errors: AtomicU64::new(0),
            connections: AtomicU64::new(0),
        }
    }
}

/// Unix socket server for Swift Network Extension
pub struct SocketServer {
    socket_path: String,
    running: Arc<AtomicBool>,
    stats: Arc<SocketServerStats>,
}

impl SocketServer {
    /// Create a new socket server
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(SocketServerStats::default()),
        }
    }

    /// Create with default socket path
    pub fn with_default_path() -> Self {
        Self::new(DEFAULT_SOCKET_PATH)
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// Get statistics
    pub fn stats(&self) -> &SocketServerStats {
        &self.stats
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the socket server
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

        // Remove existing socket file if it exists
        let socket_path = Path::new(&self.socket_path);
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Create the listener
        let listener = UnixListener::bind(socket_path)?;
        info!("Unix socket server listening on {}", self.socket_path);

        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let stats = self.stats.clone();

        let handle = tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _addr)) => {
                                stats.connections.fetch_add(1, Ordering::Relaxed);
                                info!("New connection from Swift Network Extension");

                                let tx = tx.clone();
                                let stats = stats.clone();
                                let running = running.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, tx, stats, running).await {
                                        error!("Connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Accept error: {}", e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // Check if we should stop
                    }
                }
            }

            info!("Socket server stopped");
        });

        Ok(handle)
    }

    /// Stop the socket server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Handle a single connection from the Swift extension
async fn handle_connection(
    stream: UnixStream,
    tx: mpsc::Sender<RawCaptureEvent>,
    stats: Arc<SocketServerStats>,
    running: Arc<AtomicBool>,
) -> Result<(), std::io::Error> {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while running.load(Ordering::SeqCst) {
        match lines.next_line().await {
            Ok(Some(line)) => {
                stats
                    .bytes_received
                    .fetch_add(line.len() as u64, Ordering::Relaxed);

                // Parse the JSON event
                match serde_json::from_str::<SwiftCaptureEvent>(&line) {
                    Ok(swift_event) => {
                        debug!(
                            "Received event: {} from pid {}",
                            swift_event.kind, swift_event.pid
                        );

                        // Convert to RawCaptureEvent
                        match swift_event.into_raw_event() {
                            Ok(event) => {
                                stats.events_received.fetch_add(1, Ordering::Relaxed);

                                // Send to the channel
                                if let Err(e) = tx.send(event).await {
                                    warn!("Failed to send event: {}", e);
                                }
                            }
                            Err(e) => {
                                stats.parse_errors.fetch_add(1, Ordering::Relaxed);
                                warn!("Failed to convert event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        stats.parse_errors.fetch_add(1, Ordering::Relaxed);
                        warn!(
                            "Failed to parse JSON: {} - line: {}",
                            e,
                            &line[..line.len().min(100)]
                        );
                    }
                }
            }
            Ok(None) => {
                // Connection closed
                info!("Connection closed by Swift extension");
                break;
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swift_event() {
        let json = r#"{
            "id": "01234567890abcdef01234567890abcd",
            "timestamp_ns": 1703680000000000000,
            "kind": "SslWrite",
            "pid": 12345,
            "tid": 67890,
            "data": "SGVsbG8gV29ybGQ=",
            "metadata": {
                "comm": "python3",
                "exe": "/usr/bin/python3",
                "uid": 501,
                "fd": 5,
                "ppid": 1234
            },
            "remote_host": "api.openai.com",
            "remote_port": 443
        }"#;

        let event: SwiftCaptureEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.kind, "SslWrite");
        assert_eq!(event.pid, 12345);
        assert_eq!(event.metadata.comm, "python3");
        assert_eq!(event.remote_host, Some("api.openai.com".to_string()));

        let raw = event.into_raw_event().unwrap();
        assert_eq!(raw.data, b"Hello World");
        assert!(matches!(raw.kind, RawEventKind::SslWrite));
        assert_eq!(raw.metadata.comm, Some("python3".to_string()));
        assert_eq!(raw.metadata.remote_addr, Some("api.openai.com".to_string()));
        assert_eq!(raw.metadata.remote_port, Some(443));
    }
}
