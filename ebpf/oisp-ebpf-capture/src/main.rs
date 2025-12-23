use aya::maps::RingBuf;
use aya::programs::{TracePoint, UProbe};
use clap::Parser;
use log::{debug, info, warn};
use oisp_ebpf_capture_common::{
    FileOpenEvent, ProcessExecEvent, ProcessExitEvent, SslEvent, SslEventType,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Parser)]
struct Opt {
    /// Optional: only trace this PID
    #[clap(short, long)]
    pid: Option<u32>,

    /// Path to libssl.so (auto-detected if not specified)
    #[clap(long)]
    libssl: Option<String>,

    /// Enable process tracing (exec/exit)
    #[clap(long, default_value = "true")]
    trace_process: bool,

    /// Enable file tracing (open)
    #[clap(long, default_value = "true")]
    trace_files: bool,
}

/// Find libssl.so on the system
fn find_libssl() -> Option<String> {
    let paths = [
        "/usr/lib/x86_64-linux-gnu/libssl.so.3",
        "/usr/lib/x86_64-linux-gnu/libssl.so.1.1",
        "/usr/lib/aarch64-linux-gnu/libssl.so.3",
        "/usr/lib/aarch64-linux-gnu/libssl.so.1.1",
        "/lib/x86_64-linux-gnu/libssl.so.3",
        "/lib/x86_64-linux-gnu/libssl.so.1.1",
        "/lib/aarch64-linux-gnu/libssl.so.3",
        "/lib/aarch64-linux-gnu/libssl.so.1.1",
        "/usr/lib64/libssl.so.3",
        "/usr/lib64/libssl.so.1.1",
        "/usr/lib/libssl.so.3",
        "/usr/lib/libssl.so.1.1",
        "/usr/lib/libssl.so",
    ];
    
    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

/// Convert SslEventType to string
fn event_type_str(event_type: SslEventType) -> &'static str {
    match event_type {
        SslEventType::Write => "WRITE",
        SslEventType::Read => "READ",
    }
}

/// Format process comm name (null-terminated bytes to string)
fn format_comm(comm: &[u8]) -> String {
    let end = comm.iter().position(|&c| c == 0).unwrap_or(comm.len());
    String::from_utf8_lossy(&comm[..end]).to_string()
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    env_logger::init();

    // Bump the memlock rlimit for older kernels
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {ret}");
    }

    // Load eBPF program
    let mut ebpf = aya::Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/oisp-ebpf-capture"
    )))?;

    // Initialize eBPF logger (simplified - just log errors)
    if let Err(e) = aya_log::EbpfLogger::init(&mut ebpf) {
        warn!("failed to initialize eBPF logger: {e}");
    }

    let Opt {
        pid,
        libssl,
        trace_process,
        trace_files,
    } = opt;

    // Find libssl path
    let libssl_path = libssl.or_else(find_libssl);
    let libssl_target = libssl_path.as_deref().unwrap_or("libssl");

    info!("Attaching to libssl at: {}", libssl_target);

    // Load and attach SSL_write probes
    let ssl_write: &mut UProbe = ebpf.program_mut("ssl_write").unwrap().try_into()?;
    ssl_write.load()?;
    ssl_write.attach("SSL_write", libssl_target, pid, None)?;
    info!("Attached uprobe to SSL_write");

    let ssl_write_ret: &mut UProbe = ebpf.program_mut("ssl_write_ret").unwrap().try_into()?;
    ssl_write_ret.load()?;
    ssl_write_ret.attach("SSL_write", libssl_target, pid, None)?;
    info!("Attached uretprobe to SSL_write");

    // Load and attach SSL_read probes
    let ssl_read: &mut UProbe = ebpf.program_mut("ssl_read").unwrap().try_into()?;
    ssl_read.load()?;
    ssl_read.attach("SSL_read", libssl_target, pid, None)?;
    info!("Attached uprobe to SSL_read");

    let ssl_read_ret: &mut UProbe = ebpf.program_mut("ssl_read_ret").unwrap().try_into()?;
    ssl_read_ret.load()?;
    ssl_read_ret.attach("SSL_read", libssl_target, pid, None)?;
    info!("Attached uretprobe to SSL_read");

    // Load and attach process tracepoints
    if trace_process {
        let proc_exec: &mut TracePoint =
            ebpf.program_mut("sched_process_exec").unwrap().try_into()?;
        proc_exec.load()?;
        proc_exec.attach("sched", "sched_process_exec")?;
        info!("Attached tracepoint sched/sched_process_exec");

        let proc_exit: &mut TracePoint =
            ebpf.program_mut("sched_process_exit").unwrap().try_into()?;
        proc_exit.load()?;
        proc_exit.attach("sched", "sched_process_exit")?;
        info!("Attached tracepoint sched/sched_process_exit");
    }

    // Load and attach file tracepoints
    if trace_files {
        let file_open: &mut TracePoint =
            ebpf.program_mut("sys_enter_openat").unwrap().try_into()?;
        file_open.load()?;
        file_open.attach("syscalls", "sys_enter_openat")?;
        info!("Attached tracepoint syscalls/sys_enter_openat");
    }

    // Get the ring buffer maps - take ownership of all maps first to avoid borrow issues
    let ssl_map = ebpf.take_map("SSL_EVENTS").unwrap();
    let process_map = if trace_process {
        Some(ebpf.take_map("PROCESS_EVENTS").unwrap())
    } else {
        None
    };
    let file_map = if trace_files {
        Some(ebpf.take_map("FILE_EVENTS").unwrap())
    } else {
        None
    };

    // Now create ring buffers from the owned maps
    let mut ssl_ring_buf = RingBuf::try_from(ssl_map)?;
    let mut process_ring_buf = match process_map {
        Some(map) => Some(RingBuf::try_from(map)?),
        None => None,
    };
    let mut file_ring_buf = match file_map {
        Some(map) => Some(RingBuf::try_from(map)?),
        None => None,
    };

    println!();
    println!("===========================================");
    println!("  OISP eBPF Capture - Running");
    println!("===========================================");
    println!("  SSL monitoring: {}", libssl_target);
    println!("  Process tracing: {}", if trace_process { "enabled" } else { "disabled" });
    println!("  File tracing: {}", if trace_files { "enabled" } else { "disabled" });
    if let Some(p) = pid {
        println!("  Filtering PID: {}", p);
    } else {
        println!("  Filtering PID: all processes");
    }
    println!();
    println!("  Ring buffers connected!");
    println!("  Waiting for events...");
    println!();
    println!("  Press Ctrl+C to stop");
    println!("===========================================");
    println!();
    
    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    // Event counters for summary
    let mut read_count: u64 = 0;
    let mut write_count: u64 = 0;
    let mut exec_count: u64 = 0;
    let mut exit_count: u64 = 0;
    let mut file_count: u64 = 0;

    // Poll ring buffers for events
    while running.load(Ordering::SeqCst) {
        // Process SSL events
        while let Some(item) = ssl_ring_buf.next() {
            let data = item.as_ref();
            if data.len() >= std::mem::size_of::<SslEvent>() {
                let event: &SslEvent = unsafe { &*(data.as_ptr() as *const SslEvent) };

                // Skip invalid events
                if event.data_len > 1_000_000 {
                    continue;
                }

                match event.event_type {
                    SslEventType::Read => read_count += 1,
                    SslEventType::Write => write_count += 1,
                }

                let comm = format_comm(&event.comm);
                let captured_len = event.captured_len as usize;
                let data_preview = format_data_preview(&event.data, captured_len);

                println!(
                    "[SSL_{}] pid={} comm={} len={}/{} {}",
                    event_type_str(event.event_type),
                    event.pid,
                    comm,
                    event.captured_len,
                    event.data_len,
                    data_preview,
                );
            }
        }

        // Process process events
        if let Some(ref mut ring_buf) = process_ring_buf {
            while let Some(item) = ring_buf.next() {
                let data = item.as_ref();

                // Try to parse as ProcessExecEvent
                if data.len() >= std::mem::size_of::<ProcessExecEvent>() {
                    let event: &ProcessExecEvent =
                        unsafe { &*(data.as_ptr() as *const ProcessExecEvent) };

                    // Check if it looks like an exec event (has filename)
                    if event.filename[0] != 0 {
                        exec_count += 1;
                        let comm = format_comm(&event.comm);
                        let filename = format_path(&event.filename);

                        println!(
                            "[EXEC] pid={} ppid={} comm={} exe={}",
                            event.pid, event.ppid, comm, filename,
                        );
                        continue;
                    }
                }

                // Try to parse as ProcessExitEvent
                if data.len() >= std::mem::size_of::<ProcessExitEvent>() {
                    let event: &ProcessExitEvent =
                        unsafe { &*(data.as_ptr() as *const ProcessExitEvent) };

                    exit_count += 1;
                    let comm = format_comm(&event.comm);

                    println!(
                        "[EXIT] pid={} ppid={} comm={} code={}",
                        event.pid, event.ppid, comm, event.exit_code,
                    );
                }
            }
        }

        // Process file events
        if let Some(ref mut ring_buf) = file_ring_buf {
            while let Some(item) = ring_buf.next() {
                let data = item.as_ref();
                if data.len() >= std::mem::size_of::<FileOpenEvent>() {
                    let event: &FileOpenEvent =
                        unsafe { &*(data.as_ptr() as *const FileOpenEvent) };

                    file_count += 1;
                    let comm = format_comm(&event.comm);
                    let path = format_path(&event.path);
                    let mode = if event.is_write() { "W" } else { "R" };

                    println!(
                        "[FILE_OPEN] pid={} comm={} mode={} path={}",
                        event.pid, comm, mode, path,
                    );
                }
            }
        }

        // Sleep briefly to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    println!();
    println!("===========================================");
    println!("  Summary");
    println!("===========================================");
    println!("  SSL_read events:   {}", read_count);
    println!("  SSL_write events:  {}", write_count);
    println!("  Process exec:      {}", exec_count);
    println!("  Process exit:      {}", exit_count);
    println!("  File open:         {}", file_count);
    println!(
        "  Total events:      {}",
        read_count + write_count + exec_count + exit_count + file_count
    );
    println!("===========================================");
    println!();
    println!("Exiting...");

    Ok(())
}

/// Format path bytes to string
fn format_path(path: &[u8]) -> String {
    let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    String::from_utf8_lossy(&path[..end]).to_string()
}

/// Format data preview for display
fn format_data_preview(data: &[u8], captured_len: usize) -> String {
    if captured_len == 0 {
        return String::from("(no data)");
    }

    let data_slice = &data[..captured_len.min(200)];
    let printable_count = data_slice
        .iter()
        .filter(|&&b| (0x20..0x7f).contains(&b) || b == b'\n' || b == b'\r')
        .count();

    if printable_count > data_slice.len() * 8 / 10 {
        let s = String::from_utf8_lossy(data_slice);
        let preview: String = s
            .chars()
            .take(100)
            .map(|c| if c == '\n' { ' ' } else { c })
            .collect();
        format!(
            "\"{}{}\"",
            preview,
            if captured_len > 100 { "..." } else { "" }
        )
    } else {
        let hex: String = data_slice
            .iter()
            .take(32)
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        format!("[{}{}]", hex, if captured_len > 32 { " ..." } else { "" })
    }
}
