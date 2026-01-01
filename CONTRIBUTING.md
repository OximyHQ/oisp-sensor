# Contributing to OISP Sensor

We welcome contributions across the entire stack—from low-level eBPF probes to React dashboards. This document helps you find where to contribute and how to get started.

## Contribution Areas

### Linux Capture (Rust + eBPF + C)

**Location:** `crates/oisp-capture-ebpf/`, `bpf/`

The Linux sensor uses eBPF to capture SSL/TLS traffic at the kernel level by hooking OpenSSL and GnuTLS functions.

**You can help with:**
- Adding support for additional SSL libraries (BoringSSL, LibreSSL, rustls)
- Improving kernel version compatibility
- Optimizing eBPF programs for performance
- Adding new probe types (file I/O, process exec)

**Prerequisites:** Rust, C, understanding of eBPF, Linux kernel basics

```bash
# Build eBPF programs
cd bpf && make

# Build the capture crate
cargo build -p oisp-capture-ebpf
```

---

### macOS App (Swift + SwiftUI + Network Extension)

**Location:** `macos/`

The macOS app uses a Network Extension to intercept traffic at the system level, with a SwiftUI menu bar app for control.

**You can help with:**
- Improving the menu bar app UX
- Network Extension stability and edge cases
- Code signing and notarization workflow
- DMG packaging and distribution

**Prerequisites:** Swift, SwiftUI, macOS development, Apple Developer Program membership

```bash
cd macos
xcodegen generate
open OISP.xcodeproj
```

---

### Windows App (C# + WPF + WinDivert)

**Location:** `windows/`

The Windows app uses WinDivert for packet capture and a WPF system tray application for user interaction.

**You can help with:**
- System tray app improvements
- Installer (MSI/EXE) packaging
- TLS interception reliability
- Windows service integration
- Certificate management UX

**Prerequisites:** C#, WPF, Windows development, familiarity with WinDivert

```bash
cd windows/OISPApp
dotnet build
```

---

### Core Engine (Rust)

**Location:** `crates/oisp-core/`, `crates/oisp-decode/`, `crates/oisp-export/`

The core engine handles protocol decoding, AI provider detection, event correlation, and data export.

**You can help with:**
- Adding new AI provider detection (new endpoints, request formats)
- Improving HTTP/2 and streaming response handling
- Adding export destinations (Kafka, S3, custom webhooks)
- Performance optimization

**Prerequisites:** Rust

```bash
cargo build -p oisp-core
cargo test -p oisp-decode
```

---

### Web Dashboard (TypeScript + Next.js + React)

**Location:** `frontend/`

A real-time dashboard for visualizing captured AI activity.

**You can help with:**
- UI/UX improvements
- Real-time event streaming
- Timeline and trace visualization
- Dark mode, accessibility

**Prerequisites:** TypeScript, React, Next.js, Tailwind CSS

```bash
cd frontend
npm install
npm run dev
```

---

### Terminal UI (Rust + Ratatui)

**Location:** `crates/oisp-tui/`

A terminal-based interface for monitoring AI activity.

**You can help with:**
- New views (trace details, process tree)
- Keybinding improvements
- Layout and theming
- Performance with high event volumes

**Prerequisites:** Rust, familiarity with TUI frameworks

```bash
cargo run -p oisp-tui
```

---

### Documentation Site (Astro + MDX)

**Location:** `docs-site/`

The documentation at [sensor.oisp.dev](https://sensor.oisp.dev).

**You can help with:**
- Writing guides and tutorials
- Improving API reference docs
- Adding diagrams and visualizations
- Fixing typos and improving clarity

**Prerequisites:** Markdown, basic web development

```bash
cd docs-site
npm install
npm run dev
```

---

### Cookbooks (Python, Node.js, Docker)

**Location:** [github.com/oximyhq/oisp-cookbook](https://github.com/oximyhq/oisp-cookbook)

Example applications demonstrating sensor capabilities.

**You can help with:**
- Adding examples for new frameworks (CrewAI, AutoGen, etc.)
- Edge case examples (static OpenSSL builds, custom runtimes)
- Self-hosted AI tool examples (Ollama, LocalAI)
- Integration examples (n8n, Dify, Flowise)

**Prerequisites:** Varies by example

---

## Development Setup

### Prerequisites

| Platform | Requirements |
|----------|--------------|
| **Linux** | Rust 1.83+, clang, llvm, libbpf-dev, Linux 5.8+ kernel |
| **macOS** | Rust 1.83+, Xcode 15+, Apple Developer Program |
| **Windows** | Rust 1.83+, Visual Studio 2022, .NET 8 SDK |

### Building

```bash
# Clone
git clone https://github.com/oximyhq/sensor.git
cd oisp-sensor

# Build all Rust crates
cargo build

# Run tests
cargo test

# Build release
cargo build --release
```

### Code Style

- **Rust:** Use `cargo fmt` and `cargo clippy`
- **Swift:** Follow Swift standard style
- **C#:** Follow .NET conventions
- **TypeScript:** Use Prettier and ESLint

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests and lints for the area you modified
5. Commit with a descriptive message (see below)
6. Push and open a Pull Request

### Commit Messages

Follow conventional commits:

```
feat(macos): add network extension reconnection logic
fix(decode): handle chunked transfer encoding correctly
docs: add troubleshooting guide for eBPF errors
test(core): add provider detection tests for Azure
```

### PR Guidelines

- Keep PRs focused—one feature or fix per PR
- Link related issues
- Explain the "why" not just the "what"
- Include tests where applicable
- Update docs if behavior changes

## Good First Issues

Look for issues labeled:
- [`good first issue`](https://github.com/oximyhq/sensor/labels/good%20first%20issue) — Great starting points
- [`help wanted`](https://github.com/oximyhq/sensor/labels/help%20wanted) — We need community help
- [`docs`](https://github.com/oximyhq/sensor/labels/docs) — Documentation improvements

## Security

If you discover a security vulnerability, please email **security@oximy.com** instead of opening a public issue.

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.

## Questions?

- Open a [GitHub Discussion](https://github.com/oximyhq/sensor/discussions)
- Email: community@oximy.com
