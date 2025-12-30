//! OISP-Oximy Crate - Oximy Cloud Connector
//!
//! This crate provides cloud connectivity for the OISP Sensor to the Oximy platform.
//!
//! ## Features
//!
//! - **Device Enrollment** - Register devices with Oximy Cloud using API key or enrollment token
//! - **Event Streaming** - Export events to cloud via HTTP batch API
//! - **Offline Queue** - Buffer events when disconnected for later retry
//! - **Policy Sync** - Receive and apply cloud-managed policies
//! - **Heartbeat/Telemetry** - Report sensor health and stats
//!
//! ## Quick Start
//!
//! ```no_run
//! use oisp_oximy::{OximyConfig, CloudClient, Enrollor, OximyExporter, OximyExporterConfig};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = OximyConfig {
//!         enabled: true,
//!         api_key: Some("oxm_live_xxx".to_string()),
//!         ..Default::default()
//!     };
//!
//!     // Create client
//!     let client = Arc::new(CloudClient::new(config));
//!
//!     // Enroll device
//!     let enrollor = Enrollor::new(client.clone());
//!     let credentials = enrollor.register_with_api_key("oxm_live_xxx").await?;
//!     println!("Device registered: {}", credentials.device_id);
//!
//!     // Create exporter
//!     let exporter = OximyExporter::with_client(client.clone())?;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod enrollment;
pub mod error;
pub mod exporter;
pub mod heartbeat;
pub mod offline_queue;
pub mod policy_sync;
pub mod types;

// Re-exports for convenience
pub use client::{CloudClient, HttpClient};
pub use config::OximyConfig;
pub use enrollment::{enroll_device, CredentialStore, Enrollor, FileCredentialStore};
pub use error::{OximyError, OximyResult};
pub use exporter::{ExporterStats, OximyExporter, OximyExporterConfig};
pub use heartbeat::{
    DefaultStatsProvider, HeartbeatConfig, HeartbeatService, HeartbeatStats, StatsProvider,
};
pub use offline_queue::{OfflineQueue, QueueStats};
pub use policy_sync::{CloudPolicy, LocalPolicy, PolicyDocument, PolicySync};
pub use types::{
    Credentials, DeviceInfo, HeartbeatResponse, RegistrationResponse, SensorStats, SensorStatus,
    ServerCommand,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
