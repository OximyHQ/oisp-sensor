//! Named Pipe IPC client for communicating with oisp-sensor
//!
//! The redirector sends captured events to the main sensor process
//! via Windows Named Pipes. Events are sent as newline-delimited JSON.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
#[cfg(target_os = "windows")]
use tracing::info;
use tracing::{debug, warn};

use super::connection::ConnectionInfo;

/// Default named pipe path
pub const DEFAULT_PIPE_PATH: &str = r"\\.\pipe\oisp-capture";

/// Event sent over IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEvent {
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,

    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,

    /// Event data
    pub data: IpcEventData,
}

/// Event data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IpcEventData {
    /// Connection event
    Connection(ConnectionEventData),

    /// SSL data event
    SslData(SslDataEvent),

    /// Status/heartbeat event
    Status(StatusEvent),
}

/// Connection event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEventData {
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

/// SSL data event (decrypted traffic)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Process metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetadata {
    /// Process name
    pub comm: String,

    /// Executable path
    pub exe: String,

    /// User ID (always 0 on Windows for now)
    pub uid: u32,
}

/// Status/heartbeat event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEvent {
    /// Status message
    pub status: String,

    /// Packets captured
    pub packets_captured: u64,

    /// Active connections
    pub active_connections: u64,
}

/// Statistics for IPC client
pub struct IpcClientStats {
    pub events_sent: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub send_errors: AtomicU64,
    pub reconnects: AtomicU64,
}

impl Default for IpcClientStats {
    fn default() -> Self {
        Self {
            events_sent: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            send_errors: AtomicU64::new(0),
            reconnects: AtomicU64::new(0),
        }
    }
}

/// IPC client for communicating with oisp-sensor
pub struct IpcClient {
    /// Pipe path
    pipe_path: String,

    /// Connected state
    connected: Arc<AtomicBool>,

    /// Statistics
    stats: Arc<IpcClientStats>,

    /// Pipe handle (Windows only)
    #[cfg(target_os = "windows")]
    pipe_handle: Option<windows::Win32::Foundation::HANDLE>,
}

impl IpcClient {
    /// Connect to the named pipe
    #[cfg(target_os = "windows")]
    pub async fn connect(pipe_path: &str) -> Result<Self> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE};
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING,
        };

        debug!("Connecting to named pipe: {}", pipe_path);

        // Validate pipe path format
        if !pipe_path.starts_with(r"\\.\pipe\") {
            return Err(anyhow::anyhow!("Invalid pipe path format"));
        }

        let mut client = Self {
            pipe_path: pipe_path.to_string(),
            connected: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(IpcClientStats::default()),
            pipe_handle: None,
        };

        // Try to connect to the pipe
        client.try_connect()?;

        Ok(client)
    }

    /// Connect to the named pipe (non-Windows stub)
    #[cfg(not(target_os = "windows"))]
    pub async fn connect(pipe_path: &str) -> Result<Self> {
        debug!("IPC client (non-Windows stub): {}", pipe_path);

        Ok(Self {
            pipe_path: pipe_path.to_string(),
            connected: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(IpcClientStats::default()),
        })
    }

    /// Try to connect to the pipe (Windows)
    #[cfg(target_os = "windows")]
    fn try_connect(&mut self) -> Result<()> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE};
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING,
        };

        // Convert pipe path to wide string
        let pipe_path_wide: Vec<u16> = OsStr::new(&self.pipe_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Open the pipe for writing
        let handle = unsafe {
            CreateFileW(
                PCWSTR::from_raw(pipe_path_wide.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        match handle {
            Ok(h) if h != INVALID_HANDLE_VALUE => {
                info!("Connected to named pipe: {}", self.pipe_path);
                self.pipe_handle = Some(h);
                self.connected.store(true, Ordering::SeqCst);
                Ok(())
            }
            Ok(_) => {
                warn!("Pipe not available yet: {}", self.pipe_path);
                self.connected.store(false, Ordering::SeqCst);
                Ok(()) // Not an error - sensor might not be running yet
            }
            Err(e) => {
                warn!("Failed to connect to pipe {}: {}", self.pipe_path, e);
                self.connected.store(false, Ordering::SeqCst);
                Ok(()) // Not an error - sensor might not be running yet
            }
        }
    }

    /// Send a connection event
    pub async fn send_connection_event(&mut self, conn: &ConnectionInfo) -> Result<()> {
        let event = IpcEvent {
            event_type: "connection".to_string(),
            timestamp_ns: Self::current_time_ns(),
            data: IpcEventData::Connection(ConnectionEventData {
                local_addr: conn.local_addr.ip().to_string(),
                local_port: conn.local_addr.port(),
                remote_addr: conn.remote_addr.ip().to_string(),
                remote_port: conn.remote_addr.port(),
                pid: conn.pid,
                process_name: conn.process_name.clone(),
                state: format!("{:?}", conn.state),
            }),
        };

        self.send_event(&event).await
    }

    /// Send an SSL data event
    pub async fn send_ssl_event(
        &mut self,
        id: &str,
        direction: &str,
        pid: u32,
        remote_host: &str,
        remote_port: u16,
        data: &[u8],
        process_name: &str,
        exe_path: &str,
    ) -> Result<()> {
        use base64::prelude::*;

        let event = IpcEvent {
            event_type: if direction == "read" {
                "ssl_read"
            } else {
                "ssl_write"
            }
            .to_string(),
            timestamp_ns: Self::current_time_ns(),
            data: IpcEventData::SslData(SslDataEvent {
                id: id.to_string(),
                direction: direction.to_string(),
                pid,
                remote_host: remote_host.to_string(),
                remote_port,
                data: BASE64_STANDARD.encode(data),
                metadata: ProcessMetadata {
                    comm: process_name.to_string(),
                    exe: exe_path.to_string(),
                    uid: 0, // Windows doesn't have Unix UIDs
                },
            }),
        };

        self.send_event(&event).await
    }

    /// Send a status/heartbeat event
    pub async fn send_status(&mut self, packets: u64, connections: u64) -> Result<()> {
        let event = IpcEvent {
            event_type: "status".to_string(),
            timestamp_ns: Self::current_time_ns(),
            data: IpcEventData::Status(StatusEvent {
                status: "running".to_string(),
                packets_captured: packets,
                active_connections: connections,
            }),
        };

        self.send_event(&event).await
    }

    /// Send an event over the pipe (Windows)
    #[cfg(target_os = "windows")]
    async fn send_event(&mut self, event: &IpcEvent) -> Result<()> {
        use windows::Win32::Storage::FileSystem::WriteFile;

        // Try to connect if not connected
        if !self.connected.load(Ordering::SeqCst) {
            self.try_connect()?;
            if !self.connected.load(Ordering::SeqCst) {
                debug!("IPC not connected, event dropped: {:?}", event.event_type);
                return Ok(());
            }
            self.stats.reconnects.fetch_add(1, Ordering::Relaxed);
        }

        // Serialize event to JSON with newline
        let mut json = serde_json::to_string(event).context("Failed to serialize event")?;
        json.push('\n');

        let bytes = json.as_bytes();

        // Write to pipe
        if let Some(handle) = self.pipe_handle {
            let mut bytes_written = 0u32;
            let result = unsafe { WriteFile(handle, Some(bytes), Some(&mut bytes_written), None) };

            match result {
                Ok(()) => {
                    self.stats.events_sent.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .bytes_sent
                        .fetch_add(bytes.len() as u64, Ordering::Relaxed);
                    debug!("Sent {} bytes to pipe", bytes_written);
                    Ok(())
                }
                Err(e) => {
                    self.stats.send_errors.fetch_add(1, Ordering::Relaxed);
                    warn!("Failed to write to pipe: {}", e);
                    // Mark as disconnected for reconnection
                    self.connected.store(false, Ordering::SeqCst);
                    self.pipe_handle = None;
                    Ok(()) // Don't fail the whole operation
                }
            }
        } else {
            self.connected.store(false, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Send an event over the pipe (non-Windows stub)
    #[cfg(not(target_os = "windows"))]
    async fn send_event(&mut self, event: &IpcEvent) -> Result<()> {
        debug!("IPC stub - would send event: {:?}", event.event_type);
        Ok(())
    }

    /// Get current time in nanoseconds
    fn current_time_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Get statistics
    pub fn stats(&self) -> &IpcClientStats {
        &self.stats
    }

    /// Disconnect from the pipe
    #[cfg(target_os = "windows")]
    pub fn disconnect(&mut self) {
        use windows::Win32::Foundation::CloseHandle;

        if let Some(handle) = self.pipe_handle.take() {
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
        self.connected.store(false, Ordering::SeqCst);
        info!("Disconnected from named pipe");
    }

    #[cfg(not(target_os = "windows"))]
    pub fn disconnect(&mut self) {
        self.connected.store(false, Ordering::SeqCst);
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// Note: IpcServer is implemented in oisp-capture-windows crate (pipe_server.rs)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = IpcEvent {
            event_type: "connection".to_string(),
            timestamp_ns: 1234567890,
            data: IpcEventData::Connection(ConnectionEventData {
                local_addr: "192.168.1.100".to_string(),
                local_port: 12345,
                remote_addr: "93.184.216.34".to_string(),
                remote_port: 443,
                pid: Some(1234),
                process_name: Some("python.exe".to_string()),
                state: "Established".to_string(),
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("connection"));
        assert!(json.contains("192.168.1.100"));
        assert!(json.contains("python.exe"));
    }

    #[test]
    fn test_ssl_event_serialization() {
        use base64::prelude::*;

        let event = IpcEvent {
            event_type: "ssl_write".to_string(),
            timestamp_ns: 1234567890,
            data: IpcEventData::SslData(SslDataEvent {
                id: "test-id".to_string(),
                direction: "write".to_string(),
                pid: 1234,
                remote_host: "api.openai.com".to_string(),
                remote_port: 443,
                data: BASE64_STANDARD.encode(b"Hello World"),
                metadata: ProcessMetadata {
                    comm: "python.exe".to_string(),
                    exe: r"C:\Python311\python.exe".to_string(),
                    uid: 0,
                },
            }),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ssl_write"));
        assert!(json.contains("api.openai.com"));
    }
}
