---
title: Windows Overview
description: OISP Sensor on Windows - Full SSL/TLS capture via WinDivert + TLS MITM
---

## Current Status

OISP Sensor on Windows provides **full SSL/TLS capture** via WinDivert packet interception and a TLS MITM proxy.

**What works:**
- Full SSL/TLS content capture (request & response bodies)
- Complete AI API request/response parsing
- Tool call content and function definitions
- Process execution/termination events
- Network connection metadata
- File operation events

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   OISP System Tray App (WPF)                     │
│  - Status display, settings, and process control                │
│  - Launches redirector with UAC elevation                       │
│  - One-click CA certificate installation                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                     Named Pipe IPC
                              │
┌─────────────────────────────────────────────────────────────────┐
│                    oisp-sensor.exe (Rust)                        │
│  Receives events, decodes HTTP, emits to dashboard/exports      │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                     Named Pipe IPC
                              │
┌─────────────────────────────────────────────────────────────────┐
│              oisp-redirector.exe (Elevated)                      │
│  - WinDivert packet capture and redirection                     │
│  - TLS MITM proxy (rustls + rcgen)                              │
│  - AI endpoint filtering                                        │
│  - Process attribution                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                    WinDivert Driver
                              │
┌─────────────────────────────────────────────────────────────────┐
│                Windows Network Stack                             │
│  - Kernel-mode packet interception                              │
│  - Pre-signed driver (no test signing required)                 │
└─────────────────────────────────────────────────────────────────┘
```

## Requirements

- **Windows 10/11** (64-bit)
- **Administrator privileges** - For packet capture
- **~50 MB disk space**

## Event Support

| Event Type | Support | Notes |
|------------|---------|-------|
| `ai.request` | Full | Complete request bodies, prompts, tools |
| `ai.response` | Full | Complete response content, token usage |
| `agent.tool_call` | Full | Tool names, arguments, results |
| `process.exec` | Full | Complete process events |
| `file.write` | Full | Complete file events |
| `network.connect` | Full | Complete network events |

## Supported AI Providers

OISP automatically intercepts traffic to these AI endpoints:

| Provider | Endpoints |
|----------|-----------|
| OpenAI | api.openai.com |
| Anthropic | api.anthropic.com |
| Google AI | generativelanguage.googleapis.com, aiplatform.googleapis.com |
| Azure OpenAI | *.openai.azure.com |
| AWS Bedrock | bedrock-runtime.*.amazonaws.com |
| Cohere | api.cohere.ai, api.cohere.com |
| Mistral | api.mistral.ai |
| Groq | api.groq.com |
| Together AI | api.together.xyz, api.together.ai |
| Fireworks | api.fireworks.ai |
| Perplexity | api.perplexity.ai |
| OpenRouter | openrouter.ai, api.openrouter.ai |
| Replicate | api.replicate.com |
| Hugging Face | api-inference.huggingface.co |
| DeepSeek | api.deepseek.com |
| xAI (Grok) | api.x.ai |
| Local (Ollama) | localhost:11434, 127.0.0.1:11434 |
| Local (LM Studio) | localhost:1234, 127.0.0.1:1234 |

## How It Works

### WinDivert Packet Capture

1. **Packet Interception**: WinDivert captures outbound TCP connections to AI endpoints
2. **Traffic Redirection**: Connections are redirected to a local TLS proxy
3. **Process Attribution**: Connection ownership is determined via Windows TCP table APIs

### TLS MITM Proxy

1. **Certificate Authority**: OISP generates a local CA certificate on first run
2. **Certificate Trust**: The CA must be added to the Windows certificate store
3. **Per-host Certificates**: For each AI endpoint, OISP dynamically generates certificates
4. **Bidirectional Proxy**: The proxy terminates TLS and captures plaintext traffic

## Components

| Component | Description |
|-----------|-------------|
| `OISPApp.exe` | System tray application (.NET 8 WPF) |
| `oisp-sensor.exe` | Event processing and export (Rust) |
| `oisp-redirector.exe` | Packet capture and TLS proxy (Rust, requires elevation) |
| `WinDivert.dll` | Packet capture library |
| `WinDivert64.sys` | Kernel-mode driver |

## Known Limitations

- **Certificate Pinning**: Apps that pin certificates (like some browsers) cannot be intercepted
- **HTTP/2**: Currently limited HTTP/2 support (falls back to HTTP/1.1)
- **Non-TCP Traffic**: Only TCP traffic is intercepted (not UDP/QUIC)
- **Antivirus**: Some AV software may flag WinDivert driver

## Next Steps

- [Installation](./installation) - Install on Windows
- [Quick Start](./quick-start) - Get started
