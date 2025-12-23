# Contributing to OISP Sensor

Thank you for your interest in contributing to OISP Sensor! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.83 or later
- Linux: clang, llvm, libbpf-dev for eBPF development
- macOS: Xcode command line tools
- Windows: Visual Studio Build Tools

### Building

```bash
# Clone the repository
git clone https://github.com/oximyHQ/oisp-sensor.git
cd oisp-sensor

# Build all crates
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Project Structure

```
oisp-sensor/
├── Cargo.toml              # Workspace configuration
├── README.md               # Project overview
├── bpf/                    # eBPF C programs
│   ├── ssl_monitor.bpf.c   # SSL/TLS capture
│   └── process_monitor.bpf.c
├── crates/
│   ├── oisp-sensor/        # Main binary and CLI
│   ├── oisp-core/          # Core types and traits
│   ├── oisp-capture/       # Capture abstraction
│   ├── oisp-capture-ebpf/  # Linux eBPF capture
│   ├── oisp-capture-macos/ # macOS ESF capture
│   ├── oisp-capture-windows/ # Windows ETW capture
│   ├── oisp-decode/        # HTTP/SSE/AI decoding
│   ├── oisp-enrich/        # Event enrichment
│   ├── oisp-redact/        # Redaction/privacy
│   ├── oisp-correlate/     # Trace building
│   ├── oisp-export/        # Event export
│   ├── oisp-tui/           # Terminal UI
│   └── oisp-web/           # Web UI backend
└── docker/                 # Docker configuration
```

## Development Workflow

### Code Style

- Follow Rust standard style (use `cargo fmt`)
- Run `cargo clippy` before submitting PRs
- Keep functions small and focused
- Document public APIs with doc comments

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p oisp-core

# Run with logging
RUST_LOG=debug cargo test
```

### Building eBPF Programs (Linux only)

```bash
cd bpf
make vmlinux  # Generate vmlinux.h
make          # Compile eBPF programs
```

## Contributing Areas

### High Priority

1. **eBPF Programs**: Improve SSL capture, add file content capture
2. **macOS Support**: Implement ESF and Network Extension capture
3. **Windows Support**: Implement ETW-based capture
4. **Provider Detection**: Add more AI providers to the registry
5. **Performance**: Optimize event processing pipeline

### Medium Priority

1. **Web UI**: Improve timeline, add trace visualization
2. **TUI**: Add more views (trace details, risk indicators)
3. **Documentation**: Tutorials, architecture docs
4. **Testing**: Unit tests, integration tests

### Good First Issues

Look for issues labeled `good first issue` in the GitHub issue tracker.

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run lints (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit with a descriptive message
8. Push to your fork
9. Open a Pull Request

### Commit Messages

Follow conventional commits:

```
feat: add support for Gemini provider detection
fix: handle streaming responses correctly
docs: update installation instructions
test: add tests for redaction patterns
refactor: simplify event pipeline
```

## Security

If you discover a security vulnerability, please email security@oximy.com instead of opening a public issue.

## License

By contributing to OISP Sensor, you agree that your contributions will be licensed under the Apache 2.0 License.

## Questions?

- Open a GitHub Discussion
- Join our community Discord (coming soon)
- Email: community@oximy.com

