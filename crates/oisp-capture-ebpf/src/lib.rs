//! Linux eBPF capture for OISP Sensor
//!
//! Uses the battle-tested sslsniff binary (libbpf-based) for SSL/TLS interception.
//! The sslsniff binary is embedded in the sensor and extracted at runtime.
//!
//! Based on [AgentSight's sslsniff](https://github.com/eunomia-bpf/agentsight).

#[cfg(target_os = "linux")]
mod sslsniff_runner;

#[cfg(target_os = "linux")]
pub use sslsniff_runner::{SslsniffCapture, SslsniffConfig};

// Re-export as the main capture type for backwards compatibility
#[cfg(target_os = "linux")]
pub type EbpfCapture = SslsniffCapture;

#[cfg(target_os = "linux")]
pub type EbpfCaptureConfig = SslsniffConfig;

// Stub for non-Linux platforms
#[cfg(not(target_os = "linux"))]
pub struct EbpfCapture;

#[cfg(not(target_os = "linux"))]
impl Default for EbpfCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "linux"))]
impl EbpfCapture {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Default)]
pub struct EbpfCaptureConfig {
    pub ssl: bool,
    pub process: bool,
    pub file: bool,
    pub network: bool,
    pub ssl_binary_paths: Vec<String>,
    pub comm_filter: Vec<String>,
    pub pid_filter: Option<u32>,
    pub ebpf_bytecode_path: Option<String>,
}
