//! Extended configuration types for Oximy cloud connector
//!
//! This module extends the basic `OximyExportConfig` from oisp-core
//! with additional settings needed for cloud connectivity.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Extended Oximy configuration with all connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OximyConfig {
    /// Enable Oximy cloud export
    pub enabled: bool,

    /// API key (for registration)
    pub api_key: Option<String>,

    /// Enrollment token (alternative to API key, for MDM flow)
    pub enrollment_token: Option<String>,

    /// REST API endpoint
    pub api_endpoint: String,

    /// WebSocket streaming endpoint
    pub stream_endpoint: String,

    /// Device ID (set after enrollment)
    pub device_id: Option<String>,

    /// Event batch size before sending
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,

    /// Offline buffer size (max events to queue)
    pub offline_buffer_size: usize,

    /// Max age for offline events in hours
    pub offline_max_age_hours: u64,

    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,

    /// Enable automatic reconnection
    pub reconnect_enabled: bool,

    /// Max reconnection delay in milliseconds
    pub reconnect_max_delay_ms: u64,

    /// Credential storage path (for file-based storage)
    pub credential_path: Option<String>,
}

impl Default for OximyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            enrollment_token: None,
            api_endpoint: "https://api.oximy.com".to_string(),
            stream_endpoint: "wss://stream.oximy.com".to_string(),
            device_id: None,
            batch_size: 100,
            flush_interval_ms: 5000,
            heartbeat_interval_ms: 30000,
            offline_buffer_size: 100_000,
            offline_max_age_hours: 168, // 7 days
            connect_timeout_ms: 10000,
            reconnect_enabled: true,
            reconnect_max_delay_ms: 30000,
            credential_path: None,
        }
    }
}

impl OximyConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("OISP_OXIMY_ENABLED") {
            config.enabled = val.parse().unwrap_or(false);
        }
        if let Ok(val) = std::env::var("OISP_OXIMY_API_KEY") {
            config.api_key = Some(val);
        }
        if let Ok(val) = std::env::var("OISP_OXIMY_ENROLLMENT_TOKEN") {
            config.enrollment_token = Some(val);
        }
        if let Ok(val) = std::env::var("OISP_OXIMY_API_ENDPOINT") {
            config.api_endpoint = val;
        }
        if let Ok(val) = std::env::var("OISP_OXIMY_STREAM_ENDPOINT") {
            config.stream_endpoint = val;
        }

        config
    }

    /// Convert from basic OximyExportConfig
    pub fn from_export_config(basic: &oisp_core::OximyExportConfig) -> Self {
        Self {
            enabled: basic.enabled,
            api_key: basic.api_key.clone(),
            api_endpoint: basic.endpoint.clone(),
            device_id: basic.device_id.clone(),
            batch_size: basic.batch_size,
            flush_interval_ms: basic.flush_interval_ms,
            ..Default::default()
        }
    }

    /// Get flush interval as Duration
    pub fn flush_interval(&self) -> Duration {
        Duration::from_millis(self.flush_interval_ms)
    }

    /// Get heartbeat interval as Duration
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Get connection timeout as Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }

    /// Get max reconnect delay as Duration
    pub fn reconnect_max_delay(&self) -> Duration {
        Duration::from_millis(self.reconnect_max_delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OximyConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.api_endpoint, "https://api.oximy.com");
        assert_eq!(config.stream_endpoint, "wss://stream.oximy.com");
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.heartbeat_interval_ms, 30000);
    }

    #[test]
    fn test_duration_helpers() {
        let config = OximyConfig::default();
        assert_eq!(config.flush_interval(), Duration::from_millis(5000));
        assert_eq!(config.heartbeat_interval(), Duration::from_secs(30));
        assert_eq!(config.connect_timeout(), Duration::from_secs(10));
    }
}
