//! Linux eBPF capture for OISP Sensor
//!
//! Uses eBPF uprobes for SSL/TLS interception and tracepoints for syscalls.
//!
//! Based on the approach used by AgentSight (https://github.com/eunomia-bpf/agentsight)

#[cfg(target_os = "linux")]
pub mod file;
#[cfg(target_os = "linux")]
pub mod loader;
#[cfg(target_os = "linux")]
pub mod network;
#[cfg(target_os = "linux")]
pub mod process;
#[cfg(target_os = "linux")]
pub mod ssl;

#[cfg(target_os = "linux")]
mod ebpf_capture;
#[cfg(target_os = "linux")]
pub use ebpf_capture::EbpfCapture;

#[cfg(not(target_os = "linux"))]
pub struct EbpfCapture;

#[cfg(not(target_os = "linux"))]
impl Default for EbpfCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl EbpfCapture {
    pub fn new() -> Self {
        Self
    }
}
