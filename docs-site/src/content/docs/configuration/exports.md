---
title: Export Configuration
description: Configure where events are sent
---

OISP Sensor supports multiple export destinations. Events can be sent to multiple destinations simultaneously.

## JSONL Files

Simple file-based storage in JSON Lines format.

```toml
[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
append = true
pretty = false
flush_each = true
```

### File Rotation

```toml
[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
rotate_size_mb = 100    # Rotate at 100MB
rotate_count = 5        # Keep 5 rotated files
```

### Analysis Example

```bash
# Count events by type
cat events.jsonl | jq -r '.event_type' | sort | uniq -c

# Find all OpenAI requests
cat events.jsonl | jq 'select(.data.provider.name == "openai")'

# Calculate total tokens
cat events.jsonl | jq 'select(.event_type == "ai.response") | .data.usage.total_tokens' | awk '{s+=$1} END {print s}'
```

## WebSocket

Real-time streaming to the Web UI and custom clients.

```toml
[export.websocket]
enabled = true
port = 7777
```

### Connecting from JavaScript

```javascript
const ws = new WebSocket('ws://localhost:7777/ws/events');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data);
};
```

## OpenTelemetry (OTLP)

Export to any OpenTelemetry-compatible backend.

### gRPC Protocol

```toml
[export.otlp]
enabled = true
endpoint = "http://localhost:4317"
protocol = "grpc"
service_name = "oisp-sensor"
batch_size = 100
flush_interval_ms = 5000
```

### HTTP Protocol

```toml
[export.otlp]
enabled = true
endpoint = "http://localhost:4318/v1/logs"
protocol = "http"
service_name = "oisp-sensor"
```

### With Authentication

```toml
[export.otlp]
enabled = true
endpoint = "https://otlp.example.com:4317"
protocol = "grpc"
headers = { "Authorization" = "Bearer your-token" }
tls_cert_path = "/path/to/ca.crt"
```

### Popular Backends

**Grafana Cloud:**
```toml
[export.otlp]
enabled = true
endpoint = "https://otlp-gateway-prod-us-central-0.grafana.net:443"
protocol = "grpc"
headers = { "Authorization" = "Basic base64-encoded-credentials" }
```

**Datadog:**
```toml
[export.otlp]
enabled = true
endpoint = "https://trace.agent.datadoghq.com:4317"
protocol = "grpc"
headers = { "DD-API-KEY" = "your-api-key" }
```

**Honeycomb:**
```toml
[export.otlp]
enabled = true
endpoint = "https://api.honeycomb.io:443"
protocol = "grpc"
headers = { "x-honeycomb-team" = "your-api-key" }
```

## Kafka

High-throughput event streaming to Apache Kafka.

```toml
[export.kafka]
enabled = true
brokers = "kafka1:9092,kafka2:9092,kafka3:9092"
topic = "oisp-events"
batch_size = 100
flush_interval_ms = 1000
compression = "snappy"
acks = "all"
```

### With Authentication

```toml
[export.kafka]
enabled = true
brokers = "kafka:9092"
topic = "oisp-events"
security_protocol = "SASL_SSL"
sasl_mechanism = "PLAIN"
sasl_username = "user"
sasl_password = "password"
```

### Confluent Cloud

```toml
[export.kafka]
enabled = true
brokers = "pkc-xxxxx.us-west-2.aws.confluent.cloud:9092"
topic = "oisp-events"
security_protocol = "SASL_SSL"
sasl_mechanism = "PLAIN"
sasl_username = "API_KEY"
sasl_password = "API_SECRET"
```

### Consuming Events

```bash
# Kafka CLI
kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic oisp-events \
  --from-beginning

# With jq
kafka-console-consumer ... | jq '.event_type'
```

## Webhooks

Send events to any HTTP endpoint.

```toml
[export.webhook]
enabled = true
url = "https://your-service.com/events"
method = "POST"
batch_size = 10
flush_interval_ms = 5000
headers = { "Authorization" = "Bearer token", "Content-Type" = "application/json" }
timeout_ms = 30000
retry_count = 3
```

### Request Format

Events are sent as a JSON array:

```json
{
  "events": [
    { "event_type": "ai.request", ... },
    { "event_type": "ai.response", ... }
  ],
  "sensor_id": "my-sensor",
  "batch_id": "batch-123"
}
```

### Integration Examples

**Slack Webhook:**
```toml
[export.webhook]
enabled = true
url = "https://hooks.slack.com/services/T00/B00/xxx"
# Note: Requires custom transformation service
```

**n8n Webhook:**
```toml
[export.webhook]
enabled = true
url = "https://n8n.example.com/webhook/oisp"
```

## Multiple Exports

Enable multiple exports simultaneously:

```toml
# Store locally
[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"

# Stream to UI
[export.websocket]
enabled = true

# Send to observability backend
[export.otlp]
enabled = true
endpoint = "http://localhost:4317"

# Stream to data pipeline
[export.kafka]
enabled = true
brokers = "localhost:9092"
topic = "oisp-events"
```

## Export Performance

### Batching

All exports (except WebSocket) support batching:

```toml
batch_size = 100        # Max events per batch
flush_interval_ms = 5000  # Max time to wait
```

Events are sent when either threshold is reached.

### Backpressure

If an export destination is slow or unavailable:
- Events are buffered in memory (bounded)
- Oldest events are dropped if buffer is full
- Error counts are tracked in metrics

### Monitoring

Check export health:

```bash
# API endpoint
curl http://localhost:7777/api/health

# Returns:
{
  "status": "healthy",
  "exports": {
    "jsonl": { "status": "ok", "events_exported": 1234 },
    "otlp": { "status": "ok", "events_exported": 1230, "last_error": null },
    "kafka": { "status": "degraded", "last_error": "Connection refused" }
  }
}
```

