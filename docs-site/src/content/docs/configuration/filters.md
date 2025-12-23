---
title: Filters
description: Control what events are captured
---

Filters control which events OISP Sensor captures. Filtering happens at different levels for performance.

## Kernel-Level Filtering (eBPF)

The most efficient filtering happens in the kernel, before events reach userspace.

### Process Name Filter

Only capture events from specific processes:

```toml
[capture]
process_filter = ["python", "node", "claude", "cursor"]
```

Or via CLI:

```bash
oisp-sensor record --process python,node
```

The filter matches the process `comm` (first 15 characters of executable name).

### PID Filter

Monitor specific process IDs:

```toml
[capture]
pid_filter = [12345, 67890]
```

Or via CLI:

```bash
oisp-sensor record --pid 12345
```

### Filter Behavior

- **Empty filter = all processes**: If no filter is specified, all processes are monitored
- **Whitelist mode**: Only matching processes are captured
- **OR logic**: Event is captured if PID matches OR comm matches

### Example: Monitor AI Agents

```toml
[capture]
process_filter = [
  "python",     # Python-based agents
  "node",       # JavaScript agents
  "claude",     # Claude CLI
  "cursor",     # Cursor editor
  "code",       # VS Code with Copilot
  "aider",      # Aider CLI
]
```

## Event Type Filtering

Control which event types are captured:

```toml
[capture]
ssl = true        # SSL/TLS traffic
process = true    # Process exec/exit
file = false      # Disable file events (noisy)
network = true    # Network connections
```

### Reducing Noise

File events can be very noisy. Disable if not needed:

```bash
oisp-sensor record --no-file
```

Or in config:

```toml
[capture]
file = false
```

## Path Filtering (eBPF Level)

The eBPF programs automatically filter out common system paths:

```
/proc/*
/sys/*
/dev/*
```

These are not configurable (hardcoded in eBPF for performance).

## Userspace Filtering

Additional filtering can happen after capture:

### Via Redaction

Events matching patterns can be dropped or modified:

```toml
[redaction]
mode = "safe"
drop_patterns = [
  "/health",      # Health check endpoints
  "/metrics",     # Prometheus metrics
]
```

### Via Export

Each export can have its own filters:

```toml
[export.otlp]
enabled = true
endpoint = "http://localhost:4317"
filter_event_types = ["ai.request", "ai.response"]  # Only AI events
```

## Dynamic Filtering

Filters can be updated at runtime via the API:

```bash
# Add a PID to filter
curl -X POST http://localhost:7777/api/filters/pid \
  -H "Content-Type: application/json" \
  -d '{"pid": 12345}'

# Add a process name
curl -X POST http://localhost:7777/api/filters/process \
  -H "Content-Type: application/json" \
  -d '{"name": "my-agent"}'

# Get current filters
curl http://localhost:7777/api/filters

# Clear all filters
curl -X DELETE http://localhost:7777/api/filters
```

## Performance Impact

| Filter Level | Performance Impact | Recommended |
|--------------|-------------------|-------------|
| eBPF (kernel) | Minimal | Always use for high-volume filtering |
| Userspace | Low | For complex logic |
| Export | Negligible | For per-destination filtering |

### Best Practices

1. **Filter early**: Use process_filter/pid_filter for broad filtering
2. **Disable unused captures**: Set `file = false` if not needed
3. **Use specific processes**: Monitor only relevant applications
4. **Avoid filter sprawl**: Keep filters simple for maintainability

## Examples

### Monitor Only Python AI Agents

```toml
[capture]
ssl = true
process = true
file = false
network = true
process_filter = ["python", "python3"]
```

### Development: Monitor Everything

```toml
[capture]
ssl = true
process = true
file = true
network = true
# No process_filter = capture all
```

### Production: Minimal Capture

```toml
[capture]
ssl = true
process = true
file = false
network = false
process_filter = ["my-agent"]
```

