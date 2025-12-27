---
title: Installation
description: Install OISP Sensor on Linux, macOS, or via Docker
---

import { Tabs, TabItem } from '@astrojs/starlight/components';
import { Aside } from '@astrojs/starlight/components';

## Quick Install (Linux/macOS)

The fastest way to install OISP Sensor:

```bash
curl -sSL https://sensor.oisp.dev/install.sh | sh
```

This universal installer:
- **Auto-detects** your Linux distribution (Ubuntu, Debian, RHEL, Fedora, Rocky, Alma)
- **Checks system requirements** (kernel 5.8+, BTF, OpenSSL)
- **Installs via native package** (.deb or .rpm) or fallback to binary
- **Configures systemd service** for automatic startup
- **Sets capabilities** for secure non-root operation

<Aside type="tip">
**✅ Fully Supported:** Ubuntu 22.04+, Debian 12+, Rocky Linux 9, AlmaLinux 9, Fedora 39+, RHEL 9
</Aside>

## Platform-Specific Instructions

<Tabs>
<TabItem label="Linux">

### Prerequisites

- Linux kernel 5.8+ (for eBPF ring buffers)
- Root or CAP_BPF capability

### Using the Install Script

```bash
# Download and run installer
curl -sSL https://sensor.oisp.dev/install.sh | sh

# Verify installation
oisp-sensor --version
oisp-sensor status
```

### Manual Installation

```bash
# Download binary
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-x86_64-unknown-linux-gnu.tar.gz

# Extract
tar -xzf oisp-sensor-x86_64-unknown-linux-gnu.tar.gz

# Install
sudo mv oisp-sensor /usr/local/bin/
sudo chmod +x /usr/local/bin/oisp-sensor

# Set capabilities (allows running without root)
sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/local/bin/oisp-sensor
```

### Package Installation

**Ubuntu / Debian (.deb):**

```bash
# Download .deb package
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor_0.2.0_amd64.deb

# Install
sudo dpkg -i oisp-sensor_0.2.0_amd64.deb

# Start service
sudo systemctl enable --now oisp-sensor
```

**RHEL / Rocky / AlmaLinux / Fedora (.rpm):**

```bash
# Download .rpm package
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-0.2.0-1.x86_64.rpm

# Install
sudo dnf install ./oisp-sensor-0.2.0-1.x86_64.rpm

# For RHEL 8:
# sudo yum install ./oisp-sensor-0.2.0-1.x86_64.rpm

# Start service
sudo systemctl enable --now oisp-sensor
```

### Pre-Flight Check

After installation, verify your system is ready:

```bash
oisp-sensor check
```

**Expected output:**
```
OISP Sensor System Check
========================

Platform: linux x86_64 (supported)
Distribution: Ubuntu 24.04

Kernel Version:    6.8.0 [OK]
BTF Support:       /sys/kernel/btf/vmlinux [OK]
eBPF Filesystem:   /sys/fs/bpf [OK]
Permissions:       CAP_BPF+CAP_PERFMON set [OK]
Systemd:           Available [OK]

SSL Libraries:
  /usr/lib/x86_64-linux-gnu/libssl.so.3 [FOUND]

Result: READY
```

### Systemd Service

After installation, you can run OISP Sensor as a system service:

```bash
# Enable on boot
sudo systemctl enable oisp-sensor

# Start now
sudo systemctl start oisp-sensor

# Check status
sudo systemctl status oisp-sensor

# View logs
sudo journalctl -u oisp-sensor -f
```

</TabItem>
<TabItem label="macOS">

<Aside type="caution">
macOS support is currently in preview. Full eBPF-style capture requires a system extension.
</Aside>

### Install via Script

```bash
curl -sSL https://sensor.oisp.dev/install.sh | sh
```

### Install via Homebrew (coming soon)

```bash
brew tap oximyHQ/oisp
brew install oisp-sensor
```

### Current Limitations

On macOS, OISP Sensor can:
- Run the demo mode with synthetic events
- Display process information
- Export to all supported sinks

Full SSL/TLS capture requires Apple Endpoint Security Framework and Network Extension entitlements (coming in a future release).

</TabItem>
<TabItem label="Docker">

### Quick Start

```bash
# Run with Docker
docker run --privileged -p 7777:7777 oximy/oisp-sensor

# With volume for config
docker run --privileged \
  -p 7777:7777 \
  -v $(pwd)/config.toml:/etc/oisp/config.toml \
  oximy/oisp-sensor
```

### Docker Compose

```yaml
version: '3.8'
services:
  oisp-sensor:
    image: oximy/oisp-sensor:latest
    privileged: true
    ports:
      - "7777:7777"
    volumes:
      - ./config.toml:/etc/oisp/config.toml
      - ./events:/var/log/oisp
    environment:
      - RUST_LOG=info
```

<Aside type="note">
The `--privileged` flag is required for eBPF operations. See [Security Considerations](/configuration/redaction#docker-security) for alternatives.
</Aside>

</TabItem>
<TabItem label="Build from Source">

### Prerequisites

- Rust 1.75+
- Linux: clang, llvm, libelf-dev
- Node.js 18+ (for frontend build)

### Build Steps

```bash
# Clone repository
git clone https://github.com/oximyHQ/oisp-sensor.git
cd oisp-sensor

# Build eBPF programs (Linux only)
cd ebpf
cargo build --release
cd ..

# Build frontend
cd frontend
npm install
npm run build
cd ..

# Build sensor
cargo build --release

# Install
sudo cp target/release/oisp-sensor /usr/local/bin/
```

### Development Build

```bash
# Run with debug output
RUST_LOG=debug cargo run -- record
```

</TabItem>
</Tabs>

## Verify Installation

After installation, verify everything is working:

```bash
# Check version
oisp-sensor --version

# Check system capabilities
oisp-sensor status
```

Expected output on Linux:

```
OISP Sensor Status
==================

Platform: Linux x86_64
Kernel: 6.1.0-generic

Capabilities:
  Root/CAP_BPF:     Yes
  eBPF Support:     Yes
  BTF Available:    Yes
  libssl Found:     /lib/x86_64-linux-gnu/libssl.so.3

Ready to capture!
```

## Next Steps

- [Quick Start Guide](/getting-started/quick-start) - Capture your first events
- [Configuration](/configuration/config-file) - Customize OISP Sensor
- [Architecture](/architecture/overview) - Understand how it works

