---
title: Linux Overview
description: Complete overview of OISP Sensor on Linux with full eBPF capture
---


OISP Sensor on Linux is **production-ready** with full SSL/TLS capture capabilities using eBPF (extended Berkeley Packet Filter).

## Status

✅ **Production Ready** - Fully tested and deployed in production environments

## Capabilities

### Full SSL/TLS Capture

Linux is the only platform with **complete** SSL/TLS capture using eBPF:

- **Plaintext capture** - Direct access to decrypted HTTP traffic
- **No proxy required** - Captures at the kernel level, not network level
- **No application changes** - Works with any binary
- **Minimal performance impact** - <3% CPU overhead
- **Invisible to applications** - Cannot be detected by monitored apps

### Event Types

All event types are fully supported on Linux:

| Event Type | Support | Description |
|------------|---------|-------------|
| `ai.request` | ✅ Full | Complete request with model, messages, tools |
| `ai.response` | ✅ Full | Complete response with content, tool calls |
| `agent.tool_call` | ✅ Full | Tool invocations by AI agents |
| `agent.tool_result` | ✅ Full | Tool execution results |
| `process.exec` | ✅ Full | Process execution events |
| `process.exit` | ✅ Full | Process termination events |
| `file.read` | ✅ Full | File read operations |
| `file.write` | ✅ Full | File write operations |
| `network.connect` | ✅ Full | Outbound network connections |
| `network.dns` | ✅ Full | DNS resolution events |

## Distribution Support

OISP Sensor supports all major Linux distributions:

### Officially Tested

| Distribution | Version | Package | Architecture |
|--------------|---------|---------|--------------|
| **Ubuntu** | 22.04 LTS, 24.04 LTS | .deb | x86_64, aarch64 |
| **Debian** | 12 (Bookworm) | .deb | x86_64, aarch64 |
| **Rocky Linux** | 9 | .rpm | x86_64, aarch64 |
| **AlmaLinux** | 9 | .rpm | x86_64, aarch64 |
| **Fedora** | 39, 40 | .rpm | x86_64, aarch64 |
| **RHEL** | 9 | .rpm | x86_64, aarch64 |

See [Distribution Support](./distributions) for detailed compatibility information.

### Architecture Support

- **x86_64** (AMD64) - Fully supported and tested
- **aarch64** (ARM64) - Fully supported and tested

## System Requirements

### Minimum Requirements

| Component | Requirement | Notes |
|-----------|-------------|-------|
| **Kernel** | Linux 5.8+ | 4.18+ works with limited features |
| **BTF** | Required | CONFIG_DEBUG_INFO_BTF=y |
| **Memory** | 256MB RAM | 512MB+ recommended |
| **CPU** | 1 core | 2+ cores for high-throughput |
| **Disk** | 1GB free | More for JSONL logs |
| **OpenSSL** | 1.1.x or 3.x | System library required |

### Kernel Features

Required kernel config options:
- `CONFIG_DEBUG_INFO_BTF=y` - BTF (BPF Type Format) support
- `CONFIG_BPF=y` - eBPF support
- `CONFIG_BPF_SYSCALL=y` - BPF system call
- `CONFIG_BPF_JIT=y` - BPF JIT compiler

Most modern distributions include these by default.

## How It Works

### eBPF Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  User Applications                                           │
│  (Cursor, Python, Node.js, curl, etc.)                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ SSL_read/SSL_write calls
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  OpenSSL Library (libssl.so)                                │
│  ┌─────────────────┐                                        │
│  │ eBPF uprobes    │  ← OISP Sensor hooks                   │
│  └─────────────────┘                                        │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ Plaintext data
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  eBPF Ring Buffer (kernel space)                            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ Efficient zero-copy transfer
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  OISP Sensor (userspace)                                    │
│  ├─ HTTP Parser                                             │
│  ├─ Provider Detection                                      │
│  ├─ Event Correlation                                       │
│  └─ Export (JSONL, OTLP, Kafka, etc.)                       │
└─────────────────────────────────────────────────────────────┘
```

### SSL Library Support

OISP Sensor attaches eBPF uprobes to OpenSSL functions:

**Supported Libraries:**
- OpenSSL 3.x (Ubuntu 22.04+, Debian 12+, RHEL 9+)
- OpenSSL 1.1.x (older distributions)

**Hooked Functions:**
- `SSL_read()` / `SSL_write()` - Main read/write functions
- `SSL_read_ex()` / `SSL_write_ex()` - Extended versions
- `SSL_do_handshake()` - TLS handshake

**Known Limitations:**
- Go crypto/tls - Uses different TLS implementation (future: USDT probes)
- rustls - Rust-native TLS library (future: USDT probes)
- BoringSSL - Chrome/gRPC fork (future: add support)
- GnuTLS - Different API (future: add support)
- NSS - Firefox/Chromium (future: add support)

Most server-side and CLI applications use OpenSSL (90%+ coverage).

## Installation Methods

OISP Sensor supports multiple installation methods on Linux:

1. **[Package Manager](./installation#package-manager)** (.deb/.rpm) - Recommended for production
2. **[Universal Installer](./installation#universal-installer)** - Auto-detects distribution
3. **[Binary Installation](./installation#binary)** - Portable, no dependencies
4. **[Docker](../docker/overview)** - Containerized deployment
5. **[Kubernetes](../kubernetes/overview)** - DaemonSet for cluster-wide monitoring

See the [Installation Guide](./installation) for detailed instructions.

## Production Deployment

For production environments, see:

- **[Production Deployment Guide](./production)** - System requirements, security hardening, monitoring
- **[Running as a Service](./service)** - Systemd service configuration
- **[Troubleshooting](./troubleshooting)** - Common issues and solutions

## Performance

Typical resource usage on Ubuntu 24.04 (Intel Core i7, 16GB RAM):

| Load | CPU | Memory | Disk I/O |
|------|-----|--------|----------|
| **Idle** | <0.5% | 80MB | <100 KB/s |
| **Light** (10 req/s) | <2% | 150MB | <1 MB/s |
| **Heavy** (100 req/s) | <5% | 300MB | <10 MB/s |

## Next Steps

1. **[Install OISP Sensor](./installation)** - Get started with installation
2. **[Quick Start](./quick-start)** - See events in 5 minutes
3. **[Production Deployment](./production)** - Deploy in production
4. **[Troubleshooting](./troubleshooting)** - Solve common issues

---

**Linux is the primary platform for OISP Sensor and receives all new features first.**
