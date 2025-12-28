<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-light.svg">
  <img alt="OISP Sensor - Universal AI Activity Observability" src="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/banner-light.svg" width="100%">
</picture>

<div align="center">

# OISP Sensor

**Universal AI Activity Observability. Zero Instrumentation.**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/oximyHQ/oisp-sensor/ci.yml?branch=main)](https://github.com/oximyHQ/oisp-sensor/actions)
[![Release](https://img.shields.io/github/v/release/oximyHQ/oisp-sensor)](https://github.com/oximyHQ/oisp-sensor/releases)

[Quick Start](#quick-start) · [Documentation](https://sensor.oisp.dev) · [Cookbooks](https://github.com/oximyhq/oisp-cookbook) · [Community](#community)

</div>

---

## What It Does

OISP Sensor captures **every AI interaction** on your machine without changing a single line of code. It observes at the system boundary—below your applications, above the network—giving you complete visibility into:

- **What AI tools are running** (ChatGPT, Cursor, Claude CLI, custom agents)
- **What data is being sent** to AI providers (prompts, context, files)
- **What actions agents take** (tool calls, file writes, command executions)
- **Which accounts are being used** (personal vs. corporate API keys)

Traditional observability requires instrumenting every app. OISP Sensor works differently: one binary, zero config, immediate visibility.

### Use Cases

- **Development Monitoring** - See what AI tools your engineers use and how
- **Production Observability** - Track AI agents running on servers in real-time
- **Security Auditing** - Detect unauthorized AI usage or data exfiltration
- **Cost Tracking** - Understand AI spend across teams and projects

---

## Quick Start

### Linux (Production Ready)

Full SSL/TLS capture with eBPF:

```bash
# One-line install
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh

# Start with TUI
sudo oisp-sensor

# Or with Web UI
sudo oisp-sensor --web
```

**→ [Full Linux Guide](https://sensor.oisp.dev/platforms/linux/)**

**Supported:** Ubuntu 22.04+, Debian 12+, RHEL 9, Rocky Linux 9, Fedora 39+

---

### macOS (Preview)

Full SSL/TLS capture via System Extension:

```bash
# Build from source (requires Apple Developer ID)
cd macos && xcodegen generate && xcodebuild -scheme OISP build

# Run the sensor (listens for events from Network Extension)
./target/release/oisp-sensor record --output ~/oisp-events.jsonl

# Or with web dashboard
./target/release/oisp-sensor record --web --output ~/oisp-events.jsonl
```

**→ [macOS Guide](https://sensor.oisp.dev/platforms/macos/)**

**Requirements:**
- macOS 13+ (Ventura)
- Apple Developer Program ($99/year) for System Extension signing
- User must approve Network Extension in System Settings

---

### Windows (Preview)

Metadata capture (process, network, file events):

```powershell
# Install via winget
winget install Oximy.OISPSensor

# Run
oisp-sensor.exe
```

**→ [Windows Guide](https://sensor.oisp.dev/platforms/windows/)**

**Note:** Full SSL capture coming soon with ETW service

---

### Docker

Works on any Linux distribution:

```bash
docker run --privileged --pid=host --network=host \
  -v /sys:/sys:ro -v /usr:/usr:ro -v /lib:/lib:ro \
  ghcr.io/oximyhq/oisp-sensor:latest
```

**→ [Docker Guide](https://sensor.oisp.dev/platforms/docker/)**

---

### Kubernetes

Deploy as DaemonSet:

```bash
kubectl apply -f https://sensor.oisp.dev/manifests/daemonset.yaml
```

**→ [Kubernetes Guide](https://sensor.oisp.dev/platforms/kubernetes/)**

---

## Platform Support

| Platform | Status | SSL Capture | Docs |
|----------|--------|-------------|------|
| **Linux** (Ubuntu, Debian, RHEL, Rocky, Fedora) | ✅ Production | Full (eBPF) | [Guide](https://sensor.oisp.dev/platforms/linux/) |
| **macOS** | ⚠️ Preview | Metadata only | [Guide](https://sensor.oisp.dev/platforms/macos/) |
| **Windows** | ⚠️ Preview | Metadata only | [Guide](https://sensor.oisp.dev/platforms/windows/) |
| **Docker** (Linux) | ✅ Production | Full (eBPF) | [Guide](https://sensor.oisp.dev/platforms/docker/) |
| **Kubernetes** (Linux nodes) | ✅ Production | Full (eBPF) | [Guide](https://sensor.oisp.dev/platforms/kubernetes/) |

---

## What It Captures

### Event Types

| Event | Description | Linux | macOS | Windows |
|-------|-------------|:-----:|:-----:|:-------:|
| `ai.request` | AI API request (model, prompt, tools) | Full | Meta | Meta |
| `ai.response` | AI API response (content, tool calls) | Full | Meta | Meta |
| `agent.tool_call` | Tool invocation by AI agent | Full | Meta | Meta |
| `process.exec` | Process execution | Full | Full | Full |
| `file.write` | File write operation | Full | Full | Full |
| `network.connect` | Outbound connection | Full | Full | Full |

**Full** = Complete data including content
**Meta** = Metadata only (provider, timing, size)

**→ [Complete Event Schema](https://sensor.oisp.dev/reference/events/)**

---

### Supported AI Providers

Automatic detection for:

- OpenAI (ChatGPT, GPT-4, etc.)
- Anthropic (Claude)
- Google (Gemini)
- Azure OpenAI
- AWS Bedrock
- Mistral, Cohere, Groq
- Ollama (local)
- LM Studio (local)
- Any OpenAI-compatible API

**→ [Provider Detection Guide](https://sensor.oisp.dev/architecture/providers/)**

---

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│  Your Apps: Cursor, ChatGPT, Claude CLI, Python scripts, ...   │
├─────────────────────────────────────────────────────────────────┤
│                    OISP Sensor (this tool)                      │
│         Captures everything at the system boundary              │
├─────────────────────────────────────────────────────────────────┤
│                     Operating System                            │
└─────────────────────────────────────────────────────────────────┘
```

### Linux: eBPF Full Capture

OISP Sensor uses eBPF (extended Berkeley Packet Filter) to intercept SSL/TLS traffic at the kernel level:

- **No application changes** - Works with any binary
- **No MITM proxy** - Captures plaintext directly from OpenSSL
- **No performance impact** - <3% CPU overhead
- **No detection** - Invisible to applications

**→ [Architecture Details](https://sensor.oisp.dev/architecture/overview/)**

---

## Example Output

### Terminal UI (TUI)

```
┌─ OISP Sensor ─────────────────────────────────── v0.2.0 ────┐
│                                                              │
│  AI ACTIVITY (last 5 min)                                   │
│  ──────────────────────────────────────────────────────────│
│  14:32:15  ai.request   OpenAI gpt-4o      cursor  [FULL]  │
│  14:32:16  ai.response  OpenAI gpt-4o      cursor  [FULL]  │
│  14:32:16  tool_call    write_file         cursor          │
│  14:32:17  file.write   /src/main.rs       cursor          │
│  14:32:18  process.exec cargo build        cursor→sh       │
│                                                              │
│  PROVIDERS          APPS USING AI          CONFIDENCE       │
│  ─────────────      ─────────────          ──────────       │
│  OpenAI      47     cursor       52        Full: 89%        │
│  Anthropic   12     claude-cli    8        Metadata: 11%    │
│                                                              │
│  [t]imeline  [i]nventory  [p]rocess tree  [q]uit          │
└──────────────────────────────────────────────────────────────┘
```

### Web UI

Access at `http://localhost:7777` when running with `--web` flag.

**→ [Web UI Guide](https://sensor.oisp.dev/guides/web-ui/)**

---

## Common Workflows

### Monitor Development Environment

```bash
# Start sensor with web UI
sudo oisp-sensor --web

# Open browser to http://localhost:7777
# Use Cursor, Claude CLI, or any AI tool
# See real-time activity in the Web UI
```

### Export to Production Monitoring

```bash
# Export to OTLP (OpenTelemetry)
sudo oisp-sensor --export otlp --otlp-endpoint http://collector:4317

# Or to Kafka
sudo oisp-sensor --export kafka --kafka-brokers kafka1:9092 --kafka-topic ai-events

# Or to JSONL file
sudo oisp-sensor --output /var/log/oisp/events.jsonl
```

**→ [Export Configuration Guide](https://sensor.oisp.dev/configuration/exports/)**

---

### Analyze AI Usage

```bash
# Capture events to file
sudo oisp-sensor record --output events.jsonl

# Later, analyze the file
oisp-sensor analyze events.jsonl

# Shows:
# - Provider inventory (OpenAI, Anthropic, etc.)
# - Model usage (gpt-4, claude-3, etc.)
# - Cost estimates
# - Top applications
```

**→ [Analysis Guide](https://sensor.oisp.dev/guides/analysis/)**

---

## Cookbooks

Ready-to-run examples for common scenarios:

| Cookbook | Description | Guide |
|----------|-------------|-------|
| **Python + OpenAI** | Simple chat completion | [Guide](https://sensor.oisp.dev/cookbooks/python/openai-simple/) |
| **Python + LangChain** | Agent with tool calls | [Guide](https://sensor.oisp.dev/cookbooks/python/langchain-agent/) |
| **Node.js + OpenAI** | TypeScript chat app | [Guide](https://sensor.oisp.dev/cookbooks/node/openai-simple/) |
| **n8n Self-Hosted** | n8n workflow automation | [Guide](https://sensor.oisp.dev/cookbooks/self-hosted/n8n/) |
| **Kubernetes DaemonSet** | Cluster-wide monitoring | [Guide](https://sensor.oisp.dev/cookbooks/kubernetes/daemonset/) |
| **Python Celery** | Multi-process workers | [Guide](https://sensor.oisp.dev/cookbooks/multi-process/celery/) |
| **NVM Node.js** | Static OpenSSL edge case | [Guide](https://sensor.oisp.dev/cookbooks/edge-cases/nvm-node/) |
| **pyenv Python** | Static OpenSSL edge case | [Guide](https://sensor.oisp.dev/cookbooks/edge-cases/pyenv-python/) |

**→ [Browse All Cookbooks](https://sensor.oisp.dev/cookbooks/)**

---

## Documentation

### Getting Started
- [Installation](https://sensor.oisp.dev/getting-started/installation/) - Install on Linux, macOS, Windows, Docker, Kubernetes
- [Quick Start](https://sensor.oisp.dev/getting-started/quick-start/) - Get up and running in 5 minutes
- [What It Captures](https://sensor.oisp.dev/getting-started/what-it-captures/) - Event types and provider support

### Platform Guides
- [Linux](https://sensor.oisp.dev/platforms/linux/) - Production deployment, systemd service, troubleshooting
- [macOS](https://sensor.oisp.dev/platforms/macos/) - Preview features and limitations
- [Windows](https://sensor.oisp.dev/platforms/windows/) - Preview features and limitations
- [Docker](https://sensor.oisp.dev/platforms/docker/) - Container deployment
- [Kubernetes](https://sensor.oisp.dev/platforms/kubernetes/) - DaemonSet deployment

### Configuration
- [Config File](https://sensor.oisp.dev/configuration/config-file/) - TOML configuration reference
- [Export Formats](https://sensor.oisp.dev/configuration/exports/) - JSONL, OTLP, Kafka, Webhook
- [Redaction & Privacy](https://sensor.oisp.dev/configuration/redaction/) - Safe mode, custom patterns
- [Filtering](https://sensor.oisp.dev/configuration/filters/) - Process and event filtering

### Reference
- [CLI Commands](https://sensor.oisp.dev/reference/cli/) - Complete command reference
- [Event Types](https://sensor.oisp.dev/reference/events/) - Event schema and examples
- [OISP Spec](https://sensor.oisp.dev/reference/oisp-spec/) - Open Inference Standard Protocol

### Guides
- [Troubleshooting](https://sensor.oisp.dev/guides/troubleshooting/) - Common issues and solutions
- [Multi-Node Deployment](https://sensor.oisp.dev/guides/multi-node/) - Centralized logging patterns
- [CI/CD Integration](https://sensor.oisp.dev/guides/ci-cd/) - Integrate with your pipeline

---

## Privacy & Security

OISP Sensor is designed with **privacy as a first-class concern**.

### Safe Mode (Default)

By default, OISP Sensor operates in "safe mode":

- **Content is hashed, not stored** - You see that a request happened, but not the actual prompt/response
- **Secrets are redacted** - API keys, tokens, passwords are detected and replaced with `[REDACTED]`
- **Local only** - Nothing leaves the machine unless you configure an external exporter
- **Confidence markers** - Every event clearly indicates what was captured vs. inferred

### Full Capture Mode

For debugging or authorized monitoring, you can enable full capture:

```bash
sudo oisp-sensor --redaction-mode full
```

Use responsibly. See [Privacy Guide](https://sensor.oisp.dev/configuration/redaction/) for details.

---

## Performance

Resource usage on Linux (Ubuntu 24.04, Intel Core i7):

| Load | CPU | Memory | Disk I/O |
|------|-----|--------|----------|
| Idle | <0.5% | 80MB | <100 KB/s |
| Light (10 req/s) | <2% | 150MB | <1 MB/s |
| Heavy (100 req/s) | <5% | 300MB | <10 MB/s |

**→ [Performance Tuning Guide](https://sensor.oisp.dev/guides/performance/)**

---

## Community

- **GitHub Discussions** - [Ask questions, share ideas](https://github.com/oximyHQ/oisp-sensor/discussions)
- **GitHub Issues** - [Report bugs, request features](https://github.com/oximyHQ/oisp-sensor/issues)
- **Twitter/X** - [@oximyHQ](https://twitter.com/oximyHQ)
- **Documentation** - [sensor.oisp.dev](https://sensor.oisp.dev)

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Areas where we especially need help:
- macOS System Extension implementation
- Windows ETW implementation
- Additional AI provider parsers
- Documentation and examples

---

## Acknowledgments

- [AgentSight](https://github.com/eunomia-bpf/agentsight) - Inspiration for the eBPF approach
- [libbpf](https://github.com/libbpf/libbpf) - eBPF library
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [OpenTelemetry](https://opentelemetry.io/) - Observability standards

---

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

<div align="center">

**OISP Sensor is part of the [Oximy](https://oximy.com) open-source ecosystem.**

[Website](https://oisp.dev) · [Documentation](https://sensor.oisp.dev) · [GitHub](https://github.com/oximyHQ/oisp-sensor) · [Cookbooks](https://github.com/oximyhq/oisp-cookbook)

</div>
