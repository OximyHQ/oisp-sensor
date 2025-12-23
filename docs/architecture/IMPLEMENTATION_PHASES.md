# OISP Sensor Implementation Phases

## Current State Assessment

### What's Done
- [x] Pipeline architecture (capture → decode → enrich → redact → export)
- [x] HTTP decoder for parsing request/response
- [x] Host and process tree enrichers
- [x] Redaction plugin with safe/full/minimal modes
- [x] JSONL and WebSocket exporters
- [x] Web UI foundation
- [x] TUI foundation
- [x] Test event generator for pipeline testing
- [x] Demo mode CLI command
- [x] Docker Compose setup

### What's Missing (eBPF Layer)
- [ ] Actual eBPF programs (currently stubs)
- [ ] eBPF program loader
- [ ] SSL symbol resolution
- [ ] Ring buffer communication
- [ ] Real event capture

---

## Phase 1: Aya Infrastructure Setup (Week 1)

### 1.1 Create eBPF Crate Structure

```bash
# From oisp-sensor root
cargo xtask scaffold-ebpf
```

**New crates:**
```
oisp-sensor/
├── oisp-ebpf/                    # BPF programs (compiled to BPF bytecode)
│   ├── Cargo.toml
│   ├── rust-toolchain.toml       # Nightly for BPF target
│   └── src/
│       └── ssl/
│           └── main.rs           # SSL_read/SSL_write uprobes
│
├── oisp-ebpf-common/             # Shared types
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
│
└── xtask/                        # Build orchestration
    ├── Cargo.toml
    └── src/
        └── main.rs
```

### 1.2 Dependencies

**oisp-ebpf/Cargo.toml:**
```toml
[dependencies]
aya-ebpf = "0.1"
aya-log-ebpf = "0.1"
oisp-ebpf-common = { path = "../oisp-ebpf-common", features = ["kernel"] }

[build-dependencies]
# None needed - pure Rust
```

**oisp-ebpf-common/Cargo.toml:**
```toml
[features]
default = []
kernel = []     # For BPF programs
userspace = []  # For userspace code

[dependencies]
# No dependencies - just plain structs
```

**crates/oisp-capture-ebpf/Cargo.toml:**
```toml
[target.'cfg(target_os = "linux")'.dependencies]
aya = "0.13"
aya-log = "0.2"
oisp-ebpf-common = { path = "../../oisp-ebpf-common", features = ["userspace"] }
```

### 1.3 Install Toolchain

```bash
# Install bpf-linker
cargo install bpf-linker

# Add nightly + BPF target
rustup install nightly
rustup component add rust-src --toolchain nightly
```

---

## Phase 2: Basic SSL Capture (Week 2)

### 2.1 Shared Event Types

**oisp-ebpf-common/src/lib.rs:**
```rust
#![no_std]

/// Maximum captured data per event
pub const MAX_DATA_LEN: usize = 16384;  // 16KB

/// SSL event types
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SslEventType {
    Read = 1,
    Write = 2,
    ReadEx = 3,
    WriteEx = 4,
}

/// Data direction
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ingress = 1,  // Data received (SSL_read)
    Egress = 2,   // Data sent (SSL_write)
}

/// SSL event from kernel to userspace
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SslEvent {
    /// Timestamp in nanoseconds since boot
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Process name (command)
    pub comm: [u8; 16],
    /// Event type (read/write)
    pub event_type: SslEventType,
    /// Direction (ingress/egress)
    pub direction: Direction,
    /// Data length
    pub data_len: u32,
    /// Captured data (first N bytes)
    pub data: [u8; MAX_DATA_LEN],
}
```

### 2.2 SSL Uprobe Program

**oisp-ebpf/src/ssl/main.rs:**
```rust
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{uprobe, uretprobe, map},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use aya_log_ebpf::info;
use oisp_ebpf_common::{SslEvent, SslEventType, Direction, MAX_DATA_LEN};

/// Per-CPU map to store SSL_write entry args
#[map]
static mut SSL_WRITE_ARGS: HashMap<u64, SslWriteArgs> = HashMap::with_max_entries(10240, 0);

/// Ring buffer for events to userspace
#[map]
static mut EVENTS: RingBuf = RingBuf::with_byte_size(16 * 1024 * 1024, 0);  // 16MB

#[repr(C)]
struct SslWriteArgs {
    buf: *const u8,
    len: usize,
}

/// SSL_write entry - capture buffer pointer
#[uprobe]
pub fn ssl_write(ctx: ProbeContext) -> u32 {
    match try_ssl_write(ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_ssl_write(ctx: ProbeContext) -> Result<(), i64> {
    let ssl: *const core::ffi::c_void = ctx.arg(0).ok_or(1)?;
    let buf: *const u8 = ctx.arg(1).ok_or(1)?;
    let len: usize = ctx.arg(2).ok_or(1)?;
    
    let pid_tgid = bpf_get_current_pid_tgid();
    
    // Store args for retrieval in uretprobe
    unsafe {
        SSL_WRITE_ARGS.insert(
            &pid_tgid,
            &SslWriteArgs { buf, len },
            0
        )?;
    }
    
    Ok(())
}

/// SSL_write return - emit event with data
#[uretprobe]
pub fn ssl_write_ret(ctx: RetProbeContext) -> u32 {
    match try_ssl_write_ret(ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_ssl_write_ret(ctx: RetProbeContext) -> Result<(), i64> {
    let ret: i32 = ctx.ret().ok_or(1)?;
    if ret <= 0 {
        return Ok(()); // Error or no data
    }
    
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tid = pid_tgid as u32;
    
    // Retrieve stored args
    let args = unsafe { SSL_WRITE_ARGS.get(&pid_tgid) }.ok_or(1)?;
    
    // Prepare event
    let data_len = core::cmp::min(ret as usize, MAX_DATA_LEN);
    
    // Reserve space in ring buffer
    if let Some(mut entry) = unsafe { EVENTS.reserve::<SslEvent>(0) } {
        let event = entry.as_mut_ptr();
        unsafe {
            (*event).timestamp_ns = bpf_ktime_get_ns();
            (*event).pid = pid;
            (*event).tid = tid;
            bpf_get_current_comm(&mut (*event).comm)?;
            (*event).event_type = SslEventType::Write;
            (*event).direction = Direction::Egress;
            (*event).data_len = data_len as u32;
            
            // Copy data from user buffer
            bpf_probe_read_user_buf(
                (*event).data.as_mut_ptr(),
                &args.buf[..data_len]
            )?;
        }
        entry.submit(0);
    }
    
    // Clean up
    unsafe { SSL_WRITE_ARGS.remove(&pid_tgid)? };
    
    Ok(())
}

// Similar for SSL_read...
```

### 2.3 Userspace Loader

**crates/oisp-capture-ebpf/src/loader.rs (rewritten):**
```rust
use aya::{
    include_bytes_aligned,
    programs::UProbe,
    maps::RingBuf,
    Ebpf,
};
use aya_log::EbpfLogger;
use oisp_ebpf_common::SslEvent;
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

/// Load and manage eBPF programs
pub struct EbpfLoader {
    ebpf: Ebpf,
    running: bool,
}

impl EbpfLoader {
    /// Load eBPF programs from embedded bytecode
    pub fn new() -> anyhow::Result<Self> {
        // Load the BPF object (embedded at compile time)
        let ebpf = Ebpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/release/ssl"
        ))?;
        
        Ok(Self { ebpf, running: false })
    }
    
    /// Attach to SSL libraries
    pub fn attach_ssl(&mut self, lib_path: &Path) -> anyhow::Result<()> {
        info!("Attaching SSL probes to {:?}", lib_path);
        
        // SSL_write entry
        let prog: &mut UProbe = self.ebpf.program_mut("ssl_write")?.try_into()?;
        prog.load()?;
        prog.attach(Some("SSL_write"), 0, lib_path, None)?;
        
        // SSL_write return
        let prog: &mut UProbe = self.ebpf.program_mut("ssl_write_ret")?.try_into()?;
        prog.load()?;
        prog.attach(Some("SSL_write"), 0, lib_path, None)?;
        
        // SSL_read entry
        let prog: &mut UProbe = self.ebpf.program_mut("ssl_read")?.try_into()?;
        prog.load()?;
        prog.attach(Some("SSL_read"), 0, lib_path, None)?;
        
        // SSL_read return
        let prog: &mut UProbe = self.ebpf.program_mut("ssl_read_ret")?.try_into()?;
        prog.load()?;
        prog.attach(Some("SSL_read"), 0, lib_path, None)?;
        
        info!("SSL probes attached successfully");
        Ok(())
    }
    
    /// Start consuming events from ring buffer
    pub async fn start(&mut self, tx: mpsc::Sender<SslEvent>) -> anyhow::Result<()> {
        let ring_buf = RingBuf::try_from(self.ebpf.map_mut("EVENTS")?)?;
        
        self.running = true;
        
        tokio::spawn(async move {
            loop {
                // Poll ring buffer for events
                if let Some(event) = ring_buf.next() {
                    let ssl_event: &SslEvent = unsafe {
                        &*(event.as_ptr() as *const SslEvent)
                    };
                    
                    if tx.send(*ssl_event).await.is_err() {
                        break; // Channel closed
                    }
                }
                
                tokio::task::yield_now().await;
            }
        });
        
        Ok(())
    }
}
```

---

## Phase 3: Process & Network Correlation (Week 3)

### 3.1 Process Tracepoints

**oisp-ebpf/src/process/main.rs:**
```rust
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{tracepoint, map},
    maps::RingBuf,
    programs::TracePointContext,
};
use oisp_ebpf_common::ProcessEvent;

#[map]
static mut PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(4 * 1024 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    // Capture process exec events
    // ...
}

#[tracepoint]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    // Capture process exit events
    // ...
}
```

### 3.2 Network Tracepoints

**oisp-ebpf/src/network/main.rs:**
```rust
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{tracepoint, map},
    maps::RingBuf,
    programs::TracePointContext,
};
use oisp_ebpf_common::NetworkEvent;

#[map]
static mut NETWORK_EVENTS: RingBuf = RingBuf::with_byte_size(4 * 1024 * 1024, 0);

#[tracepoint]
pub fn sys_enter_connect(ctx: TracePointContext) -> u32 {
    // Capture connect syscall
    // ...
}
```

### 3.3 Correlation Engine

**crates/oisp-correlate/src/lib.rs (enhanced):**
```rust
/// Correlate SSL events with process and network context
pub struct Correlator {
    /// PID -> ProcessInfo
    processes: HashMap<u32, ProcessInfo>,
    /// (PID, FD) -> SocketInfo  
    sockets: HashMap<(u32, i32), SocketInfo>,
    /// Connection tracking
    connections: HashMap<ConnectionKey, ConnectionState>,
}

impl Correlator {
    /// Receive SSL event, enrich with process/network context
    pub fn correlate(&mut self, ssl_event: SslEvent) -> CorrelatedEvent {
        let process = self.processes.get(&ssl_event.pid);
        let socket = self.find_socket(ssl_event.pid);
        
        CorrelatedEvent {
            ssl: ssl_event,
            process: process.cloned(),
            network: socket.cloned(),
        }
    }
}
```

---

## Phase 4: End-to-End Integration (Week 4)

### 4.1 Build System (xtask)

**xtask/src/main.rs:**
```rust
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        Some("build-ebpf") => build_ebpf(),
        Some("build-all") => {
            build_ebpf();
            build_userspace();
        }
        _ => println!("Usage: cargo xtask [build-ebpf|build-all]"),
    }
}

fn build_ebpf() {
    let status = Command::new("cargo")
        .current_dir("oisp-ebpf")
        .args([
            "+nightly",
            "build",
            "--release",
            "-Z", "build-std=core",
            "--target", "bpfel-unknown-none",
        ])
        .status()
        .expect("failed to build eBPF programs");
    
    assert!(status.success());
}

fn build_userspace() {
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("failed to build userspace");
    
    assert!(status.success());
}
```

### 4.2 CI/CD Updates

**.github/workflows/build.yml:**
```yaml
jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          toolchain: stable
          components: rust-src
      
      - name: Install nightly (for BPF)
        run: rustup install nightly && rustup component add rust-src --toolchain nightly
      
      - name: Install bpf-linker
        run: cargo install bpf-linker
      
      - name: Build eBPF programs
        run: cargo xtask build-ebpf
      
      - name: Build sensor
        run: cargo build --release
      
      - name: Test
        run: cargo test --release
```

---

## Phase 5: Docker Testing Environment (Ongoing)

### 5.1 Development Dockerfile

**docker/Dockerfile.dev:**
```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libssl-dev \
    pkg-config \
    clang \
    llvm \
    linux-headers-generic \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install nightly + bpf-linker
RUN rustup install nightly \
    && rustup component add rust-src --toolchain nightly \
    && cargo install bpf-linker

WORKDIR /app
```

### 5.2 Test Script

**scripts/test-capture.sh:**
```bash
#!/bin/bash
set -e

echo "Building eBPF programs..."
cargo xtask build-ebpf

echo "Building sensor..."
cargo build --release

echo "Starting sensor (requires root)..."
sudo ./target/release/oisp-sensor record --port 7777 &
SENSOR_PID=$!

sleep 3

echo "Making test request..."
curl -s https://api.openai.com/v1/models \
    -H "Authorization: Bearer sk-test" \
    -H "Content-Type: application/json" || true

echo "Waiting for capture..."
sleep 2

echo "Stopping sensor..."
kill $SENSOR_PID

echo "Done! Check events.jsonl"
```

---

## Success Criteria

### Phase 1 Complete When:
- [ ] `cargo xtask build-ebpf` compiles BPF programs
- [ ] BPF bytecode embedded in userspace binary
- [ ] Loader can load programs (may fail attach without root)

### Phase 2 Complete When:
- [ ] SSL_read/SSL_write uprobes attach to libssl.so
- [ ] Events flow from kernel to userspace via ring buffer
- [ ] Demo mode can simulate + real capture works

### Phase 3 Complete When:
- [ ] Process events captured (exec, exit, fork)
- [ ] Network events captured (connect, accept)
- [ ] Correlation engine links SSL ↔ process ↔ network

### Phase 4 Complete When:
- [ ] Full end-to-end test passes
- [ ] Docker container works with eBPF
- [ ] CI/CD pipeline builds everything

### Production Ready When:
- [ ] <5% CPU overhead
- [ ] <50MB memory usage
- [ ] 0 event drops under normal load
- [ ] Documentation complete
- [ ] Release binaries published

---

## Timeline

| Week | Focus | Deliverable |
|------|-------|-------------|
| 1 | Aya infrastructure | Build system, shared types |
| 2 | SSL capture | Working SSL_read/SSL_write interception |
| 3 | Process/Network | Full event types, correlation |
| 4 | Integration | E2E tests, Docker, CI/CD |
| 5+ | Polish | Performance, docs, release |

