---
title: Event Schema
description: OISP event format and structure
---

OISP Sensor emits events conforming to the [OISP v0.1 specification](https://github.com/oximyHQ/oisp-spec). This page describes the event schema.

## Event Structure

All events share a common envelope with event-specific data:

```json
{
  "oisp_version": "0.1",
  "event_id": "01HXK7ABCD...",
  "event_type": "ai.request",
  "ts": "2024-12-23T10:30:00.000Z",
  "host": {
    "hostname": "dev-machine",
    "os": "linux",
    "arch": "x86_64"
  },
  "process": {
    "pid": 12345,
    "ppid": 1000,
    "name": "python",
    "exe": "/usr/bin/python3"
  },
  "source": {
    "collector": "oisp-sensor",
    "collector_version": "0.2.0"
  },
  "confidence": {
    "level": "high",
    "completeness": "full"
  },
  "data": {
    // Event-specific fields
  }
}
```

## Envelope Fields

| Field | Type | Description |
|-------|------|-------------|
| `oisp_version` | string | Specification version ("0.1") |
| `event_id` | string | Unique event ID (ULID) |
| `event_type` | string | Event type (e.g., "ai.request") |
| `ts` | string | ISO 8601 timestamp |
| `ts_mono` | number? | Monotonic timestamp (optional) |
| `host` | object | Host information |
| `actor` | object? | User/service identity |
| `process` | object | Process information |
| `source` | object | Collector information |
| `confidence` | object | Data completeness |
| `attrs` | object? | Custom attributes |
| `trace_context` | object? | Distributed tracing |
| `related_events` | array? | Related event IDs |

## Event Types

### ai.request

AI model request:

```json
{
  "event_type": "ai.request",
  "data": {
    "request_id": "req_abc123",
    "provider": {
      "name": "openai",
      "endpoint": "https://api.openai.com/v1/chat/completions"
    },
    "model": {
      "id": "gpt-4",
      "family": "gpt"
    },
    "request_type": "completion",
    "streaming": true,
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello!"}
    ],
    "tools": [],
    "estimated_tokens": 50
  }
}
```

### ai.response

AI model response:

```json
{
  "event_type": "ai.response",
  "data": {
    "request_id": "req_abc123",
    "provider_request_id": "chatcmpl-xyz",
    "status_code": 200,
    "success": true,
    "choices": [
      {
        "index": 0,
        "message": {
          "role": "assistant",
          "content": "Hello! How can I help you today?"
        },
        "finish_reason": "stop"
      }
    ],
    "usage": {
      "prompt_tokens": 25,
      "completion_tokens": 10,
      "total_tokens": 35
    },
    "latency_ms": 850,
    "finish_reason": "stop"
  }
}
```

### ai.streaming_chunk

SSE streaming chunk:

```json
{
  "event_type": "ai.streaming_chunk",
  "data": {
    "request_id": "req_abc123",
    "chunk_index": 5,
    "delta_content": " help",
    "finish_reason": null
  }
}
```

### agent.tool_call

Agent tool invocation:

```json
{
  "event_type": "agent.tool_call",
  "data": {
    "call_id": "call_abc",
    "request_id": "req_abc123",
    "tool_name": "read_file",
    "tool_type": "function",
    "input": {
      "path": "/etc/hosts"
    },
    "mcp_server": "filesystem-server"
  }
}
```

### agent.tool_result

Tool execution result:

```json
{
  "event_type": "agent.tool_result",
  "data": {
    "call_id": "call_abc",
    "request_id": "req_abc123",
    "tool_name": "read_file",
    "success": true,
    "output_preview": "127.0.0.1 localhost...",
    "output_size_bytes": 156,
    "duration_ms": 5
  }
}
```

### process.exec

Process execution:

```json
{
  "event_type": "process.exec",
  "data": {
    "exe": "/usr/bin/python3",
    "args": ["python3", "agent.py"],
    "cwd": "/home/user/project"
  }
}
```

### process.exit

Process termination:

```json
{
  "event_type": "process.exit",
  "data": {
    "exit_code": 0,
    "termination_type": "normal",
    "runtime_ms": 5432
  }
}
```

### file.open

File open operation:

```json
{
  "event_type": "file.open",
  "data": {
    "path": "/home/user/.env",
    "flags": 0,
    "access": "read"
  }
}
```

### network.connect

Network connection:

```json
{
  "event_type": "network.connect",
  "data": {
    "dest": {
      "ip": "104.18.6.192",
      "port": 443,
      "domain": "api.openai.com"
    },
    "protocol": "tcp",
    "success": true
  }
}
```

## Provider Information

The `provider` object identifies the AI service:

```json
{
  "provider": {
    "name": "openai",
    "endpoint": "https://api.openai.com/v1/chat/completions",
    "region": "us-east-1",
    "organization_id": "org-xxx"
  }
}
```

Supported providers:
- `openai` - OpenAI API
- `anthropic` - Anthropic Claude
- `google` - Google AI (Gemini)
- `mistral` - Mistral AI
- `cohere` - Cohere
- `aws_bedrock` - AWS Bedrock
- `azure_openai` - Azure OpenAI
- `ollama` - Local Ollama
- `vllm` - vLLM server
- `openrouter` - OpenRouter
- `together` - Together AI
- `anyscale` - Anyscale Endpoints
- `groq` - Groq
- `fireworks` - Fireworks AI
- `perplexity` - Perplexity AI
- `deepinfra` - DeepInfra

## Model Information

```json
{
  "model": {
    "id": "gpt-4-turbo-preview",
    "name": "GPT-4 Turbo",
    "family": "gpt",
    "version": "2024-01-25",
    "context_window": 128000,
    "max_output_tokens": 4096
  }
}
```

## Usage and Cost

```json
{
  "usage": {
    "prompt_tokens": 1000,
    "completion_tokens": 500,
    "total_tokens": 1500,
    "cached_tokens": 100,
    "input_cost_usd": 0.01,
    "output_cost_usd": 0.03,
    "total_cost_usd": 0.04
  }
}
```

## Confidence Levels

Indicates how complete/reliable the captured data is:

```json
{
  "confidence": {
    "level": "high",        // high, medium, low
    "completeness": "full", // full, partial, minimal
    "flags": []             // Optional: ["truncated", "reassembled"]
  }
}
```

## Trace Context

For distributed tracing integration:

```json
{
  "trace_context": {
    "trace_id": "abc123...",
    "span_id": "def456...",
    "parent_span_id": "parent..."
  }
}
```

## Custom Attributes

Add custom metadata:

```json
{
  "attrs": {
    "deployment.environment": "production",
    "service.name": "my-agent",
    "custom.tag": "value"
  }
}
```

## Redacted Events

When redaction is enabled, sensitive data is replaced:

```json
{
  "data": {
    "request_id": "req_abc123",
    "messages": [
      {"role": "user", "content": "[REDACTED:4 chars]"}
    ]
  }
}
```

See [Redaction](/configuration/redaction) for configuration.

