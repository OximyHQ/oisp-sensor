---
title: CLI Reference
description: Complete command-line interface reference
---

## Global Options

These options apply to all commands:

```
oisp-sensor [OPTIONS] <COMMAND>

Options:
  -v, --verbose     Increase verbosity (can be repeated: -vv)
  -f, --format      Output format: text, json [default: text]
  -c, --config      Path to configuration file
  -h, --help        Print help
  -V, --version     Print version
```

## Commands

### record

Start capturing AI activity.

```
oisp-sensor record [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-o, --output <PATH>` | Output file for JSONL events |
| `--web` | Start web UI [default: true] |
| `--port <PORT>` | Web UI port [default: 7777] |
| `--tui` | Start terminal UI |
| `-p, --process <NAMES>` | Filter by process names (comma-separated) |
| `--pid <PIDS>` | Filter by PIDs (comma-separated) |
| `--redaction <MODE>` | Redaction mode: safe, full, minimal [default: safe] |
| `--no-ssl` | Disable SSL/TLS capture |
| `--no-process` | Disable process capture |
| `--no-file` | Disable file capture |
| `--no-network` | Disable network capture |
| `--ebpf-path <PATH>` | Path to eBPF bytecode (Linux) |
| `--libssl-path <PATH>` | Path to libssl.so (Linux) |

**Examples:**

```bash
# Basic recording
sudo oisp-sensor record

# Record to file
sudo oisp-sensor record --output events.jsonl

# Filter by process
sudo oisp-sensor record --process python,node

# Different port
sudo oisp-sensor record --port 8080

# Full capture (no redaction)
sudo oisp-sensor record --redaction full

# Minimal capture
sudo oisp-sensor record --no-file --no-network
```

### show

Display captured events.

```
oisp-sensor show [OPTIONS] <INPUT>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--type <TYPE>` | Filter by event type |
| `--process <NAME>` | Filter by process name |
| `--since <TIME>` | Events after timestamp |
| `--until <TIME>` | Events before timestamp |
| `--limit <N>` | Maximum events to show |
| `--follow` | Follow file for new events (like `tail -f`) |
| `--stats` | Show statistics instead of events |

**Examples:**

```bash
# Show all events
oisp-sensor show events.jsonl

# Filter by type
oisp-sensor show events.jsonl --type ai.request

# Last 10 events
oisp-sensor show events.jsonl --limit 10

# Statistics
oisp-sensor show events.jsonl --stats

# Follow file
oisp-sensor show events.jsonl --follow
```

### analyze

Analyze captured events for patterns and insights.

```
oisp-sensor analyze [OPTIONS] <INPUT>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: text, json, csv |
| `--report <TYPE>` | Report type: summary, costs, providers, models |

**Examples:**

```bash
# Summary analysis
oisp-sensor analyze events.jsonl

# Cost breakdown
oisp-sensor analyze events.jsonl --report costs

# Provider usage
oisp-sensor analyze events.jsonl --report providers

# JSON output
oisp-sensor analyze events.jsonl --format json
```

### status

Check system capabilities and sensor status.

```
oisp-sensor status
```

**Output example (Linux):**

```
OISP Sensor Status
==================

Platform: Linux x86_64
Kernel: 6.1.0-generic

Capabilities:
  Root/CAP_BPF:     Yes
  eBPF Support:     Yes
  BTF Available:    Yes
  libssl Found:     /lib/x86_64-linux-gnu/libssl.so.3

Ready to capture!
```

**Output example (macOS):**

```
OISP Sensor Status
==================

Platform: macOS arm64

Capabilities:
  System Extension:  Not installed
  Full Disk Access:  Unknown

Note: Full capture requires system extension (coming soon)
Demo mode available: oisp-sensor demo
```

### demo

Run with synthetic events (no capture required).

```
oisp-sensor demo [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--port <PORT>` | Web UI port [default: 7777] |
| `--rate <N>` | Events per second [default: 1] |

**Examples:**

```bash
# Start demo
oisp-sensor demo

# Higher event rate
oisp-sensor demo --rate 5
```

### test

Run internal tests and diagnostics.

```
oisp-sensor test [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--capture` | Test capture capabilities |
| `--export` | Test export destinations |
| `--all` | Run all tests |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Permission denied |
| 4 | Capture not supported |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OISP_CONFIG` | Path to config file |
| `OISP_WEB_PORT` | Web UI port |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) |
| `NO_COLOR` | Disable colored output |

## Signals

| Signal | Action |
|--------|--------|
| `SIGINT` (Ctrl+C) | Graceful shutdown |
| `SIGTERM` | Graceful shutdown |
| `SIGHUP` | Reload configuration (planned) |

## Logging

Control log verbosity:

```bash
# Default (info)
oisp-sensor record

# Debug
RUST_LOG=debug oisp-sensor record

# Trace (very verbose)
RUST_LOG=trace oisp-sensor record

# Per-module
RUST_LOG=oisp_capture_ebpf=debug,oisp_decode=trace oisp-sensor record
```

