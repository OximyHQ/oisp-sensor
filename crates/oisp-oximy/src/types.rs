//! Shared types for Oximy cloud communication

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device information sent during registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Hostname
    pub hostname: String,

    /// Operating system
    pub os: String,

    /// OS version
    pub os_version: String,

    /// Architecture (x86_64, aarch64, etc.)
    pub arch: String,

    /// Sensor version
    pub sensor_version: String,

    /// Number of CPUs
    pub cpu_count: u32,

    /// Total memory in bytes
    pub memory_bytes: u64,

    /// Optional device name (user-friendly)
    pub name: Option<String>,

    /// Optional tags for grouping
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            hostname: hostname::get()
                .map(|h: std::ffi::OsString| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            os: std::env::consts::OS.to_string(),
            os_version: os_version(),
            arch: std::env::consts::ARCH.to_string(),
            sensor_version: env!("CARGO_PKG_VERSION").to_string(),
            cpu_count: num_cpus(),
            memory_bytes: total_memory(),
            name: None,
            tags: vec![],
        }
    }
}

/// Registration response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Registered device info
    pub device: RegisteredDevice,

    /// Device credentials
    pub credentials: DeviceCredentials,
}

/// Registered device from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    /// Device ID
    pub id: String,

    /// Organization ID
    pub organization_id: String,

    /// Optional workspace ID
    pub workspace_id: Option<String>,

    /// Device name
    pub name: String,

    /// Device status
    pub status: String,
}

/// Device credentials returned after registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCredentials {
    /// Device token for authentication
    pub device_token: String,

    /// Token expiry time
    pub expires_at: DateTime<Utc>,
}

/// Stored credentials (includes device info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// Device ID
    pub device_id: String,

    /// Device token
    pub device_token: String,

    /// Token expiry
    pub token_expires_at: DateTime<Utc>,

    /// Organization ID
    pub organization_id: String,

    /// Workspace ID (optional)
    pub workspace_id: Option<String>,

    /// API endpoint used
    pub api_endpoint: String,

    /// Stream endpoint used
    pub stream_endpoint: String,

    /// When credentials were stored
    pub created_at: DateTime<Utc>,
}

impl Credentials {
    /// Create from registration response
    pub fn from_registration(
        response: RegistrationResponse,
        api_endpoint: &str,
        stream_endpoint: &str,
    ) -> Self {
        Self {
            device_id: response.device.id,
            device_token: response.credentials.device_token,
            token_expires_at: response.credentials.expires_at,
            organization_id: response.device.organization_id,
            workspace_id: response.device.workspace_id,
            api_endpoint: api_endpoint.to_string(),
            stream_endpoint: stream_endpoint.to_string(),
            created_at: Utc::now(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.token_expires_at
    }

    /// Check if token expires soon (within given duration)
    pub fn expires_soon(&self, within: chrono::Duration) -> bool {
        Utc::now() + within >= self.token_expires_at
    }
}

/// Heartbeat request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    /// Current sensor status
    pub status: SensorStatus,

    /// Sensor statistics
    pub stats: SensorStats,
}

/// Heartbeat response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    /// Acknowledged
    pub ok: bool,

    /// Server timestamp
    pub timestamp: DateTime<Utc>,

    /// Optional commands to execute
    #[serde(default)]
    pub commands: Vec<ServerCommand>,

    /// Optional policy version available
    pub policy_version: Option<String>,
}

/// Sensor status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensorStatus {
    /// Sensor is active and capturing
    Active,

    /// Sensor is paused
    Paused,

    /// Sensor is starting up
    Starting,

    /// Sensor is shutting down
    Stopping,

    /// Sensor encountered an error
    Error,
}

/// Sensor statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensorStats {
    /// Sensor version
    pub sensor_version: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Total events captured
    pub events_captured: u64,

    /// Total events exported
    pub events_exported: u64,

    /// Events currently queued
    pub events_queued: u64,

    /// Current policy version (if any)
    pub policy_version: Option<String>,

    /// Memory usage in MB
    pub memory_mb: u32,

    /// CPU usage percentage
    pub cpu_percent: f32,
}

/// Commands from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerCommand {
    /// Rotate device token
    RotateToken,

    /// Fetch new policies
    FetchPolicies,

    /// Restart sensor
    Restart,

    /// Update sensor
    Update { version: String },
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Optional details
    pub details: Option<serde_json::Value>,
}

// Helper functions

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("VERSION_ID="))
                    .map(|l| {
                        l.trim_start_matches("VERSION_ID=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        "unknown".to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "unknown".to_string()
    }
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|p| p.get() as u32)
        .unwrap_or(1)
}

fn total_memory() -> u64 {
    // Simple fallback - could use sysinfo crate for accurate values
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_default() {
        let info = DeviceInfo::default();
        assert!(!info.hostname.is_empty());
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_credentials_expiry() {
        let mut creds = Credentials {
            device_id: "dev_123".to_string(),
            device_token: "tok_xxx".to_string(),
            token_expires_at: Utc::now() + chrono::Duration::hours(24),
            organization_id: "org_123".to_string(),
            workspace_id: None,
            api_endpoint: "https://api.oximy.com".to_string(),
            stream_endpoint: "wss://stream.oximy.com".to_string(),
            created_at: Utc::now(),
        };

        assert!(!creds.is_expired());
        assert!(!creds.expires_soon(chrono::Duration::hours(1)));
        assert!(creds.expires_soon(chrono::Duration::hours(25)));

        // Make it expired
        creds.token_expires_at = Utc::now() - chrono::Duration::hours(1);
        assert!(creds.is_expired());
    }
}
