---
title: Introduction
description: What is OISP Sensor and why you need it
---

OISP Sensor is a **zero-instrumentation observability tool** for AI systems. It captures LLM API calls, tool invocations, and agent behavior without requiring any code changes to your applications.

## The Problem

AI agents are becoming increasingly autonomous, making decisions and taking actions on behalf of users. But how do you know:

- What API calls are being made?
- How much are you spending on tokens?
- What data is being sent to AI providers?
- What tools are agents invoking?

Traditional observability tools require instrumenting your code with SDKs. But AI development moves fast, and many teams use multiple libraries, frameworks, and providers.

## The Solution

OISP Sensor sits at the operating system level and captures all AI activity automatically:

```bash
# Install in seconds
curl -sSL https://sensor.oisp.dev/install.sh | sh

# Start recording
sudo oisp-sensor record
```

That's it. No code changes. No SDK integration. No configuration required.

## What It Captures

| Category | Events |
|----------|--------|
| **AI Requests** | OpenAI, Anthropic, Google, Mistral, Cohere, local Ollama |
| **AI Responses** | Completions, streaming chunks, embeddings |
| **Agent Tools** | MCP tool calls, function calling, results |
| **Processes** | Execution, exit, process hierarchy |
| **Files** | Open, read, write operations |
| **Network** | TCP connections, destinations |

## How It Works

On Linux, OISP Sensor uses **eBPF** (Extended Berkeley Packet Filter) to intercept SSL/TLS traffic at the kernel level. This means:

1. **Zero overhead** - eBPF runs in kernel space with minimal performance impact
2. **No proxies** - Traffic is captured directly, not redirected
3. **All languages** - Works with Python, Node.js, Go, Rust, any language that uses OpenSSL/libssl
4. **Encrypted traffic** - Captures plaintext before/after SSL encryption

```
┌─────────────┐
│ AI Agent    │  ← Your Python/Node/Go code
│ (Python)    │
└─────┬───────┘
      │ HTTPS request
      ▼
┌─────────────┐
│ libssl.so   │  ← eBPF uprobes here
└─────┬───────┘
      │ Encrypted
      ▼
┌─────────────┐
│ Network     │
└─────────────┘

OISP Sensor captures at the libssl layer:
✓ Sees plaintext HTTP
✓ No proxy configuration
✓ Works with any HTTP client
```

## Export Options

Captured events can be exported to multiple destinations:

- **JSONL files** - For offline analysis
- **WebSocket** - For real-time streaming
- **Web UI** - Built-in dashboard at `http://localhost:7777`
- **OTLP** - To any OpenTelemetry-compatible backend
- **Kafka** - For high-throughput event streaming
- **Webhooks** - To any HTTP endpoint

## Privacy & Security

OISP Sensor includes built-in redaction:

```toml
[redaction]
mode = "safe"  # Redacts API keys, emails, phone numbers
```

All processing happens locally. No data is sent anywhere unless you configure an export.

## Next Steps

- [Installation Guide](/getting-started/installation) - Get OISP Sensor running
- [Quick Start](/getting-started/quick-start) - Capture your first events
- [Architecture](/architecture/overview) - Deep dive into how it works

