---
title: OISP Specification
description: Open Instrumentation Standard for AI Pipelines
---

OISP Sensor emits events conforming to the [OISP v0.1 specification](https://github.com/oximyHQ/oisp-spec).

## What is OISP?

**O**pen **I**nstrumentation **S**tandard for AI **P**ipelines (OISP) is an open standard for representing AI system activity. It provides:

- **Unified schema** for AI events across providers
- **Interoperability** between observability tools
- **Extensibility** for custom event types

## Specification Repository

- **GitHub**: https://github.com/oximyHQ/oisp-spec
- **Schema**: JSON Schema files in `/schema/v0.1/`
- **Examples**: Sample events in `/examples/`

## Core Concepts

### Events

An OISP event represents a single observable occurrence in an AI system:

- AI model request/response
- Agent tool invocation
- Process execution
- File operation
- Network connection

### Envelope

Every event has a common envelope with metadata:

```json
{
  "oisp_version": "0.1",
  "event_id": "unique-id",
  "event_type": "ai.request",
  "ts": "2024-12-23T10:30:00.000Z",
  "host": { ... },
  "process": { ... },
  "source": { ... },
  "confidence": { ... },
  "data": { ... }
}
```

### Traces

Related events are grouped into traces:

- Agent session
- Request/response pairs
- Tool call chains

## Event Categories

### ai.*

AI model interactions:
- `ai.request` - Request to AI model
- `ai.response` - Response from AI model
- `ai.streaming_chunk` - Streaming response chunk
- `ai.embedding` - Embedding request

### agent.*

Agent-specific events:
- `agent.tool_call` - Tool invocation
- `agent.tool_result` - Tool result

### process.*

Process lifecycle:
- `process.exec` - Process started
- `process.exit` - Process ended
- `process.fork` - Process forked

### file.*

File operations:
- `file.open` - File opened
- `file.read` - File read
- `file.write` - File written
- `file.close` - File closed

### network.*

Network activity:
- `network.connect` - Outgoing connection
- `network.accept` - Incoming connection
- `network.flow` - Flow summary
- `network.dns` - DNS query

## Provider Detection

OISP defines provider identification via:

1. **Endpoint URL patterns**
2. **Request headers**
3. **Response headers**
4. **Request/response body structure**

See `/semconv/providers/` in the spec for detection rules.

## Semantic Conventions

OISP extends OpenTelemetry semantic conventions for AI:

### AI Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `ai.provider` | string | Provider name |
| `ai.model.id` | string | Model identifier |
| `ai.model.family` | string | Model family (gpt, claude, etc.) |
| `ai.request_type` | string | completion, embedding, etc. |
| `ai.tokens.prompt` | int | Input tokens |
| `ai.tokens.completion` | int | Output tokens |
| `ai.latency_ms` | float | Response latency |

### Agent Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `agent.tool.name` | string | Tool name |
| `agent.tool.type` | string | function, mcp, etc. |
| `agent.mcp_server` | string | MCP server name |

## Validation

Validate events against the schema:

```bash
# Using ajv-cli
npx ajv validate -s oisp-spec/schema/v0.1/ai-request.json -d event.json
```

## Contributing

The OISP specification is open for contributions:

1. Fork https://github.com/oximyHQ/oisp-spec
2. Propose changes via pull request
3. Discuss in issues

## Relationship to OpenTelemetry

OISP is designed to complement OpenTelemetry:

- Uses OTel semantic convention patterns
- Events can be exported as OTel logs/spans
- Extends OTel for AI-specific concepts

OISP Sensor exports to OTLP natively.

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 0.1 | 2024-12 | Initial release |

## Links

- [OISP Spec GitHub](https://github.com/oximyHQ/oisp-spec)
- [JSON Schema](https://github.com/oximyHQ/oisp-spec/tree/main/schema/v0.1)
- [Examples](https://github.com/oximyHQ/oisp-spec/tree/main/examples)
- [OpenTelemetry](https://opentelemetry.io/)

