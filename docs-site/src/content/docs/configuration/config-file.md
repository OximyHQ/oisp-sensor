---
title: Configuration File
description: TOML configuration reference
---

OISP Sensor can be configured via a TOML file. By default, it looks for:

1. `./config.toml` (current directory)
2. `~/.config/oisp/config.toml`
3. `/etc/oisp/config.toml`

Or specify a path: `oisp-sensor record --config /path/to/config.toml`

## Complete Example

```toml
# OISP Sensor Configuration

[sensor]
name = "my-sensor"  # Optional name for this instance

[capture]
ssl = true          # Capture SSL/TLS traffic
process = true      # Capture process events
file = true         # Capture file operations
network = true      # Capture network connections

# Process filtering (empty = all processes)
process_filter = ["python", "node", "claude"]
pid_filter = []     # Specific PIDs to monitor

[redaction]
mode = "safe"       # safe, full, minimal

[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
append = true
pretty = false      # Compact JSON for smaller files
flush_each = true   # Flush after each event

[export.websocket]
enabled = true
port = 7777

[export.otlp]
enabled = false
endpoint = "http://localhost:4317"
protocol = "grpc"   # grpc or http
service_name = "oisp-sensor"
batch_size = 100
flush_interval_ms = 5000

[export.kafka]
enabled = false
brokers = "localhost:9092"
topic = "oisp-events"
batch_size = 100
flush_interval_ms = 1000

[export.webhook]
enabled = false
url = "https://example.com/webhook"
batch_size = 10
flush_interval_ms = 5000
headers = { "Authorization" = "Bearer xxx" }

[web]
enabled = true
host = "0.0.0.0"    # Use 127.0.0.1 to restrict to localhost
port = 7777

[correlation]
enabled = true
time_window_ms = 5000
max_trace_duration_ms = 300000  # 5 minutes
max_traces = 100
```

## Section Reference

### [sensor]

General sensor settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | "oisp-sensor" | Instance name (for multi-sensor setups) |

### [capture]

Event capture configuration.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `ssl` | bool | true | Capture SSL/TLS traffic |
| `process` | bool | true | Capture process exec/exit |
| `file` | bool | true | Capture file operations |
| `network` | bool | true | Capture network connections |
| `process_filter` | array | [] | Process names to monitor (empty = all) |
| `pid_filter` | array | [] | Specific PIDs to monitor |
| `ebpf_bytecode_path` | string? | auto | Path to eBPF bytecode (Linux) |
| `ssl_binary_paths` | array | auto | Paths to libssl.so |

### [redaction]

Sensitive data handling.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mode` | string | "safe" | Redaction mode: safe, full, minimal |

See [Redaction](/configuration/redaction) for details.

### [export.jsonl]

JSONL file export.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | false | Enable JSONL export |
| `path` | string | required | Output file path |
| `append` | bool | true | Append to existing file |
| `pretty` | bool | false | Pretty-print JSON |
| `flush_each` | bool | true | Flush after each event |
| `rotate_size_mb` | int? | none | Rotate when file exceeds size |
| `rotate_count` | int? | 5 | Number of rotated files to keep |

### [export.websocket]

WebSocket streaming (for Web UI).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | true | Enable WebSocket export |
| `port` | int | 7777 | WebSocket port |

### [export.otlp]

OpenTelemetry Protocol export.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | false | Enable OTLP export |
| `endpoint` | string | required | OTLP endpoint URL |
| `protocol` | string | "grpc" | Protocol: grpc, http |
| `service_name` | string | "oisp-sensor" | Service name for traces |
| `batch_size` | int | 100 | Events per batch |
| `flush_interval_ms` | int | 5000 | Max time between flushes |
| `headers` | map | {} | Custom headers |
| `tls_cert_path` | string? | none | TLS certificate path |

### [export.kafka]

Apache Kafka export.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | false | Enable Kafka export |
| `brokers` | string | required | Comma-separated broker list |
| `topic` | string | "oisp-events" | Kafka topic |
| `batch_size` | int | 100 | Events per batch |
| `flush_interval_ms` | int | 1000 | Max time between flushes |
| `compression` | string | "snappy" | Compression: none, gzip, snappy, lz4 |
| `acks` | string | "all" | Acknowledgment level |

### [export.webhook]

HTTP webhook export.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | false | Enable webhook export |
| `url` | string | required | Webhook URL |
| `method` | string | "POST" | HTTP method |
| `batch_size` | int | 10 | Events per request |
| `flush_interval_ms` | int | 5000 | Max time between flushes |
| `headers` | map | {} | Custom headers |
| `timeout_ms` | int | 30000 | Request timeout |
| `retry_count` | int | 3 | Retry attempts |

### [web]

Web UI configuration.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | true | Enable web UI |
| `host` | string | "0.0.0.0" | Bind address |
| `port` | int | 7777 | HTTP port |

### [correlation]

Event correlation and trace building.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | true | Enable trace building |
| `time_window_ms` | int | 5000 | Correlation time window |
| `max_trace_duration_ms` | int | 300000 | Max trace duration |
| `max_traces` | int | 100 | Max traces in memory |

## Environment Variables

Configuration can be overridden with environment variables:

```bash
# General
OISP_CONFIG=/path/to/config.toml

# Capture
OISP_CAPTURE_SSL=true
OISP_CAPTURE_PROCESS=true

# Redaction
OISP_REDACTION_MODE=safe

# Exports
OISP_JSONL_PATH=/var/log/oisp/events.jsonl
OISP_OTLP_ENDPOINT=http://localhost:4317
OISP_KAFKA_BROKERS=localhost:9092

# Web
OISP_WEB_PORT=7777

# Logging
RUST_LOG=info
```

Environment variables take precedence over config file values.

## CLI Override

CLI arguments override both config file and environment variables:

```bash
oisp-sensor record \
  --config /etc/oisp/config.toml \
  --port 8080 \
  --process python,node \
  --output /tmp/events.jsonl
```

## Validation

Validate your configuration:

```bash
oisp-sensor config --validate /path/to/config.toml
```

