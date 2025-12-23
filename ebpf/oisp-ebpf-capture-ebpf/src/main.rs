#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_ktime_get_ns, bpf_probe_read_user_buf, bpf_probe_read_user_str_bytes,
    },
    macros::{map, tracepoint, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext, TracePointContext},
};
use oisp_ebpf_capture_common::{
    FileOpenEvent, ProcessExecEvent, ProcessExitEvent, SslEvent, SslEventType, COMM_LEN,
    MAX_DATA_LEN,
};

// =============================================================================
// Maps
// =============================================================================

/// Ring buffer for sending SSL events to userspace
/// Size: 256KB (262144 bytes) - can hold ~60 events at 4KB each
#[map]
static SSL_EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// Ring buffer for process events (exec, exit)
/// Size: 64KB - process events are smaller
#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(64 * 1024, 0);

/// Ring buffer for file events
/// Size: 64KB
#[map]
static FILE_EVENTS: RingBuf = RingBuf::with_byte_size(64 * 1024, 0);

/// HashMap to store SSL_write entry arguments for correlation with return
/// Key: pid_tgid (u64), Value: (buf_ptr, len)
#[map]
static SSL_WRITE_ARGS: HashMap<u64, SslArgs> = HashMap::with_max_entries(1024, 0);

/// HashMap to store SSL_read entry arguments for correlation with return
/// Key: pid_tgid (u64), Value: (buf_ptr, len)
#[map]
static SSL_READ_ARGS: HashMap<u64, SslArgs> = HashMap::with_max_entries(1024, 0);

/// Arguments stored from uprobe entry for correlation with uretprobe
#[repr(C)]
#[derive(Clone, Copy)]
struct SslArgs {
    /// Buffer pointer
    buf: u64,
    /// Requested length
    len: u64,
}

// =============================================================================
// Helper functions
// =============================================================================

/// Create an SSL event with current process info and captured data
#[inline(always)]
fn create_and_submit_event(event_type: SslEventType, buf_ptr: u64, data_len: usize) {
    let pid_tgid = bpf_get_current_pid_tgid();
    let uid_gid = bpf_get_current_uid_gid();
    
    // Calculate how much data to capture (capped at MAX_DATA_LEN)
    let captured_len = if data_len > MAX_DATA_LEN { MAX_DATA_LEN } else { data_len };
    
    // Reserve space in ring buffer
    if let Some(mut entry) = SSL_EVENTS.reserve::<SslEvent>(0) {
        let event = unsafe { entry.as_mut_ptr().as_mut().unwrap() };
        
        event.timestamp_ns = unsafe { bpf_ktime_get_ns() };
        event.pid = (pid_tgid >> 32) as u32;
        event.tid = pid_tgid as u32;
        event.uid = uid_gid as u32;
        event.event_type = event_type;
        event.data_len = data_len as u32;
        event.captured_len = captured_len as u32;
        
        // Get process command name
        if let Ok(comm) = bpf_get_current_comm() {
            event.comm = comm;
        }
        
        // Read user buffer data if we have a valid pointer and length
        if buf_ptr != 0 && captured_len > 0 {
            // Zero out the data buffer first
            event.data = [0u8; MAX_DATA_LEN];
            
            // Read from user space - use slice of exact captured_len
            // The eBPF verifier needs us to bound the length
            if captured_len <= MAX_DATA_LEN {
                let _ = unsafe {
                    bpf_probe_read_user_buf(
                        buf_ptr as *const u8,
                        &mut event.data[..captured_len],
                    )
                };
            }
        }
        
        entry.submit(0);
    }
}

// =============================================================================
// SSL_write probes
// =============================================================================

/// SSL_write uprobe - captures entry arguments
/// int SSL_write(SSL *ssl, const void *buf, int num);
#[uprobe]
pub fn ssl_write(ctx: ProbeContext) -> u32 {
    match try_ssl_write_entry(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_ssl_write_entry(ctx: &ProbeContext) -> Result<(), i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    
    // Get arguments: SSL_write(ssl, buf, num)
    // arg0 = ssl (we don't need it)
    // arg1 = buf pointer (const void*)
    // arg2 = num (int - length to write)
    let buf: u64 = ctx.arg::<u64>(1).ok_or(1i64)?;
    let len: u64 = ctx.arg::<u64>(2).ok_or(1i64)?;
    
    // Store arguments for correlation with return probe
    let args = SslArgs { buf, len };
    SSL_WRITE_ARGS.insert(&pid_tgid, &args, 0).map_err(|_| 1i64)?;
    
    Ok(())
}

/// SSL_write uretprobe - captures data on successful return
#[uretprobe]
pub fn ssl_write_ret(ctx: RetProbeContext) -> u32 {
    match try_ssl_write_ret(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_ssl_write_ret(ctx: &RetProbeContext) -> Result<(), i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    
    // Get return value (bytes written, or <= 0 on error)
    // RetProbeContext::ret() returns the value directly
    let ret: i64 = ctx.ret::<i64>();
    
    // Remove stored args
    let args = unsafe { SSL_WRITE_ARGS.get(&pid_tgid).ok_or(1i64)? };
    let buf = args.buf;
    
    // Clean up the args map
    let _ = SSL_WRITE_ARGS.remove(&pid_tgid);
    
    // Only capture if write succeeded (ret > 0)
    if ret > 0 {
        let data_len = ret as usize;
        create_and_submit_event(SslEventType::Write, buf, data_len);
    }
    
    Ok(())
}

// =============================================================================
// SSL_read probes  
// =============================================================================

/// SSL_read uprobe - captures entry arguments
/// int SSL_read(SSL *ssl, void *buf, int num);
#[uprobe]
pub fn ssl_read(ctx: ProbeContext) -> u32 {
    match try_ssl_read_entry(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_ssl_read_entry(ctx: &ProbeContext) -> Result<(), i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    
    // Get arguments: SSL_read(ssl, buf, num)
    // arg0 = ssl (we don't need it)
    // arg1 = buf pointer (void*)
    // arg2 = num (int - max length to read)
    let buf: u64 = ctx.arg::<u64>(1).ok_or(1i64)?;
    let len: u64 = ctx.arg::<u64>(2).ok_or(1i64)?;
    
    // Store arguments for correlation with return probe
    let args = SslArgs { buf, len };
    SSL_READ_ARGS.insert(&pid_tgid, &args, 0).map_err(|_| 1i64)?;
    
    Ok(())
}

/// SSL_read uretprobe - captures data on successful return
#[uretprobe]
pub fn ssl_read_ret(ctx: RetProbeContext) -> u32 {
    match try_ssl_read_ret(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

fn try_ssl_read_ret(ctx: &RetProbeContext) -> Result<(), i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    
    // Get return value (bytes read, or <= 0 on error)
    // RetProbeContext::ret() returns the value directly
    let ret: i64 = ctx.ret::<i64>();
    
    // Get stored args
    let args = unsafe { SSL_READ_ARGS.get(&pid_tgid).ok_or(1i64)? };
    let buf = args.buf;
    
    // Clean up the args map
    let _ = SSL_READ_ARGS.remove(&pid_tgid);
    
    // Only capture if read succeeded (ret > 0)
    if ret > 0 {
        let data_len = ret as usize;
        create_and_submit_event(SslEventType::Read, buf, data_len);
    }
    
    Ok(())
}

// =============================================================================
// Process Exec Tracepoint
// =============================================================================

/// sched_process_exec tracepoint - captures when a process executes a new binary
/// This is critical for building the process tree as it provides ppid!
#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

/// Tracepoint arguments for sched_process_exec
/// See: /sys/kernel/debug/tracing/events/sched/sched_process_exec/format
#[repr(C)]
struct SchedProcessExecArgs {
    /// Common tracepoint fields (we skip these)
    _common_type: u16,
    _common_flags: u8,
    _common_preempt_count: u8,
    _common_pid: i32,
    /// Pointer to filename string
    filename_ptr: u64,
    /// Process ID
    pid: i32,
    /// Old process ID (before exec)
    old_pid: i32,
}

fn try_sched_process_exec(ctx: &TracePointContext) -> Result<(), i64> {
    // Read tracepoint args
    let args: SchedProcessExecArgs =
        unsafe { ctx.read_at(0).map_err(|_| 1i64)? };

    let pid_tgid = bpf_get_current_pid_tgid();
    let uid_gid = bpf_get_current_uid_gid();

    // Get parent PID from task_struct
    // The current task's parent is available via current->real_parent->tgid
    // For simplicity, we use the tgid (thread group leader = process)
    let pid = (pid_tgid >> 32) as u32;
    let tid = pid_tgid as u32;
    let uid = uid_gid as u32;

    // Get ppid - this requires reading from task_struct
    // For now, use old_pid as approximation (before exec, this was the parent context)
    // In practice, we'd read current->real_parent->tgid
    let ppid = if args.old_pid > 0 && args.old_pid != args.pid {
        args.old_pid as u32
    } else {
        1 // fallback to init
    };

    // Reserve space in ring buffer for process exec event
    if let Some(mut entry) = PROCESS_EVENTS.reserve::<ProcessExecEvent>(0) {
        let event = unsafe { entry.as_mut_ptr().as_mut().unwrap() };

        event.timestamp_ns = unsafe { bpf_ktime_get_ns() };
        event.pid = pid;
        event.ppid = ppid;
        event.tid = tid;
        event.uid = uid;

        // Get process command name
        if let Ok(comm) = bpf_get_current_comm() {
            event.comm = comm;
        }

        // Read filename from user space
        if args.filename_ptr != 0 {
            let _ = unsafe {
                bpf_probe_read_user_str_bytes(args.filename_ptr as *const u8, &mut event.filename)
            };
        }

        event.cmdline_len = 0; // TODO: read cmdline if needed

        entry.submit(0);
    }

    Ok(())
}

// =============================================================================
// Process Exit Tracepoint
// =============================================================================

/// sched_process_exit tracepoint - captures when a process exits
#[tracepoint]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    match try_sched_process_exit(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

/// Tracepoint arguments for sched_process_exit
/// See: /sys/kernel/debug/tracing/events/sched/sched_process_exit/format
#[repr(C)]
struct SchedProcessExitArgs {
    /// Common tracepoint fields
    _common_type: u16,
    _common_flags: u8,
    _common_preempt_count: u8,
    _common_pid: i32,
    /// Process comm name
    comm: [u8; COMM_LEN],
    /// Process ID
    pid: i32,
    /// Priority
    prio: i32,
}

fn try_sched_process_exit(ctx: &TracePointContext) -> Result<(), i64> {
    let args: SchedProcessExitArgs =
        unsafe { ctx.read_at(0).map_err(|_| 1i64)? };

    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tid = pid_tgid as u32;

    // Only capture main thread exit (when pid == tid, it's the process leader)
    // This avoids capturing every thread exit
    if pid != tid {
        return Ok(());
    }

    // Reserve space in ring buffer
    if let Some(mut entry) = PROCESS_EVENTS.reserve::<ProcessExitEvent>(0) {
        let event = unsafe { entry.as_mut_ptr().as_mut().unwrap() };

        event.timestamp_ns = unsafe { bpf_ktime_get_ns() };
        event.pid = args.pid as u32;
        event.ppid = 0; // We'd need to read task->real_parent->tgid
        event.tid = tid;
        event.exit_code = 0; // Exit code requires reading from task_struct
        event.comm = args.comm;

        entry.submit(0);
    }

    Ok(())
}

// =============================================================================
// File Open Tracepoint (sys_enter_openat)
// =============================================================================

/// sys_enter_openat tracepoint - captures file open operations
#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    match try_sys_enter_openat(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

/// Tracepoint arguments for sys_enter_openat
/// See: /sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/format
#[repr(C)]
struct SysEnterOpenatArgs {
    /// Common tracepoint fields
    _common_type: u16,
    _common_flags: u8,
    _common_preempt_count: u8,
    _common_pid: i32,
    /// Syscall number
    _syscall_nr: i32,
    /// Padding
    _pad: i32,
    /// Directory file descriptor (AT_FDCWD = -100 for current dir)
    dfd: i64,
    /// Filename pointer
    filename_ptr: u64,
    /// Flags (O_RDONLY, O_WRONLY, etc.)
    flags: i64,
    /// Mode (for O_CREAT)
    mode: i64,
}

fn try_sys_enter_openat(ctx: &TracePointContext) -> Result<(), i64> {
    let args: SysEnterOpenatArgs =
        unsafe { ctx.read_at(0).map_err(|_| 1i64)? };

    // Skip if no filename
    if args.filename_ptr == 0 {
        return Ok(());
    }

    let pid_tgid = bpf_get_current_pid_tgid();
    let uid_gid = bpf_get_current_uid_gid();
    let pid = (pid_tgid >> 32) as u32;
    let tid = pid_tgid as u32;
    let uid = uid_gid as u32;

    // Reserve space in ring buffer
    if let Some(mut entry) = FILE_EVENTS.reserve::<FileOpenEvent>(0) {
        let event = unsafe { entry.as_mut_ptr().as_mut().unwrap() };

        event.timestamp_ns = unsafe { bpf_ktime_get_ns() };
        event.pid = pid;
        event.ppid = 0; // Would need to read from task_struct
        event.tid = tid;
        event.uid = uid;
        event.flags = args.flags as u32;
        event.mode = args.mode as u32;

        // Get process command name
        if let Ok(comm) = bpf_get_current_comm() {
            event.comm = comm;
        }

        // Read filename from user space
        let _ = unsafe {
            bpf_probe_read_user_str_bytes(args.filename_ptr as *const u8, &mut event.path)
        };

        // Filter out noise: skip /proc, /sys, /dev paths
        // Check first few bytes of path
        let should_skip = event.path[0] == b'/'
            && ((event.path[1] == b'p' && event.path[2] == b'r' && event.path[3] == b'o')  // /proc
                || (event.path[1] == b's' && event.path[2] == b'y' && event.path[3] == b's')  // /sys
                || (event.path[1] == b'd' && event.path[2] == b'e' && event.path[3] == b'v')); // /dev

        if should_skip {
            // Don't submit, just discard
            entry.discard(0);
        } else {
            entry.submit(0);
        }
    }

    Ok(())
}

// =============================================================================
// Required for eBPF programs
// =============================================================================

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
