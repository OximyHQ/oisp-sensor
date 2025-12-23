---
title: eBPF Capture
description: How OISP Sensor uses eBPF for zero-instrumentation capture
---

import { Aside } from '@astrojs/starlight/components';

OISP Sensor uses **eBPF** (Extended Berkeley Packet Filter) to capture system events without modifying applications. This page explains the technical details.

## Why eBPF?

Traditional approaches to capturing SSL/TLS traffic:

| Approach | Pros | Cons |
|----------|------|------|
| **Proxy (mitmproxy)** | Easy setup | Requires certificate trust, config changes |
| **SDK instrumentation** | Full control | Code changes required, per-language |
| **ptrace** | No code changes | High overhead, breaks debuggers |
| **eBPF** | No code changes, low overhead | Linux only, requires kernel 5.8+ |

eBPF wins because it:
- Runs in kernel space with minimal overhead
- Requires no application changes
- Works with any language that uses OpenSSL/libssl
- Captures plaintext before encryption

## eBPF Program Types

OISP Sensor uses several types of eBPF programs:

### Uprobes (SSL Capture)

Uprobes attach to userspace functions. We attach to OpenSSL's SSL_write and SSL_read:

```
┌──────────────────────────────────────────────────────────┐
│                  Python/Node/Go Process                   │
├──────────────────────────────────────────────────────────┤
│  HTTP Client Library (requests, axios, etc.)              │
├──────────────────────────────────────────────────────────┤
│  SSL Library (libssl.so)                                  │
│  ┌──────────────────┐  ┌──────────────────┐              │
│  │  SSL_write()     │  │  SSL_read()      │              │
│  │   ↑ uprobe       │  │   ↑ uprobe       │              │
│  │   ↓ uretprobe    │  │   ↓ uretprobe    │              │
│  └──────────────────┘  └──────────────────┘              │
├──────────────────────────────────────────────────────────┤
│  Network Stack                                            │
└──────────────────────────────────────────────────────────┘
```

The uprobe captures:
- Function entry: pointer to data buffer, length
- Function return: actual bytes written/read

### Tracepoints (System Events)

Tracepoints are stable kernel attachment points:

| Tracepoint | Captures |
|------------|----------|
| `sched/sched_process_exec` | Process execution (with parent PID) |
| `sched/sched_process_exit` | Process exit (with exit code) |
| `syscalls/sys_enter_openat` | File open operations |
| `syscalls/sys_enter_connect` | Network connection attempts |
| `syscalls/sys_exit_connect` | Connection results |

## Data Structures

### SSL Events

```rust
#[repr(C)]
pub struct SslEvent {
    pub timestamp_ns: u64,     // Kernel monotonic time
    pub pid: u32,              // Process ID (tgid)
    pub tid: u32,              // Thread ID
    pub uid: u32,              // User ID
    pub event_type: u8,        // 1=write, 2=read
    pub data_len: u32,         // Actual data length
    pub captured_len: u32,     // Captured (may be truncated)
    pub comm: [u8; 16],        // Process name
    pub data: [u8; 4096],      // Captured bytes
}
```

### Process Events

```rust
#[repr(C)]
pub struct ProcessExecEvent {
    pub timestamp_ns: u64,
    pub pid: u32,              // New process ID
    pub ppid: u32,             // Parent PID (critical!)
    pub tid: u32,
    pub uid: u32,
    pub comm: [u8; 16],
    pub filename: [u8; 256],   // Executable path
}
```

## Ring Buffers

Events are sent from kernel to userspace via eBPF ring buffers:

```
Kernel Space                    User Space
┌─────────────────┐            ┌─────────────────┐
│  eBPF Program   │            │  OISP Sensor    │
│                 │            │                 │
│  SSL_EVENTS     │───ring────▶│  poll loop      │
│  PROCESS_EVENTS │───ring────▶│  (10ms)         │
│  FILE_EVENTS    │───ring────▶│                 │
│  NETWORK_EVENTS │───ring────▶│                 │
└─────────────────┘            └─────────────────┘
```

Ring buffers are preferred over perf buffers because they:
- Support variable-length events
- Have lower overhead
- Don't lose events on CPU migration

## Filtering

OISP Sensor supports kernel-side filtering to reduce overhead:

### PID Filtering

```
eBPF Maps:
┌─────────────────┐
│ TARGET_PIDS     │  → HashSet of PIDs to trace
├─────────────────┤
│ CONFIG_FLAGS    │  → Bit 0: PID filter enabled
│                 │  → Bit 1: Comm filter enabled
└─────────────────┘
```

When filters are configured:
1. Userspace populates `TARGET_PIDS` map
2. Sets `FLAG_PID_FILTER_ENABLED` in `CONFIG_FLAGS`
3. eBPF program checks filter before processing

### Process Name Filtering

```rust
// eBPF side
fn should_trace_process() -> bool {
    if !is_filter_enabled() { return true; }
    
    let comm = get_current_comm();
    if TARGET_COMMS.contains(&comm) { return true; }
    
    let pid = get_current_pid();
    if TARGET_PIDS.contains(&pid) { return true; }
    
    false
}
```

## Socket Correlation

SSL events don't include destination addresses, so we correlate them with network connections:

```
┌──────────────────────────────────────────────────────────┐
│  sys_enter_connect                                        │
│    → Store (pid, fd) → (addr, port) in CONNECT_ARGS      │
│                                                           │
│  sys_exit_connect                                         │
│    → If success, move to SOCKET_MAP                       │
│    → Emit NetworkConnectEvent                             │
│                                                           │
│  SSL_write/SSL_read                                       │
│    → Lookup (pid, fd) in SOCKET_MAP                       │
│    → Add addr:port to SSL event                           │
└──────────────────────────────────────────────────────────┘
```

Userspace also maintains a socket cache for additional correlation.

## Building eBPF Programs

The eBPF programs are built with [Aya](https://aya-rs.dev/):

```bash
cd ebpf
cargo build --release
```

<Aside type="caution">
eBPF programs require:
- Rust nightly (for `#![no_std]` and BPF target)
- LLVM/Clang for BPF backend
- BTF-enabled kernel for CO-RE (Compile Once, Run Everywhere)
</Aside>

## Capabilities Required

OISP Sensor needs these Linux capabilities:

| Capability | Required For |
|------------|--------------|
| `CAP_BPF` | Loading eBPF programs |
| `CAP_PERFMON` | Accessing perf events, ring buffers |
| `CAP_SYS_ADMIN` | Some eBPF operations (fallback) |
| `CAP_NET_ADMIN` | Network-related eBPF |

Set capabilities instead of running as root:

```bash
sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/local/bin/oisp-sensor
```

## Performance

Typical overhead measurements:

| Metric | Value |
|--------|-------|
| CPU overhead | < 2% |
| Memory (sensor) | ~50 MB |
| Memory (eBPF maps) | ~5 MB |
| Event latency | < 1ms |

The eBPF programs themselves are extremely efficient:
- O(1) map lookups
- Bounded loops (BPF verifier enforced)
- No dynamic memory allocation

## Troubleshooting

### "eBPF not supported"

Check kernel version:
```bash
uname -r  # Needs 5.8+
```

Check BTF availability:
```bash
ls /sys/kernel/btf/vmlinux
```

### "Failed to attach uprobe"

Verify libssl path:
```bash
ldd /usr/bin/curl | grep ssl
# Should show /lib/x86_64-linux-gnu/libssl.so.3 or similar
```

### "Permission denied"

Ensure capabilities are set:
```bash
getcap /usr/local/bin/oisp-sensor
# Should show: cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin=ep
```

Or run with sudo.

## Further Reading

- [Aya Book](https://aya-rs.dev/book/) - Rust eBPF framework
- [BPF Performance Tools](http://www.brendangregg.com/bpf-performance-tools-book.html) - Brendan Gregg
- [Linux eBPF Documentation](https://www.kernel.org/doc/html/latest/bpf/)

