<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-light.svg">
  <img alt="OISP Sensor - Universal AI Activity Observability" src="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-light.svg" width="100%">
</picture>

<div align="center">

# OISP Sensor

**See every AI interaction on your machine. Zero instrumentation.**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/oximyHQ/oisp-sensor/ci.yml?branch=main)](https://github.com/oximyHQ/oisp-sensor/actions)
[![Release](https://img.shields.io/github/v/release/oximyHQ/oisp-sensor)](https://github.com/oximyHQ/oisp-sensor/releases)

[Quick Start](#quick-start) · [How It Works](#how-it-works) · [What It Captures](#what-it-captures) · [Installation](#installation) · [Documentation](#documentation)

</div>

---

## The Problem

You're running AI tools everywhere - ChatGPT in the browser, Cursor on your desktop, Claude CLI in the terminal, custom agents on your servers. But you have no idea:

- **What AI tools are active** on this machine?
- **What data is being sent** to AI providers?
- **What actions do agents take** after getting responses?
- **Which account** (personal vs corporate) is being used?

Traditional observability requires instrumenting every app. That doesn't scale.

## The Solution

OISP Sensor observes at the **system boundary** - below applications, above the network. It captures all AI activity without modifying any code.

```
┌─────────────────────────────────────────────────────────────────┐
│  Your Apps: Cursor, ChatGPT, Claude CLI, Python scripts, ...   │
├─────────────────────────────────────────────────────────────────┤
│                    OISP Sensor (this tool)                      │
│         Captures everything at the system boundary              │
├─────────────────────────────────────────────────────────────────┤
│                     Operating System                            │
└─────────────────────────────────────────────────────────────────┘
```

**One binary. Zero config. Immediate visibility.**

---

## Quick Start

### Linux (Full Capture)

```bash
# Install
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh

# Run with TUI
sudo oisp-sensor

# Or with web UI at localhost:7777
sudo oisp-sensor --web
```

### macOS

```bash
# Install via Homebrew
brew install oximy/tap/oisp-sensor

# Run (metadata capture, full capture requires System Extension)
sudo oisp-sensor
```

### Windows

```powershell
# Install via winget
winget install Oximy.OISPSensor

# Run (metadata capture, full capture requires service installation)
oisp-sensor.exe
```

### Docker (Linux)

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro -v /usr:/usr:ro -v /lib:/lib:ro \
  ghcr.io/oximyhq/oisp-sensor:latest
```

---

## What You'll See

### Terminal UI (TUI)

```
┌─ OISP Sensor ─────────────────────────────────────── v0.1.0 ────┐
│                                                                  │
│  AI ACTIVITY (last 5 min)                                       │
│  ───────────────────────────────────────────────────────────────│
│  14:32:15  ai.request   OpenAI gpt-4o      cursor    [FULL]     │
│  14:32:16  ai.response  OpenAI gpt-4o      cursor    [FULL]     │
│  14:32:16  tool_call    write_file         cursor               │
│  14:32:17  file.write   /src/main.rs       cursor               │
│  14:32:18  process.exec cargo build        cursor→sh            │
│                                                                  │
│  PROVIDERS           APPS USING AI         CONFIDENCE           │
│  ─────────────       ─────────────         ──────────           │
│  OpenAI      47      cursor       52       Full: 89%            │
│  Anthropic   12      claude-cli    8       Metadata: 11%        │
│  Ollama       3      python        2                            │
│                                                                  │
│  [t]imeline  [i]nventory  [p]rocess tree  [q]uit               │
└──────────────────────────────────────────────────────────────────┘
```

### Web UI

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/webui-dark.png">
  <img alt="OISP Sensor Web UI" src="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/webui-light.png" width="100%">
</picture>

**Timeline View** - See every AI interaction in chronological order with full context

**Trace View** - Understand how one prompt leads to multiple LLM calls and tool executions

**Inventory View** - Discover all AI tools and providers active on your machine

---

## What It Captures

### Event Types

| Event | Description | Linux | macOS | Windows |
|-------|-------------|:-----:|:-----:|:-------:|
| `ai.request` | AI API request (model, messages, tools) | Full | Meta | Meta |
| `ai.response` | AI API response (content, tool calls) | Full | Meta | Meta |
| `agent.tool_call` | Tool invocation by AI agent | Full | Meta | Meta |
| `agent.tool_result` | Tool execution result | Full | Meta | Meta |
| `process.exec` | Process execution | Full | Full | Full |
| `process.exit` | Process termination | Full | Full | Full |
| `file.read` | File read operation | Full | Full | Full |
| `file.write` | File write operation | Full | Full | Full |
| `network.connect` | Outbound connection | Full | Full | Full |
| `network.dns` | DNS resolution | Full | Full | Full |

**Full** = Complete data including content  
**Meta** = Metadata only (provider, timing, size) - content requires opt-in

### Supported AI Providers

Automatic detection and parsing for:

| Provider | Detection | Content Parsing |
|----------|:---------:|:---------------:|
| OpenAI | Domain + API shape | Full |
| Anthropic | Domain + API shape | Full |
| Google Gemini | Domain + API shape | Full |
| Azure OpenAI | Domain pattern | Full |
| AWS Bedrock | Domain pattern | Full |
| Mistral | Domain | Full |
| Cohere | Domain | Full |
| Groq | Domain | Full |
| Ollama (local) | Port 11434 | Full |
| LM Studio (local) | Port 1234 | Full |
| Any OpenAI-compatible | API shape heuristics | Full |

---

## How It Works

### The Capture Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                         CAPTURE                                 │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐    │
│  │ TLS/SSL   │  │  Process  │  │   File    │  │  Network  │    │
│  │ Boundary  │  │ Lifecycle │  │ Operations│  │  Connect  │    │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘    │
└────────┼──────────────┼──────────────┼──────────────┼───────────┘
         └──────────────┴──────────────┴──────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                          DECODE                                 │
│  HTTP/1.1 Parser → SSE Reassembly → Provider Detection → JSON  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         CORRELATE                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  Temporal   │  │   Process   │  │  Tool Call  │             │
│  │  Matching   │  │    Tree     │  │   Linking   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                 │
│  Build traces: Request → Response → Tool → Action → Result     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                          EXPORT                                 │
│  ┌────────┐  ┌────────┐  ┌───────────┐  ┌───────┐  ┌────────┐  │
│  │ JSONL  │  │  OTLP  │  │ WebSocket │  │ Kafka │  │ Webhook│  │
│  └────────┘  └────────┘  └───────────┘  └───────┘  └────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Platform-Specific Capture

#### Linux (Full Capture via eBPF)

We use eBPF to intercept at the system boundary:

- **SSL/TLS**: uprobes on `SSL_read`/`SSL_write` in OpenSSL, BoringSSL, GnuTLS
- **Processes**: tracepoints on `sched_process_exec`, `sched_process_exit`
- **Files**: tracepoints on `sys_enter_openat`, `sys_enter_read`, `sys_enter_write`
- **Network**: tracepoints on `sys_enter_connect`, `sys_enter_sendto`

This gives us **plaintext** of all HTTPS traffic without MITM proxying.

```bash
# Requires kernel 5.0+ and root/CAP_BPF
sudo oisp-sensor
```

#### macOS (Metadata + Optional Full Capture)

**Default (no special permissions):**
- Process monitoring via `libproc`
- Network connections via `lsof`/`netstat` APIs
- File operations via FSEvents

**Full capture (requires System Extension approval):**
- Endpoint Security Framework for process/file
- Network Extension for traffic inspection
- Requires signed `.pkg` installation

```bash
# Install the System Extension for full capture
sudo oisp-sensor install-extension
# Then approve in System Preferences → Privacy & Security
```

#### Windows (Metadata + Optional Full Capture)

**Default (no elevation):**
- Process monitoring via WMI
- Network connections via Windows APIs
- File operations via ReadDirectoryChangesW

**Full capture (requires service installation):**
- ETW (Event Tracing for Windows) for all events
- Requires Administrator and service registration

```powershell
# Install as Windows service for full capture
oisp-sensor.exe install-service
```

---

## Trace Reconstruction

OISP Sensor doesn't just log events - it understands **agent behavior**.

When an AI agent runs, it typically:
1. Receives a user prompt
2. Calls an LLM
3. LLM responds with a tool call
4. Agent executes the tool
5. Agent sends tool result back to LLM
6. Repeat until done

We reconstruct this entire chain:

```
Trace: tr_01JGXYZ... (12.3s, 15,420 tokens)
├── [User] "Fix the bug in main.rs"
│
├── [LLM Turn 1] → gpt-4o (1.2s)
│   └── tool_call: read_file("/src/main.rs")
│
├── [Tool Execution] read_file (3ms)
│   ├── file.open /src/main.rs
│   └── file.read 4,096 bytes
│
├── [LLM Turn 2] → gpt-4o (2.1s)
│   └── tool_call: edit_file("/src/main.rs", ...)
│
├── [Tool Execution] edit_file (5ms)
│   └── file.write /src/main.rs
│
├── [LLM Turn 3] → gpt-4o (1.1s)
│   └── tool_call: execute("cargo build")
│
├── [Tool Execution] execute (5.2s)
│   ├── process.exec cargo build
│   ├── network.connect crates.io:443
│   └── process.exit (code: 0)
│
└── [LLM Turn 4] → gpt-4o (0.9s)
    └── "Done! The bug has been fixed."
```

---

## Installation

### Linux

#### One-Line Install (Recommended)

```bash
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh
```

This will:
1. Detect your architecture (x86_64, aarch64)
2. Download the latest release
3. Install to `/usr/local/bin`
4. Set up capabilities for non-root operation (optional)

#### Package Managers

```bash
# Debian/Ubuntu
curl -fsSL https://apt.oisp.dev/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/oisp.gpg
echo "deb [signed-by=/etc/apt/keyrings/oisp.gpg] https://apt.oisp.dev stable main" | sudo tee /etc/apt/sources.list.d/oisp.list
sudo apt update && sudo apt install oisp-sensor

# Fedora/RHEL
sudo dnf config-manager --add-repo https://rpm.oisp.dev/oisp.repo
sudo dnf install oisp-sensor

# Arch Linux (AUR)
yay -S oisp-sensor
```

#### Docker

```bash
# Run directly
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  ghcr.io/oximyhq/oisp-sensor:latest

# With persistent logs
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro \
  -v /usr:/usr:ro \
  -v /lib:/lib:ro \
  -v $(pwd)/logs:/var/log/oisp-sensor \
  ghcr.io/oximyhq/oisp-sensor:latest \
  --output /var/log/oisp-sensor/events.jsonl
```

### macOS

#### Homebrew (Recommended)

```bash
brew install oximy/tap/oisp-sensor
```

#### Manual Download

1. Download the `.pkg` from [Releases](https://github.com/oximyHQ/oisp-sensor/releases)
2. Open the `.pkg` file
3. Follow the installer prompts
4. Approve the System Extension in System Preferences → Privacy & Security

### Windows

#### winget (Recommended)

```powershell
winget install Oximy.OISPSensor
```

#### Manual Download

1. Download the `.msi` from [Releases](https://github.com/oximyHQ/oisp-sensor/releases)
2. Run the installer
3. Optionally install the Windows service for full capture:
   ```powershell
   oisp-sensor.exe install-service
   ```

---

## Configuration

OISP Sensor works out of the box with sensible defaults. For customization:

```bash
# Generate default config
oisp-sensor config init

# Config location
# Linux: ~/.config/oisp-sensor/config.toml
# macOS: ~/Library/Application Support/oisp-sensor/config.toml
# Windows: %APPDATA%\oisp-sensor\config.toml
```

### Example Configuration

```toml
# ~/.config/oisp-sensor/config.toml

[capture]
# What to capture
ssl = true
process = true
file = true
network = true

# Filter by process name (empty = all)
process_filter = []  # e.g., ["cursor", "claude", "python"]

[capture.ssl]
# Additional binary paths for SSL library detection
# Useful for NVM Node.js, pyenv Python, etc.
binary_paths = [
    "~/.nvm/versions/node/*/bin/node",
    "~/.pyenv/versions/*/bin/python*",
]

[redaction]
# Privacy mode: "safe" (default), "full", or "minimal"
# - safe: metadata + content hashes, secrets redacted
# - full: complete capture (use with caution)
# - minimal: only process + network flow metadata
mode = "safe"

# Always redact these patterns
[redaction.patterns]
api_keys = true      # sk-*, AKIA*, etc.
emails = true        # user@domain.com
credit_cards = true  # Card number patterns
custom = [
    "INTERNAL_SECRET_\\w+",
    "password\\s*=\\s*[\"'][^\"']+[\"']",
]

[export]
# Where to send events
outputs = ["jsonl", "websocket"]

[export.jsonl]
path = "/var/log/oisp-sensor/events.jsonl"
rotate = "daily"
max_size = "100MB"

[export.otlp]
endpoint = "http://localhost:4317"
headers = { "x-api-key" = "${OTLP_API_KEY}" }

[export.websocket]
port = 7777  # For local UI

[export.kafka]
brokers = ["localhost:9092"]
topic = "oisp-events"

[ui]
# Default UI mode
default = "tui"  # "tui", "web", or "none"

[ui.web]
port = 7777
host = "127.0.0.1"  # Bind to localhost only
```

---

## CLI Reference

```bash
oisp-sensor [OPTIONS] [COMMAND]

Commands:
  record      Start capturing and recording events
  trace       Show real-time event stream
  inventory   Show AI tool inventory summary
  replay      Replay events from a JSONL file
  config      Manage configuration
  install-*   Platform-specific installation helpers

Options:
  -c, --comm <NAME>       Filter by process command name
  -p, --pid <PID>         Filter by process ID
  -o, --output <PATH>     Write events to JSONL file
  --web                   Enable web UI (default: localhost:7777)
  --tui                   Enable terminal UI (default)
  --no-ui                 Disable all UI, just export
  --web-port <PORT>       Web UI port [default: 7777]
  -v, --verbose           Increase verbosity (-v, -vv, -vvv)
  -q, --quiet             Suppress all output except errors
  --config <PATH>         Path to config file
  -h, --help              Print help
  -V, --version           Print version

Examples:
  # Start with TUI
  sudo oisp-sensor

  # Monitor specific app with web UI
  sudo oisp-sensor record --comm cursor --web

  # Export to file
  sudo oisp-sensor record --output /tmp/ai-activity.jsonl --no-ui

  # Send to OTLP endpoint
  sudo oisp-sensor record --export otlp --otlp-endpoint http://localhost:4317

  # Show live inventory
  sudo oisp-sensor inventory --watch
```

---

## Privacy & Security

OISP Sensor is designed with **privacy as a first-class concern**.

### Safe Mode (Default)

By default, OISP Sensor operates in "safe mode":

- **Content is hashed, not stored** - You see that a request happened, its size, and a hash for correlation, but not the actual prompt/response
- **Secrets are redacted** - API keys, tokens, passwords are detected and replaced with `[REDACTED]`
- **Local only** - Nothing leaves the machine unless you configure an external exporter
- **Confidence markers** - Every event clearly indicates what was actually captured vs inferred

### Full Capture Mode

For debugging or authorized monitoring, you can enable full capture:

```bash
sudo oisp-sensor --redaction-mode full
```

This captures complete request/response content. Use responsibly.

### What We Never Do

- **No phone home** - The sensor never contacts Oximy servers
- **No hidden logging** - All captured data goes only where you configure
- **No persistence without consent** - By default, nothing is saved to disk
- **No kernel modifications** - eBPF programs are read-only observers

---

## Event Schema

All events conform to [OISP Spec](https://github.com/oximyHQ/oisp-spec), an open schema for AI observability.

### Event Envelope

Every event has this structure:

```json
{
  "oisp_version": "0.1",
  "event_id": "01JGXYZ123ABC...",
  "event_type": "ai.request",
  "ts": "2025-12-22T14:32:15.123456Z",
  
  "host": {
    "hostname": "dev-laptop",
    "os": "linux",
    "arch": "x86_64"
  },
  
  "actor": {
    "uid": 1000,
    "user": "alice"
  },
  
  "process": {
    "pid": 12345,
    "ppid": 1234,
    "exe": "/usr/bin/cursor",
    "cmdline": "cursor /home/alice/project"
  },
  
  "source": {
    "collector": "oisp-sensor",
    "capture_method": "ebpf_uprobe"
  },
  
  "confidence": {
    "level": "high",
    "completeness": "full"
  },
  
  "data": {
    // Event-type-specific payload
  }
}
```

See [oisp-spec documentation](https://github.com/oximyHQ/oisp-spec) for complete schema details.

---

## Architecture

```
oisp-sensor/
├── crates/
│   ├── oisp-sensor/          # Main binary, CLI
│   ├── oisp-core/            # Event types, plugin traits, pipeline
│   ├── oisp-capture/         # Capture abstraction layer
│   ├── oisp-capture-ebpf/    # Linux eBPF implementation
│   ├── oisp-capture-macos/   # macOS ESF/NE implementation
│   ├── oisp-capture-windows/ # Windows ETW implementation
│   ├── oisp-decode/          # HTTP, SSE, LLM protocol parsing
│   ├── oisp-enrich/          # Process tree, identity enrichment
│   ├── oisp-redact/          # Redaction and safe defaults
│   ├── oisp-correlate/       # Trace building and correlation
│   ├── oisp-export/          # JSONL, OTLP, WebSocket, Kafka
│   ├── oisp-tui/             # Terminal UI (ratatui)
│   └── oisp-web/             # Web UI backend (axum)
├── bpf/                      # eBPF C programs
├── frontend/                 # Web UI frontend (React/TypeScript)
├── installers/
│   ├── linux/                # .deb, .rpm specs
│   ├── macos/                # .pkg scripts, entitlements
│   └── windows/              # .msi WiX config
└── docker/
    └── Dockerfile
```

### Plugin System

OISP Sensor is built on a plugin architecture. Every pipeline stage is a trait:

```rust
// Capture plugins produce raw events
pub trait CapturePlugin: Send + Sync {
    fn start(&mut self, tx: Sender<RawEvent>) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
}

// Decode plugins transform raw bytes → structured events
pub trait DecodePlugin: Send + Sync {
    fn can_decode(&self, raw: &RawEvent) -> bool;
    fn decode(&self, raw: RawEvent) -> Result<OispEvent>;
}

// Export plugins send events somewhere
pub trait ExportPlugin: Send + Sync {
    fn export(&self, event: &OispEvent) -> Result<()>;
}
```

This enables:
- Custom capture sources (vendor audit logs, SDK instrumentation)
- Custom decoders (internal AI APIs, custom protocols)
- Custom exporters (internal systems, custom formats)

---

## Development

### Prerequisites

- **Rust**: 1.83+ (`rustup update stable`)
- **Linux**: kernel 5.0+, clang, llvm, libelf-dev, linux-headers
- **macOS**: Xcode Command Line Tools
- **Windows**: Visual Studio Build Tools, Windows SDK

### Building

```bash
# Clone
git clone https://github.com/oximyHQ/oisp-sensor.git
cd oisp-sensor

# Build all (current platform)
cargo build --release

# Build with specific features
cargo build --release --features linux    # Linux with eBPF
cargo build --release --features macos    # macOS
cargo build --release --features windows  # Windows

# Build eBPF programs (Linux only)
cd bpf && make

# Build web frontend
cd frontend && npm install && npm run build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug sudo ./target/release/oisp-sensor
```

### Project Structure

| Crate | Purpose |
|-------|---------|
| `oisp-core` | Event types, traits, pipeline orchestration |
| `oisp-capture` | Platform-agnostic capture abstraction |
| `oisp-capture-ebpf` | Linux eBPF programs and loader |
| `oisp-capture-macos` | macOS ESF/Network Extension |
| `oisp-capture-windows` | Windows ETW provider |
| `oisp-decode` | HTTP parsing, SSE, provider detection |
| `oisp-enrich` | Process tree, container/k8s metadata |
| `oisp-redact` | Secret detection, PII redaction |
| `oisp-correlate` | Trace building, tool call matching |
| `oisp-export` | JSONL, OTLP, WebSocket, Kafka |
| `oisp-tui` | Terminal UI with ratatui |
| `oisp-web` | Web UI backend with axum |

---

## FAQ

### General

**Q: How is this different from AgentSight?**

[AgentSight](https://github.com/eunomia-bpf/agentsight) is an excellent project that inspired our eBPF approach. Key differences:
- OISP Sensor outputs to a [standardized schema](https://github.com/oximyHQ/oisp-spec) for interoperability
- Multi-OS support (not just Linux)
- Trace reconstruction with tool call correlation
- Plugin architecture for extensibility
- Part of a larger ecosystem (oisp-spec, control plane integration)

**Q: What's the performance overhead?**

Less than 3% CPU overhead on Linux with eBPF. The kernel-space programs are highly optimized and only copy relevant data to userspace.

**Q: Can applications detect they're being monitored?**

Extremely difficult. eBPF operates at the kernel level without modifying application code or memory. There are no injected libraries or modified binaries.

**Q: Does this work with all AI tools?**

Any tool making HTTPS requests to AI providers will be captured. This includes:
- Browser-based (ChatGPT, Claude.ai, Gemini)
- Desktop apps (Cursor, Copilot, Windsurf)
- CLI tools (claude-cli, aider, llm)
- Custom scripts (Python, Node.js, etc.)
- Server-side agents (LangChain, AutoGPT, etc.)

### Technical

**Q: Why do I need sudo/root on Linux?**

eBPF program loading requires `CAP_BPF` and `CAP_SYS_ADMIN` capabilities. You can run without root by setting capabilities:

```bash
sudo setcap cap_bpf,cap_sys_admin+ep /usr/local/bin/oisp-sensor
```

**Q: Why doesn't it capture traffic from my NVM Node.js?**

NVM-installed Node.js statically links OpenSSL instead of using system libraries. Specify the binary path:

```bash
sudo oisp-sensor --binary-path ~/.nvm/versions/node/v20.0.0/bin/node
```

**Q: How do I capture traffic from Go applications?**

Go uses its own TLS implementation. We attach to Go's crypto/tls functions. Most Go apps are supported, but some may require the binary path.

**Q: Can I filter which applications are monitored?**

Yes:

```bash
# Monitor only specific commands
sudo oisp-sensor --comm cursor,claude,python

# Monitor specific PID
sudo oisp-sensor --pid 12345
```

### Privacy

**Q: Is my data sent anywhere?**

No. OISP Sensor is completely local by default. Data only leaves your machine if you explicitly configure an external exporter.

**Q: How do I ensure sensitive data isn't captured?**

Use safe mode (default) or configure redaction patterns:

```toml
[redaction]
mode = "safe"  # Hashes content instead of storing it

[redaction.patterns]
custom = ["SENSITIVE_\\w+", "internal_api_key"]
```

---

## Comparison

| Feature | OISP Sensor | AgentSight | Langfuse | Network DLP |
|---------|:-----------:|:----------:|:--------:|:-----------:|
| Zero instrumentation | Yes | Yes | No | Yes |
| Full prompt/response | Yes | Yes | Yes | Maybe |
| Process attribution | Yes | Yes | No | No |
| Tool call correlation | Yes | Partial | Yes | No |
| Multi-OS | Yes | Linux only | N/A | Yes |
| Open schema | Yes (OISP) | No | No | No |
| Self-hosted | Yes | Yes | Yes | Varies |
| Plugin system | Yes | Yes | No | No |

---

## Roadmap

### v0.1 (Current)
- [x] Linux eBPF capture (SSL, process, file, network)
- [x] HTTP/SSE decoding with provider fingerprinting
- [x] Trace correlation and reconstruction
- [x] TUI and Web UI
- [x] JSONL and WebSocket export

### v0.2
- [ ] OTLP export
- [ ] Stable plugin API
- [ ] Redaction profiles
- [ ] AI tool inventory reports

### v0.3
- [ ] macOS full capture (System Extension)
- [ ] Windows full capture (ETW service)
- [ ] Container/Kubernetes metadata enrichment

### v1.0
- [ ] Production-ready stability
- [ ] Enterprise deployment guides
- [ ] Control plane integration (Oximy)

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where we especially need help:
- Windows ETW implementation
- macOS System Extension
- Additional AI provider parsers
- Documentation and examples

---

## Community

- **GitHub Discussions**: [Ask questions, share ideas](https://github.com/oximyHQ/oisp-sensor/discussions)
- **Twitter/X**: [@oximyHQ](https://twitter.com/oximyHQ)

---

## Acknowledgments

- [AgentSight](https://github.com/eunomia-bpf/agentsight) - Inspiration for the eBPF approach
- [libbpf](https://github.com/libbpf/libbpf) - eBPF library
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [OpenTelemetry](https://opentelemetry.io/) - Observability standards

---

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

<div align="center">

**OISP Sensor is part of the [Oximy](https://oximy.com) open-source ecosystem.**

[Website](https://oisp.dev) · [Documentation](https://docs.oisp.dev) · [GitHub](https://github.com/oximyHQ/oisp-sensor)

</div>
