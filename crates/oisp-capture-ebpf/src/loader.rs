//! eBPF program loader

use std::path::Path;
use tracing::warn;

/// Check if eBPF is available on this system
pub fn check_ebpf_available() -> bool {
    // Check for /sys/fs/bpf
    if !Path::new("/sys/fs/bpf").exists() {
        warn!("BPF filesystem not mounted at /sys/fs/bpf");
        return false;
    }

    // Check for BTF support
    if !Path::new("/sys/kernel/btf/vmlinux").exists() {
        warn!("BTF not available - CO-RE programs may not work");
    }

    true
}

/// Check kernel version for eBPF feature support
pub fn get_kernel_version() -> Option<(u32, u32, u32)> {
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    let parts: Vec<&str> = release.trim().split('.').collect();

    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].split('-').next()?.parse().ok()?;
        let patch = parts
            .get(2)
            .and_then(|p| p.split('-').next())
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);

        Some((major, minor, patch))
    } else {
        None
    }
}

/// Check if running as root
pub fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

/// Check capabilities
pub fn has_cap_bpf() -> bool {
    // TODO: Check for CAP_BPF and CAP_PERFMON
    is_root()
}
