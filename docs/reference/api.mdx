---
title: API Reference
description: REST and WebSocket API documentation
---

OISP Sensor provides a REST API and WebSocket API for integration with external tools.

## Base URL

```
http://localhost:7777
```

The port can be configured via `--port` or `[web].port` in config.

## REST API

### Health Check

```http
GET /api/health
```

**Response:**
```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "0.2.0",
  "capture": {
    "running": true,
    "events_captured": 12345
  },
  "exports": {
    "jsonl": { "status": "ok", "events_exported": 12340 },
    "websocket": { "status": "ok", "clients": 2 }
  }
}
```

### Statistics

```http
GET /api/stats
```

**Response:**
```json
{
  "events": {
    "total": 12345,
    "by_type": {
      "ai.request": 500,
      "ai.response": 500,
      "process.exec": 1000,
      "file.open": 5000,
      "network.connect": 5345
    }
  },
  "ai": {
    "requests": 500,
    "responses": 500,
    "tokens": {
      "prompt": 50000,
      "completion": 25000,
      "total": 75000
    },
    "providers": {
      "openai": 300,
      "anthropic": 150,
      "google": 50
    },
    "models": {
      "gpt-4": 200,
      "claude-3-opus": 150,
      "gpt-3.5-turbo": 100
    }
  },
  "capture": {
    "running": true,
    "uptime_ms": 3600000,
    "events_per_second": 3.4
  }
}
```

### Recent Events

```http
GET /api/events
```

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | int | Max events (default: 100, max: 1000) |
| `offset` | int | Pagination offset |
| `type` | string | Filter by event type |
| `process` | string | Filter by process name |
| `since` | string | ISO timestamp |

**Response:**
```json
{
  "events": [
    {
      "event_id": "01HXK...",
      "event_type": "ai.request",
      "ts": "2024-12-23T10:30:00Z",
      "process": {
        "pid": 12345,
        "name": "python"
      },
      "data": { ... }
    }
  ],
  "total": 500,
  "limit": 100,
  "offset": 0
}
```

### Single Event

```http
GET /api/events/{event_id}
```

**Response:**
```json
{
  "event_id": "01HXK...",
  "event_type": "ai.request",
  "ts": "2024-12-23T10:30:00Z",
  ...
}
```

### Processes

```http
GET /api/processes
```

**Response:**
```json
{
  "processes": [
    {
      "pid": 12345,
      "ppid": 1000,
      "name": "python",
      "exe": "/usr/bin/python3",
      "start_time": "2024-12-23T10:00:00Z",
      "event_count": 150
    }
  ]
}
```

### Process Tree

```http
GET /api/process-tree
```

**Response:**
```json
{
  "tree": [
    {
      "pid": 1000,
      "name": "bash",
      "children": [
        {
          "pid": 12345,
          "name": "python",
          "children": []
        }
      ]
    }
  ]
}
```

### Filters

```http
GET /api/filters
```

**Response:**
```json
{
  "process_filter": ["python", "node"],
  "pid_filter": [12345],
  "filter_enabled": true
}
```

```http
POST /api/filters/process
Content-Type: application/json

{
  "name": "my-agent"
}
```

```http
POST /api/filters/pid
Content-Type: application/json

{
  "pid": 12345
}
```

```http
DELETE /api/filters
```

### Traces

```http
GET /api/traces
```

**Response:**
```json
{
  "traces": [
    {
      "trace_id": "trace-abc",
      "start_time": "2024-12-23T10:30:00Z",
      "duration_ms": 5000,
      "span_count": 5,
      "status": "complete"
    }
  ]
}
```

```http
GET /api/traces/{trace_id}
```

## WebSocket API

### Events Stream

```
ws://localhost:7777/ws/events
```

Connect to receive real-time events.

**Message format:**
```json
{
  "type": "event",
  "data": {
    "event_id": "01HXK...",
    "event_type": "ai.request",
    ...
  }
}
```

**Client example:**
```javascript
const ws = new WebSocket('ws://localhost:7777/ws/events');

ws.onopen = () => {
  console.log('Connected');
  // Optional: subscribe to specific types
  ws.send(JSON.stringify({
    type: 'subscribe',
    filter: {
      event_types: ['ai.request', 'ai.response']
    }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'event') {
    console.log('Event:', msg.data);
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected');
};
```

### Stats Stream

```
ws://localhost:7777/ws/stats
```

Receive periodic statistics updates.

**Message format:**
```json
{
  "type": "stats",
  "data": {
    "events_per_second": 3.4,
    "active_processes": 5,
    "pending_requests": 2
  },
  "interval_ms": 1000
}
```

## Authentication

By default, the API has no authentication (localhost only). For network access, consider:

1. **Reverse proxy** with authentication (nginx, Caddy)
2. **Firewall rules** to restrict access
3. **Future**: Built-in API key authentication

## Rate Limits

No rate limits by default. For production deployments behind a proxy, consider rate limiting at the proxy level.

## CORS

CORS is enabled for all origins by default. Configure via:

```toml
[web]
cors_origins = ["http://localhost:3000", "https://my-dashboard.com"]
```

## Error Responses

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Event not found",
    "details": null
  }
}
```

**Error codes:**
- `NOT_FOUND` - Resource not found
- `INVALID_REQUEST` - Invalid parameters
- `INTERNAL_ERROR` - Server error

