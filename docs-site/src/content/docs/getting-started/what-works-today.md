---
title: "What Works Today"
description: "Honest assessment of current OISP Sensor capabilities"
---


> **Last Updated**: December 26, 2024
>
> This page provides an honest assessment of what OISP Sensor can capture today vs what's planned.

## Platform Support

| Platform | Capture Status | What Works |
|----------|:-------------:|------------|
| **Linux** | ✅ **Production Ready** | SSL/TLS content, process, file, network |
| **macOS** | Stub only | Nothing yet - implementation needed |
| **Windows** | Stub only | Nothing yet - implementation needed |

### Linux Status: 100% Complete

**✅ Fully Supported Distributions:**
- Ubuntu 22.04 LTS, 24.04 LTS
- Debian 12 (Bookworm)
- Rocky Linux 9
- AlmaLinux 9
- Fedora 39, 40
- RHEL 9

**System Requirements:**
- **Kernel**: 5.8+ (works on 4.18+)
- **Architecture**: x86_64, aarch64
- **Privileges**: Root or CAP_BPF + CAP_PERFMON + CAP_SYS_ADMIN
- **BTF**: Required (CONFIG_DEBUG_INFO_BTF=y)

**Installation Methods:**
- ✅ .deb package (Debian/Ubuntu)
- ✅ .rpm package (RHEL/Fedora/Rocky/Alma)
- ✅ Universal installer (auto-detects distro)
- ✅ Docker (multi-arch)
- ✅ Binary installation

See [LINUX_STATUS.md](https://github.com/oximyHQ/oisp-sensor/blob/main/LINUX_STATUS.md) for complete details.

## SSL/TLS Library Support

We intercept SSL/TLS at the library level. Support depends on which library your application uses.

| SSL Library | Status | Notes |
|------------|:------:|-------|
| OpenSSL 3.x | **Works** | System library, most common |
| OpenSSL 1.1.x | **Works** | Older systems |
| OpenSSL (statically linked) | **Config needed** | Add binary path to config |
| BoringSSL | **Partial** | Different symbols, may not work |
| GnuTLS | **Not supported** | Different API, needs implementation |
| Go crypto/tls | **Not supported** | Pure Go, needs USDT probes |
| Rust rustls | **Not supported** | Pure Rust, needs different approach |

### How to Check Your Application's SSL Library

```bash
# For binaries
ldd /path/to/your/binary | grep -E "(ssl|crypto)"

# For Python
python3 -c "import ssl; print(ssl.OPENSSL_VERSION)"

# For Node.js
node -e "console.log(process.versions.openssl)"
```

## Language/Runtime Support

| Runtime | SSL Library Used | Works Out of Box? | Notes |
|---------|-----------------|:-----------------:|-------|
| **Python (system)** | System OpenSSL | **Yes** | `/usr/bin/python3` |
| Python (pyenv) | May compile own | **Maybe** | Add path to config |
| Python (conda) | Bundles own | **Maybe** | Add conda lib path |
| **Node.js (system)** | System OpenSSL | **Yes** | `apt install nodejs` |
| Node.js (NVM) | Static OpenSSL | **No** | Add binary path to config |
| Node.js (fnm) | Static OpenSSL | **No** | Add binary path to config |
| **Go applications** | crypto/tls | **No** | Not supported yet |
| **Rust (native-tls)** | System OpenSSL | **Maybe** | Should work if dynamic |
| Rust (rustls) | rustls | **No** | Not supported |
| Java | JSSE | **No** | Not supported |
| Ruby | System OpenSSL | **Yes** | Usually works |

## Provider Detection

We detect AI providers by domain and API shape. Currently supported:

| Provider | Domain | API Parsing |
|----------|--------|:-----------:|
| OpenAI | api.openai.com | **Full** |
| Anthropic | api.anthropic.com | **Full** |
| Google Gemini | generativelanguage.googleapis.com | **Full** |
| Azure OpenAI | *.openai.azure.com | **Full** |
| AWS Bedrock | bedrock-runtime.*.amazonaws.com | Partial |
| Cohere | api.cohere.ai | Basic |
| Mistral | api.mistral.ai | Basic |
| Groq | api.groq.com | Basic |
| Together | api.together.xyz | Basic |
| Fireworks | api.fireworks.ai | Basic |
| Replicate | api.replicate.com | Basic |
| Hugging Face | api-inference.huggingface.co | Basic |
| Perplexity | api.perplexity.ai | Basic |
| DeepSeek | api.deepseek.com | **Full** |
| Ollama (local) | localhost:11434 | **Full** |
| LM Studio (local) | localhost:1234 | **Full** |

**Full**: Request parsing, response parsing, tool call extraction, token usage
**Basic**: Provider detection, model extraction, basic request/response
**Partial**: Some fields may be missing

## Event Types Captured

| Event Type | Linux | Description |
|------------|:-----:|-------------|
| `ai.request` | **Yes** | AI API request with model, messages, tools |
| `ai.response` | **Yes** | AI API response with content, tool calls, usage |
| `process.exec` | **Yes** | Process execution |
| `process.exit` | **Yes** | Process termination |
| `file.open` | **Yes** | File opened |
| `file.read` | Partial | File read (not all captured) |
| `file.write` | Partial | File write (not all captured) |
| `network.connect` | **Yes** | Outbound TCP connection |
| `agent.tool_call` | **Yes** | Extracted from ai.response |
| `agent.tool_result` | Partial | Extracted from subsequent ai.request |

## What We Capture in AI Events

### Request (`ai.request`)

```json
{
  "provider": { "name": "openai", "endpoint": "..." },
  "model": { "id": "gpt-4o", "family": "gpt-4" },
  "request_type": "chat",
  "streaming": true,
  "messages_count": 5,
  "has_system_prompt": true,
  "system_prompt_hash": "sha256:abc123...",
  "tools_count": 3,
  "parameters": {
    "temperature": 0.7,
    "max_tokens": 1000
  }
}
```

### Response (`ai.response`)

```json
{
  "request_id": "...",
  "provider": { "name": "openai" },
  "model": { "id": "gpt-4o" },
  "success": true,
  "finish_reason": "tool_calls",
  "tool_calls": [
    { "name": "get_weather", "arguments": "..." }
  ],
  "usage": {
    "prompt_tokens": 150,
    "completion_tokens": 45,
    "total_tokens": 195
  }
}
```

## What We DON'T Capture Yet

1. **Cost calculation** - We have tokens, but no cost lookup table yet
2. **User/account attribution** - No auth header parsing for user identification
3. **HTTP/2** - Limited support, may miss some traffic
4. **gRPC** - Not supported (affects Google Cloud AI)
5. **WebSocket AI APIs** - Limited support
6. **Go applications** - crypto/tls not intercepted
7. **Trace correlation** - Basic correlation, not full trace reconstruction

## Known Issues

### Large Responses Truncated

eBPF has buffer size limits. Very large responses (>16KB per chunk) may be truncated.

**Workaround**: We capture in chunks and reassemble, but some edge cases may miss data.

### Streaming (SSE) Reassembly

Server-Sent Events are captured per-chunk. We reassemble them, but:
- Very fast streams may have timing issues
- Non-standard SSE formats may not parse correctly

### Static OpenSSL Detection

Applications that statically link OpenSSL (common with NVM Node.js) require manual configuration:

```toml
# ~/.config/oisp-sensor/config.toml
[capture.ssl]
binary_paths = [
    "~/.nvm/versions/node/*/bin/node",
]
```

## Verified Scenarios

These scenarios have been tested and work:

| Scenario | Status | Notes |
|----------|:------:|-------|
| Python + OpenAI SDK | **Verified** | System Python |
| Python + Anthropic SDK | **Verified** | System Python |
| Python + LangChain | **Verified** | With tool calls |
| Node.js + OpenAI SDK | **Verified** | System Node only |
| n8n AI nodes | **Testing** | Needs verification |
| LiteLLM proxy | **Testing** | Needs verification |

## Unverified / Experimental

| Scenario | Status | Notes |
|----------|:------:|-------|
| NVM Node.js | **Unverified** | Needs binary_path config |
| Conda Python | **Unverified** | Needs lib path config |
| Docker containers | **Unverified** | Needs privileged mode |
| Kubernetes | **Unverified** | DaemonSet not tested |
| Go AI applications | **Not supported** | crypto/tls issue |
| Bun / Deno | **Not supported** | Different TLS |

## Getting Help

If something doesn't work:

1. Check kernel version: `uname -r` (need 5.0+)
2. Check SSL library: `ldd /path/to/app | grep ssl`
3. Run with debug: `RUST_LOG=debug sudo oisp-sensor`
4. File an issue with:
   - Distro and kernel version
   - Application and how it's installed
   - SSL library output from `ldd`
   - Debug log output

