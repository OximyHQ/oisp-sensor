//! Shared types for eBPF capture
//!
//! These types mirror the kernel-side eBPF structures and must be kept in sync.

/// Maximum data to capture per SSL event (4KB to stay within eBPF limits)
pub const MAX_DATA_LEN: usize = 4096;

/// Command name length (matches TASK_COMM_LEN in kernel)
pub const COMM_LEN: usize = 16;

/// SSL event type
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SslEventType {
    Write = 1,
    Read = 2,
}

/// SSL event sent from kernel to userspace via ring buffer
///
/// This struct must match the eBPF-side structure exactly:
/// - `#[repr(C)]` for consistent memory layout
/// - Fixed size (no dynamic allocation)
/// - Alignment matches kernel expectations
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SslEvent {
    /// Timestamp in nanoseconds (from bpf_ktime_get_ns)
    pub timestamp_ns: u64,
    /// Process ID (tgid)
    pub pid: u32,
    /// Thread ID (pid in kernel terms)
    pub tid: u32,
    /// User ID
    pub uid: u32,
    /// Event type (read/write)
    pub event_type: SslEventType,
    /// Padding for alignment
    _pad1: [u8; 3],
    /// Actual data length (may be > captured_len if data was truncated)
    pub data_len: u32,
    /// Captured data length (up to MAX_DATA_LEN)
    pub captured_len: u32,
    /// Process command name (null-terminated)
    pub comm: [u8; COMM_LEN],
    /// Captured SSL data (plaintext)
    pub data: [u8; MAX_DATA_LEN],
}

impl SslEvent {
    /// Create a new zeroed event
    pub const fn zeroed() -> Self {
        Self {
            timestamp_ns: 0,
            pid: 0,
            tid: 0,
            uid: 0,
            event_type: SslEventType::Write,
            _pad1: [0; 3],
            data_len: 0,
            captured_len: 0,
            comm: [0u8; COMM_LEN],
            data: [0u8; MAX_DATA_LEN],
        }
    }

    /// Get process command name as string
    pub fn comm_str(&self) -> String {
        let end = self.comm.iter().position(|&c| c == 0).unwrap_or(COMM_LEN);
        String::from_utf8_lossy(&self.comm[..end]).to_string()
    }

    /// Get captured data as slice
    pub fn captured_data(&self) -> &[u8] {
        let len = (self.captured_len as usize).min(MAX_DATA_LEN);
        &self.data[..len]
    }
}

impl std::fmt::Debug for SslEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_preview = if self.captured_len > 0 {
            let data = self.captured_data();
            if data.iter().filter(|&&b| b >= 0x20 && b < 0x7f).count() > data.len() * 8 / 10 {
                // Mostly printable
                let s = String::from_utf8_lossy(&data[..data.len().min(100)]);
                format!("\"{}...\"", s)
            } else {
                // Binary
                format!("[{} bytes]", data.len())
            }
        } else {
            "(empty)".to_string()
        };

        f.debug_struct("SslEvent")
            .field("timestamp_ns", &self.timestamp_ns)
            .field("pid", &self.pid)
            .field("tid", &self.tid)
            .field("uid", &self.uid)
            .field("event_type", &self.event_type)
            .field("data_len", &self.data_len)
            .field("captured_len", &self.captured_len)
            .field("comm", &self.comm_str())
            .field("data", &data_preview)
            .finish()
    }
}

// Implement Pod trait for aya ring buffer
#[cfg(target_os = "linux")]
unsafe impl aya::Pod for SslEvent {}

// =============================================================================
// Network Events
// =============================================================================

/// Network connect event from eBPF tracepoint
///
/// This struct must match the eBPF-side structure exactly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkConnectEvent {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// User ID
    pub uid: u32,
    /// Socket file descriptor
    pub fd: i32,
    /// Address family (AF_INET=2, AF_INET6=10)
    pub family: u16,
    /// Padding for alignment
    _pad1: u16,
    /// Return value from connect() (0 = success, -errno on error)
    pub ret: i32,
    /// Destination port (host byte order)
    pub port: u16,
    /// Padding for alignment
    _pad2: u16,
    /// IPv4 address (for AF_INET) - network byte order
    pub addr_v4: u32,
    /// IPv6 address (for AF_INET6) - network byte order
    pub addr_v6: [u8; 16],
    /// Process command name
    pub comm: [u8; COMM_LEN],
}

impl NetworkConnectEvent {
    /// Create a new zeroed event
    pub const fn zeroed() -> Self {
        Self {
            timestamp_ns: 0,
            pid: 0,
            tid: 0,
            uid: 0,
            fd: 0,
            family: 0,
            _pad1: 0,
            ret: 0,
            port: 0,
            _pad2: 0,
            addr_v4: 0,
            addr_v6: [0u8; 16],
            comm: [0u8; COMM_LEN],
        }
    }

    /// Get process command name as string
    pub fn comm_str(&self) -> String {
        let end = self.comm.iter().position(|&c| c == 0).unwrap_or(COMM_LEN);
        String::from_utf8_lossy(&self.comm[..end]).to_string()
    }

    /// Check if connection succeeded
    pub fn is_success(&self) -> bool {
        // 0 = success, -EINPROGRESS (-115) = non-blocking in progress
        self.ret == 0 || self.ret == -115
    }

    /// Check if this is an IPv4 connection
    pub fn is_ipv4(&self) -> bool {
        self.family == 2 // AF_INET
    }

    /// Check if this is an IPv6 connection
    pub fn is_ipv6(&self) -> bool {
        self.family == 10 // AF_INET6
    }

    /// Get IPv4 address as string
    pub fn ipv4_str(&self) -> Option<String> {
        if self.is_ipv4() {
            let bytes = self.addr_v4.to_be_bytes();
            Some(format!(
                "{}.{}.{}.{}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            ))
        } else {
            None
        }
    }

    /// Get IPv6 address as string
    pub fn ipv6_str(&self) -> Option<String> {
        if self.is_ipv6() {
            use std::net::Ipv6Addr;
            let addr = Ipv6Addr::from(self.addr_v6);
            Some(addr.to_string())
        } else {
            None
        }
    }

    /// Get address as string (works for both IPv4 and IPv6)
    pub fn addr_str(&self) -> String {
        if self.is_ipv4() {
            self.ipv4_str().unwrap_or_default()
        } else if self.is_ipv6() {
            self.ipv6_str().unwrap_or_default()
        } else {
            "unknown".to_string()
        }
    }
}

impl std::fmt::Debug for NetworkConnectEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = self.addr_str();
        let status = if self.is_success() {
            "success"
        } else {
            "failed"
        };

        f.debug_struct("NetworkConnectEvent")
            .field("timestamp_ns", &self.timestamp_ns)
            .field("pid", &self.pid)
            .field("tid", &self.tid)
            .field("uid", &self.uid)
            .field("fd", &self.fd)
            .field("family", &self.family)
            .field("ret", &self.ret)
            .field("status", &status)
            .field("addr", &format!("{}:{}", addr, self.port))
            .field("comm", &self.comm_str())
            .finish()
    }
}

#[cfg(target_os = "linux")]
unsafe impl aya::Pod for NetworkConnectEvent {}
