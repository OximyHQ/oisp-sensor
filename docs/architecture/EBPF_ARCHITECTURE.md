# OISP Sensor eBPF Architecture

## Overview

This document outlines the architecture for implementing zero-instrumentation AI activity monitoring using eBPF (extended Berkeley Packet Filter). The goal is to create a category-leading, production-grade implementation that:

1. **Works end-to-end** on Linux systems
2. **Captures ALL AI/LLM traffic** transparently (SSL/TLS decrypted at the OpenSSL layer)
3. **Correlates network events with processes** for complete observability
4. **Maintains minimal overhead** (<5% CPU, <50MB memory)
5. **Is sustainable long-term** with pure Rust implementation

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER SPACE                                      │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         OISP Sensor Pipeline                          │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │  │
│  │  │ Capture │→ │ Decode  │→ │ Enrich  │→ │ Redact  │→ │   Export    │  │  │
│  │  │ Layer   │  │ (HTTP)  │  │ (Proc)  │  │ (PII)   │  │ (WS/JSONL)  │  │  │
│  │  └────┬────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │  │
│  │       │                                                                │  │
│  │  ┌────▼────────────────────────────────────────────────────────────┐  │  │
│  │  │                    eBPF Event Manager                           │  │  │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │  │  │
│  │  │  │ Ring Buffer  │  │  Map Reader  │  │ Symbol Resolution    │  │  │  │
│  │  │  │   Consumer   │  │  (FD→Socket) │  │ (libssl functions)   │  │  │  │
│  │  │  └──────────────┘  └──────────────┘  └──────────────────────┘  │  │  │
│  │  │                                                                  │  │  │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │  │  │
│  │  │  │  Uprobe      │  │  Tracepoint  │  │ Kprobe Loader        │  │  │  │
│  │  │  │  Loader      │  │  Loader      │  │ (optional)           │  │  │  │
│  │  │  └──────────────┘  └──────────────┘  └──────────────────────┘  │  │  │
│  │  └────┬────────────────────┬───────────────────┬─────────────────┘  │  │
│  └───────┼────────────────────┼───────────────────┼────────────────────┘  │
│          │                    │                   │                       │
└──────────┼────────────────────┼───────────────────┼───────────────────────┘
           │                    │                   │
           ▼                    ▼                   ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                             KERNEL SPACE (eBPF)                              │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Ring Buffer (shared)                           │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                    ▲                                         │
│        ┌───────────────────────────┼───────────────────────────┐            │
│        │                           │                           │            │
│  ┌─────┴─────┐              ┌──────┴──────┐             ┌──────┴──────┐     │
│  │  SSL      │              │  Process    │             │  Network    │     │
│  │  Probes   │              │  Probes     │             │  Probes     │     │
│  └─────┬─────┘              └──────┬──────┘             └──────┬──────┘     │
│        │                           │                           │            │
│  ┌─────▼─────┐              ┌──────▼──────┐             ┌──────▼──────┐     │
│  │ Uprobes:  │              │Tracepoints: │             │Tracepoints: │     │
│  │ SSL_read  │              │ sched_exec  │             │ sys_connect │     │
│  │ SSL_write │              │ sched_exit  │             │ sys_accept  │     │
│  │ SSL_read  │              │ sched_fork  │             │ sys_accept4 │     │
│  │ _ex       │              │             │             │             │     │
│  │ SSL_write │              │             │             │             │     │
│  │ _ex       │              │             │             │             │     │
│  └─────┬─────┘              └──────┬──────┘             └──────┬──────┘     │
│        │                           │                           │            │
│        ▼                           ▼                           ▼            │
│  ┌───────────┐              ┌───────────┐               ┌───────────┐       │
│  │ libssl.so │              │  Kernel   │               │  Kernel   │       │
│  │ libcrypto │              │ Scheduler │               │ Net Stack │       │
│  └───────────┘              └───────────┘               └───────────┘       │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## eBPF Program Types Needed

### 1. SSL/TLS Interception (Uprobes)

**Target Functions:**
- `SSL_read` / `SSL_read_ex` - Capture decrypted data received
- `SSL_write` / `SSL_write_ex` - Capture data before encryption

**Approach:**
- Attach uprobe at function entry to capture buffer pointer and length
- Attach uretprobe at function exit to capture actual bytes read/written
- Use per-CPU hash map to correlate entry/exit calls by (pid, tid) key

**Data Captured:**
```rust
struct SslEvent {
    timestamp: u64,
    pid: u32,
    tid: u32,
    comm: [u8; 16],
    is_read: bool,     // true = SSL_read, false = SSL_write
    data_len: u32,
    data: [u8; 4096],  // Captured payload (configurable max)
}
```

### 2. Process Events (Tracepoints)

**Target Tracepoints:**
- `sched:sched_process_exec` - Process started
- `sched:sched_process_exit` - Process ended
- `sched:sched_process_fork` - Process forked

**Data Captured:**
```rust
struct ProcessEvent {
    timestamp: u64,
    pid: u32,
    ppid: u32,
    uid: u32,
    comm: [u8; 16],
    filename: [u8; 256],
    event_type: u8,    // EXEC=1, EXIT=2, FORK=3
}
```

### 3. Network Events (Tracepoints)

**Target Tracepoints:**
- `syscalls:sys_enter_connect` / `sys_exit_connect`
- `syscalls:sys_enter_accept` / `sys_exit_accept`
- `syscalls:sys_enter_accept4` / `sys_exit_accept4`

**Data Captured:**
```rust
struct NetworkEvent {
    timestamp: u64,
    pid: u32,
    tid: u32,
    comm: [u8; 16],
    sockfd: i32,
    addr_family: u16,
    port: u16,
    addr: [u8; 16],    // IPv4 (4 bytes) or IPv6 (16 bytes)
    event_type: u8,    // CONNECT=1, ACCEPT=2
    ret: i32,          // Return value (success/error)
}
```

---

## Library Choice: Aya vs libbpf-rs

### Current State
The codebase uses `libbpf-rs` with `libbpf-cargo` for build-time BPF skeleton generation.

### Recommendation: **Aya**

| Aspect | libbpf-rs | Aya |
|--------|-----------|-----|
| **BPF Program Language** | C (compiled with clang) | Pure Rust |
| **Build Complexity** | Requires clang, libbpf | Rust-only toolchain |
| **Type Safety** | Manual struct definitions | Shared types kernel↔userspace |
| **Debugging** | Harder (C + Rust) | Easier (all Rust) |
| **CO-RE Support** | Yes (mature) | Yes (good) |
| **Community** | Solid | Growing, active |
| **Long-term Maintenance** | Mixed C/Rust codebase | Pure Rust |

**Decision:** Use **Aya** for category-leading, pure-Rust implementation.

**Rationale:**
1. Pure Rust = one language for entire sensor
2. Shared types between kernel and userspace (no manual sync)
3. Better integration with Rust async ecosystem
4. More sustainable for long-term development
5. Active community with excellent documentation

---

## Implementation Plan

### Phase 1: Foundation (eBPF Program Structure)

```
oisp-sensor/
├── oisp-ebpf/                    # NEW: eBPF programs (kernel space)
│   ├── Cargo.toml                # aya-ebpf dependency
│   ├── src/
│   │   ├── main.rs               # Not used (placeholder)
│   │   └── bin/
│   │       ├── ssl.rs            # SSL uprobe programs
│   │       ├── process.rs        # Process tracepoint programs
│   │       └── network.rs        # Network tracepoint programs
│   └── .cargo/config.toml        # BPF target configuration
│
├── oisp-ebpf-common/             # NEW: Shared types (kernel ↔ userspace)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                # SslEvent, ProcessEvent, NetworkEvent
│
└── crates/
    └── oisp-capture-ebpf/        # EXISTING: Userspace loader (enhanced)
        ├── Cargo.toml            # aya dependency
        └── src/
            ├── lib.rs
            ├── ebpf_capture.rs   # Main capture plugin
            ├── loader.rs         # eBPF program loader (rewrite for Aya)
            ├── ssl.rs            # SSL library detection
            ├── process.rs        # Process event handling
            └── network.rs        # Network event handling
```

### Phase 2: SSL Interception Implementation

1. **Symbol Resolution**
   - Parse ELF to find `SSL_read`, `SSL_write` symbols in libssl.so
   - Handle multiple OpenSSL versions (1.1, 3.0)
   - Support BoringSSL (used by Chrome, many apps)

2. **Uprobe Attachment**
   - Attach to all found SSL libraries on the system
   - Handle library load/unload dynamically

3. **Data Capture**
   - Capture up to 16KB per SSL_read/SSL_write call
   - Use ring buffer for zero-copy event delivery
   - Handle fragmented reads/writes

### Phase 3: Process Correlation

1. **Process Tree**
   - Build process tree from sched tracepoints
   - Track parent-child relationships
   - Correlate network events to process lineage

2. **FD Tracking**
   - Track file descriptor → socket mapping
   - Correlate SSL operations to network connections

### Phase 4: Event Correlation Engine

```
SSL Event (PID: 1234, data: "POST /v1/chat/completions...")
      │
      ▼
Network Event (PID: 1234, sockfd: 5, connect to 104.18.6.192:443)
      │
      ▼
Process Event (PID: 1234, comm: "cursor", ppid: 1230)
      │
      ▼
OISP Event {
  id: "01HXY...",
  timestamp: "2024-...",
  request: {
    method: "POST",
    url: "https://api.anthropic.com/v1/messages",
    body: {...}
  },
  process: {
    pid: 1234,
    comm: "cursor",
    cmdline: "/usr/bin/cursor --enable-features=..."
  },
  network: {
    local_addr: "192.168.1.10:54321",
    remote_addr: "104.18.6.192:443",
    protocol: "https"
  }
}
```

---

## AI Provider Detection

The sensor must identify requests to known AI providers:

### Domains to Match
```rust
const AI_PROVIDER_PATTERNS: &[(&str, &str)] = &[
    // OpenAI
    ("api.openai.com", "openai"),
    ("oaidalleapiprodscus.blob.core.windows.net", "openai"),
    
    // Anthropic
    ("api.anthropic.com", "anthropic"),
    
    // Google
    ("generativelanguage.googleapis.com", "google"),
    ("aiplatform.googleapis.com", "google-vertex"),
    
    // Azure OpenAI
    ("openai.azure.com", "azure-openai"),
    
    // AWS Bedrock
    ("bedrock-runtime.*.amazonaws.com", "aws-bedrock"),
    
    // Mistral
    ("api.mistral.ai", "mistral"),
    
    // Cohere
    ("api.cohere.ai", "cohere"),
    
    // Together AI
    ("api.together.xyz", "together"),
    
    // Groq
    ("api.groq.com", "groq"),
    
    // Perplexity
    ("api.perplexity.ai", "perplexity"),
    
    // Local/Self-hosted
    ("localhost:*", "local"),
    ("127.0.0.1:*", "local"),
    ("ollama.*", "ollama"),
];
```

### Request Pattern Detection
Beyond domain matching, detect AI requests by:
- HTTP path patterns (`/v1/chat/completions`, `/v1/messages`, `/v1/embeddings`)
- Content-Type (`application/json`)
- Request body structure (presence of `messages`, `prompt`, `model` fields)

---

## Performance Considerations

### Ring Buffer Sizing
- Default: 16MB ring buffer
- Configurable based on expected traffic
- Multiple ring buffers for different event types if needed

### Sampling
- Optional sampling for high-throughput scenarios
- Per-process filtering (comm name, UID)
- Rate limiting per connection

### Memory Limits
- eBPF maps: ~10MB total
- Userspace buffers: ~50MB max
- Event queue: 10,000 events max before backpressure

---

## Security Considerations

### Required Capabilities
- `CAP_BPF` - Load eBPF programs
- `CAP_PERFMON` - Attach to tracepoints
- `CAP_SYS_PTRACE` - Attach uprobes to other processes

### Or run as root (development/testing)

### Data Sensitivity
- Captured data may contain API keys, tokens
- Redaction layer is CRITICAL
- Configurable redaction modes (full, safe, minimal)

---

## Testing Strategy

### Unit Tests
- eBPF program loading/unloading
- Symbol resolution
- Event parsing

### Integration Tests
- Docker container with known SSL libraries
- Synthetic HTTPS requests
- Verify event capture and correlation

### End-to-End Tests
- Capture real AI requests (e.g., curl to OpenAI API)
- Verify full pipeline from capture to export

---

## Build Requirements

### Development
- Rust 1.83+
- Linux kernel 5.8+ (for ring buffer support)
- bpf-linker (for Aya BPF compilation)

### Runtime
- Linux kernel 5.4+ (minimum)
- BTF enabled (for CO-RE)
- OpenSSL or BoringSSL installed

---

## Next Steps

1. **Create `oisp-ebpf` crate** with Aya BPF programs
2. **Create `oisp-ebpf-common` crate** for shared types
3. **Rewrite `oisp-capture-ebpf` loader** to use Aya
4. **Implement SSL uprobe programs** first (highest value)
5. **Add process/network tracepoints**
6. **Implement correlation engine**
7. **Integration testing in Docker**

---

## References

- [Aya Book](https://aya-rs.dev/book/)
- [eBPF.io](https://ebpf.io/)
- [bcc sslsniff](https://github.com/iovisor/bcc/blob/master/tools/sslsniff.py)
- [poc-rust-https-sniffer](https://github.com/douglasmakey/poc-rust-https-sniffer)
- [OISP Spec](../../../oisp-spec/docs/spec.md)

