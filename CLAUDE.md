# OISP Sensor

Cross-platform AI traffic capture using eBPF (Linux), Network Extension (macOS), and WinDivert (Windows).

## Quick Reference

```bash
# Build
cargo build --release              # Release build
cargo build                        # Debug build (faster)

# Run (requires root/admin)
sudo ./target/release/oisp-sensor  # Linux
sudo ./target/release/oisp-sensor  # macOS (after extension approval)

# Development
cargo check                        # Fast type check
cargo clippy                       # Lint
cargo fmt                          # Format
cargo test                         # Run tests

# Platform-specific
cd macos && xcodebuild            # Build macOS app
cd windows && dotnet build        # Build Windows app
```

## Architecture

```
crates/
├── oisp-sensor/        # Main binary - CLI, config, orchestration
├── oisp-core/          # Core types, event envelope, provider detection
├── oisp-capture/       # Platform abstraction for packet capture
├── oisp-capture-ebpf/  # Linux: eBPF uprobes on OpenSSL/GnuTLS
├── oisp-capture-macos/ # macOS: Network Extension binding
├── oisp-capture-windows/ # Windows: WinDivert binding
├── oisp-decode/        # HTTP parsing, AI provider protocol decoding
├── oisp-export/        # Output: JSONL, WebSocket, OTLP, Kafka
├── oisp-tui/           # Terminal UI (Ratatui)
├── oisp-web/           # Embedded web dashboard (Axum)
├── oisp-redirector/    # Traffic redirection utilities
└── oisp-oximy/         # Oximy cloud integration

macos/                  # Swift macOS app + Network Extension
windows/                # C# WPF app + WinDivert driver
frontend/               # Web dashboard (Next.js, embedded in binary)
bpf/                    # eBPF C source files
```

## Key Concepts

**Capture methods by platform:**
- Linux: eBPF uprobes intercept `SSL_read`/`SSL_write` for plaintext before encryption
- macOS: Network Extension with dynamic CA for TLS inspection
- Windows: WinDivert packet capture with MITM proxy

**Event flow:**
1. Capture layer extracts raw HTTP from TLS
2. Decode layer parses HTTP, identifies AI provider
3. Core layer wraps in OISP envelope with metadata
4. Export layer sends to configured destinations

## Code Style

- Rust 2021 edition, MSRV 1.83
- Use `thiserror` for error types, `anyhow` for error propagation
- Use `tracing` for logging (not `log`)
- Async with Tokio
- Config via `clap` derive + `toml` files

## Common Tasks

**Add AI provider support:**
1. Add detection pattern in `oisp-decode/src/providers/`
2. Add response parser if non-standard format
3. Update `Provider` enum in `oisp-core`

**Add export destination:**
1. Create exporter in `oisp-export/src/`
2. Implement `Exporter` trait
3. Add config option in `oisp-sensor`

**Modify eBPF probes:**
1. Edit `bpf/*.bpf.c`
2. Run `cargo build` (uses aya-build)
3. Test on Linux with `sudo`

## Configuration

```toml
# config.toml
[capture]
interface = "eth0"

[export.jsonl]
path = "/var/log/oisp/events.jsonl"

[export.websocket]
url = "ws://localhost:3001"
```

## Platform Requirements

- **Linux**: Kernel 5.8+, root access, `libssl-dev`
- **macOS**: System Extension approval, TCC permissions
- **Windows**: Admin rights, WinDivert driver

## Testing

```bash
cargo test                         # Unit tests
cd ../oisp-cookbook && make test   # Integration tests with real AI calls
```
