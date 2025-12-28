<p align="center">
  <a href="https://oximy.com">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/logo-dark.svg">
      <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/logo-light.svg">
      <img src="https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/assets/logo-light.svg" alt="OISP Sensor" width="80">
    </picture>
  </a>
</p>
<p align="center"><b>OISP Sensor</b></p>
<p align="center">See every AI interaction. Zero instrumentation.</p>
<p align="center">
  <a href="https://github.com/oximyHQ/oisp-sensor/releases"><img alt="Release" src="https://img.shields.io/github/v/release/oximyHQ/oisp-sensor?style=flat-square" /></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square" /></a>
  <a href="https://github.com/oximyHQ/oisp-sensor/actions"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/oximyHQ/oisp-sensor/ci.yml?style=flat-square&branch=main" /></a>
</p>

---

## What It Does

OISP Sensor captures **every AI interaction** on a machine—prompts, responses, tool calls, agent actions—without instrumenting applications. One binary, zero config, complete visibility.

---

## macOS & Windows — Shadow AI Visibility

**Purpose:** Monitor AI usage by employees and contractors. See what AI tools people are using, what data they're sending, and which accounts (personal vs corporate) are being used.

### Download

| Platform | Download |
|----------|----------|
| **macOS** (Apple Silicon) | [OISP-Sensor-arm64.dmg](https://github.com/oximyHQ/oisp-sensor/releases/latest) |
| **macOS** (Intel) | [OISP-Sensor-x64.dmg](https://github.com/oximyHQ/oisp-sensor/releases/latest) |
| **Windows** (64-bit) | [OISP-Sensor-Setup.exe](https://github.com/oximyHQ/oisp-sensor/releases/latest) |

### How It Works

| Platform | Technology | Approach |
|----------|------------|----------|
| **macOS** | Swift + Network Extension | System Extension intercepts network traffic at the OS level, TLS inspection via dynamic CA |
| **Windows** | C# WPF + WinDivert | Packet-level capture with TLS MITM proxy, system tray app for easy management |

### Use Cases

- Detect unauthorized AI tools (ChatGPT, Claude, Cursor, Copilot)
- Audit what data is being sent to AI providers
- Identify personal vs corporate API key usage
- Enforce AI usage policies across the organization

**→ [macOS Guide](https://sensor.oisp.dev/platforms/macos/)** · **[Windows Guide](https://sensor.oisp.dev/platforms/windows/)**

---

## Linux, Docker & Kubernetes — AI Agent Observability

**Purpose:** Monitor AI agents and agentic systems in production. Track what your agents are doing—every API call, tool invocation, file access, and external connection.

### Install

```bash
# Linux
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh

# Docker
docker run --privileged ghcr.io/oximyhq/oisp-sensor:latest

# Kubernetes (DaemonSet)
kubectl apply -f https://sensor.oisp.dev/manifests/daemonset.yaml
```

### How It Works

| Platform | Technology | Approach |
|----------|------------|----------|
| **Linux** | Rust + eBPF + libbpf | Kernel-level uprobes on OpenSSL/GnuTLS capture plaintext before encryption |
| **Docker** | eBPF (host kernel) | Container-aware capture with process/container attribution |
| **Kubernetes** | eBPF DaemonSet | Cluster-wide monitoring with pod and namespace attribution |

### Use Cases

- Monitor AI agent behavior in production
- Debug agent tool call chains and decision flows
- Detect anomalous agent actions or data access
- Audit agent interactions for compliance

**→ [Linux Guide](https://sensor.oisp.dev/platforms/linux/)** · **[Docker Guide](https://sensor.oisp.dev/platforms/docker/)** · **[Kubernetes Guide](https://sensor.oisp.dev/platforms/kubernetes/)**

---

## What It Captures

| Event | Description |
|-------|-------------|
| `ai.request` | Model, prompt, tools sent to AI providers |
| `ai.response` | Completion content, token usage, tool calls |
| `agent.tool_call` | Tool invocations by AI agents |
| `file.write` | Files written by AI-driven processes |
| `process.exec` | Commands executed by agents |

**Providers:** OpenAI, Anthropic, Google, Azure, AWS Bedrock, Mistral, Groq, Cohere, DeepSeek, Ollama, and any OpenAI-compatible API.

**→ [Event Schema](https://sensor.oisp.dev/reference/events/)** · **[OISP Spec](https://oisp.dev)**

---

## Documentation

- **[Getting Started](https://sensor.oisp.dev/getting-started/)** — Installation and first steps
- **[Cookbooks](https://sensor.oisp.dev/cookbooks/)** — Ready-to-run examples (Python, Node.js, LangChain, n8n)
- **[Platform Guides](https://sensor.oisp.dev/platforms/)** — Detailed setup for each platform
- **[Configuration](https://sensor.oisp.dev/configuration/)** — Exports, filtering, redaction

---

## Oximy Platform

OISP Sensor captures the data. [**Oximy**](https://oximy.com) turns it into security:

| Capability | Description |
|------------|-------------|
| **Threat Detection** | Real-time analysis with proprietary SLMs |
| **Policy Enforcement** | Control AI access and data flow |
| **Approval Workflows** | Human-in-the-loop gates for agent actions |
| **Audit Evidence** | Immutable compliance logs (SOC 2, HIPAA, GDPR) |

**[Book a Demo](https://oximy.com)** · **[Learn More](https://oximy.com)**

---

## Contributing

We welcome contributions across the entire stack. See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

| Area | Tech Stack | What You Can Work On |
|------|------------|---------------------|
| **Linux Capture** | Rust, eBPF, C | eBPF probes, kernel compatibility, SSL library support |
| **macOS App** | Swift, SwiftUI, Network Extension | App UX, extension stability, code signing |
| **Windows App** | C#, WPF, WinDivert | Installer, tray app, TLS interception |
| **Core Engine** | Rust | Protocol decoding, provider detection, event correlation |
| **Web Dashboard** | TypeScript, Next.js, React | Visualization, real-time updates, UX |
| **Terminal UI** | Rust, Ratatui | TUI views, keybindings, layouts |
| **Docs Site** | Astro, MDX | Guides, tutorials, API reference |
| **Cookbooks** | Python, Node.js, Docker | Example apps, edge cases, integrations |

## License

Apache 2.0 — See [LICENSE](LICENSE)

---

<p align="center">
  <a href="https://oximy.com">Oximy</a> ·
  <a href="https://sensor.oisp.dev">Docs</a> ·
  <a href="https://oisp.dev">OISP Spec</a> ·
  <a href="https://github.com/oximyhq/oisp-cookbook">Cookbooks</a>
</p>
