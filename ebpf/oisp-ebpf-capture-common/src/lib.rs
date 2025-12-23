#![no_std]

/// Maximum data to capture per SSL event (4KB to stay within eBPF limits)
pub const MAX_DATA_LEN: usize = 4096;

/// Maximum path length for file operations
pub const MAX_PATH_LEN: usize = 256;

/// Maximum command line length for process exec
pub const MAX_CMDLINE_LEN: usize = 256;

/// Command name length
pub const COMM_LEN: usize = 16;

// =============================================================================
// Unified Event Types
// =============================================================================

/// Event type for all captured events
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    /// SSL write (outgoing data)
    SslWrite = 1,
    /// SSL read (incoming data)
    SslRead = 2,
    /// Process execution (fork+exec)
    ProcessExec = 3,
    /// Process exit
    ProcessExit = 4,
    /// File open
    FileOpen = 5,
}

// =============================================================================
// SSL Events (existing)
// =============================================================================

/// SSL event type (legacy, for backwards compatibility)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SslEventType {
    Write = 1,
    Read = 2,
}

/// SSL event sent from kernel to userspace via ring buffer
/// 
/// This struct is shared between eBPF (kernel) and userspace code.
/// It must be:
/// - `#[repr(C)]` for consistent memory layout
/// - `Copy` for eBPF map operations
/// - Fixed size (no dynamic allocation)
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
}

// Implement Pod trait for aya userspace when the "user" feature is enabled
#[cfg(feature = "user")]
unsafe impl aya::Pod for SslEvent {}

// =============================================================================
// Process Events
// =============================================================================

/// Process execution event - captured from sched_process_exec tracepoint
/// 
/// This event is critical for building the process tree - it provides ppid!
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessExecEvent {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Process ID (new process)
    pub pid: u32,
    /// Parent process ID (critical for building process tree!)
    pub ppid: u32,
    /// Thread ID
    pub tid: u32,
    /// User ID
    pub uid: u32,
    /// Process command name (from comm)
    pub comm: [u8; COMM_LEN],
    /// Filename/executable path (from exec)
    pub filename: [u8; MAX_PATH_LEN],
    /// Command line length
    pub cmdline_len: u32,
    /// Padding for alignment
    _pad1: u32,
}

impl ProcessExecEvent {
    /// Create a new zeroed event
    pub const fn zeroed() -> Self {
        Self {
            timestamp_ns: 0,
            pid: 0,
            ppid: 0,
            tid: 0,
            uid: 0,
            comm: [0u8; COMM_LEN],
            filename: [0u8; MAX_PATH_LEN],
            cmdline_len: 0,
            _pad1: 0,
        }
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ProcessExecEvent {}

/// Process exit event - captured from sched_process_exit tracepoint
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessExitEvent {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// Thread ID
    pub tid: u32,
    /// Exit code
    pub exit_code: i32,
    /// Process command name
    pub comm: [u8; COMM_LEN],
}

impl ProcessExitEvent {
    /// Create a new zeroed event
    pub const fn zeroed() -> Self {
        Self {
            timestamp_ns: 0,
            pid: 0,
            ppid: 0,
            tid: 0,
            exit_code: 0,
            comm: [0u8; COMM_LEN],
        }
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ProcessExitEvent {}

// =============================================================================
// File Events
// =============================================================================

/// File open flags (simplified from linux/fcntl.h)
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOpenFlags {
    ReadOnly = 0,
    WriteOnly = 1,
    ReadWrite = 2,
    Create = 0o100,
    Truncate = 0o1000,
    Append = 0o2000,
}

/// File open event - captured from sys_enter_openat tracepoint
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileOpenEvent {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// Thread ID
    pub tid: u32,
    /// User ID
    pub uid: u32,
    /// Open flags (O_RDONLY, O_WRONLY, etc.)
    pub flags: u32,
    /// Mode (permissions for create)
    pub mode: u32,
    /// Process command name
    pub comm: [u8; COMM_LEN],
    /// File path
    pub path: [u8; MAX_PATH_LEN],
}

impl FileOpenEvent {
    /// Create a new zeroed event
    pub const fn zeroed() -> Self {
        Self {
            timestamp_ns: 0,
            pid: 0,
            ppid: 0,
            tid: 0,
            uid: 0,
            flags: 0,
            mode: 0,
            comm: [0u8; COMM_LEN],
            path: [0u8; MAX_PATH_LEN],
        }
    }
    
    /// Check if opened for writing
    pub fn is_write(&self) -> bool {
        (self.flags & 3) != 0 // O_WRONLY or O_RDWR
    }
    
    /// Check if file was created
    pub fn is_create(&self) -> bool {
        (self.flags & 0o100) != 0 // O_CREAT
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for FileOpenEvent {}
