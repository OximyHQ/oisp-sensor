# OISP Sensor Production Deployment Guide

This guide covers production deployment of OISP Sensor on Linux systems.

## Table of Contents

- [Prerequisites](#prerequisites)
- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Security](#security)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)
- [Performance Tuning](#performance-tuning)

---

## Prerequisites

### Minimum Requirements

| Component | Requirement | Notes |
|-----------|-------------|-------|
| **Kernel** | Linux 5.8+ | 4.18+ works with limited features |
| **BTF** | Required | CONFIG_DEBUG_INFO_BTF=y |
| **Memory** | 256MB RAM | 512MB+ recommended |
| **CPU** | 1 core | 2+ cores for high-throughput |
| **Disk** | 1GB free | More for JSONL logs |
| **OpenSSL** | 1.1.x or 3.x | System library required |

### Supported Distributions

| Distribution | Package | Status |
|--------------|---------|--------|
| Ubuntu 22.04, 24.04 | .deb | ✅ Fully tested |
| Debian 12 | .deb | ✅ Fully tested |
| Rocky Linux 9 | .rpm | ✅ Fully tested |
| AlmaLinux 9 | .rpm | ✅ Fully tested |
| Fedora 39, 40 | .rpm | ✅ Fully tested |
| RHEL 9 | .rpm | ✅ Compatible |
| Other distros | Binary | ⚠️ Use universal installer |

---

## System Requirements

### Pre-Flight Check

Before installation, run the system check:

```bash
# Download and run universal installer with checks
curl -sSL https://sensor.oisp.dev/install.sh | sh
```

Or manually check:

```bash
# 1. Kernel version
uname -r  # Should be >= 5.8

# 2. BTF support
ls /sys/kernel/btf/vmlinux  # Should exist

# 3. BPF filesystem
ls /sys/fs/bpf  # Should exist

# 4. OpenSSL library
ldconfig -p | grep libssl  # Should show libssl.so.3 or libssl.so.1.1

# 5. Capabilities support
which setcap  # Should exist (install libcap2-bin or libcap)
```

### Enable BTF if Missing

**Ubuntu/Debian:**
```bash
# Install kernel headers
sudo apt-get install linux-headers-$(uname -r)

# Or upgrade to a BTF-enabled kernel
sudo apt-get upgrade linux-image-generic
```

**RHEL/Rocky/Fedora:**
```bash
# Install kernel-devel
sudo dnf install kernel-devel

# Ensure BTF is enabled
sudo grubby --update-kernel=ALL --args="CONFIG_DEBUG_INFO_BTF=y"
```

---

## Installation

### Method 1: Package Manager (Recommended)

**Ubuntu/Debian:**
```bash
# Download .deb package
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor_0.2.0_amd64.deb

# Install
sudo dpkg -i oisp-sensor_0.2.0_amd64.deb

# Fix dependencies if needed
sudo apt-get install -f
```

**RHEL/Rocky/Fedora:**
```bash
# Download .rpm package
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-0.2.0-1.x86_64.rpm

# Install
sudo dnf install ./oisp-sensor-0.2.0-1.x86_64.rpm
# or for RHEL 8:
sudo yum install ./oisp-sensor-0.2.0-1.x86_64.rpm
```

### Method 2: Universal Installer

```bash
# One-line install (auto-detects distro and package manager)
curl -sSL https://sensor.oisp.dev/install.sh | sudo sh

# With custom options
INSTALL_DIR=/opt/oisp/bin curl -sSL https://sensor.oisp.dev/install.sh | sudo sh
```

### Method 3: Binary Installation

```bash
# Download binary
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-x86_64-unknown-linux-gnu.tar.gz

# Extract
tar -xzf oisp-sensor-x86_64-unknown-linux-gnu.tar.gz

# Install
sudo mv oisp-sensor /usr/local/bin/
sudo chmod +x /usr/local/bin/oisp-sensor

# Set capabilities
sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/local/bin/oisp-sensor
```

### Verify Installation

```bash
# Check binary
oisp-sensor --version

# Run system check
oisp-sensor check

# Expected output:
# Platform: linux x86_64 (supported)
# Kernel Version:    5.15.0 [OK]
# BTF Support:       /sys/kernel/btf/vmlinux [OK]
# eBPF Filesystem:   /sys/fs/bpf [OK]
# Permissions:       CAP_BPF+CAP_PERFMON set [OK]
# Systemd:           Available [OK]
# Result: READY
```

---

## Configuration

### Configuration File

Create `/etc/oisp/config.toml`:

```toml
[sensor]
name = "prod-server-01"

[capture]
ssl = true
process = true
file = true
network = true

# SSL library paths (auto-detected by default)
# ssl_binary_paths = [
#     "/usr/lib/x86_64-linux-gnu/libssl.so.3",
#     "~/.nvm/versions/node/v20.10.0/bin/node"
# ]

[redaction]
mode = "safe"  # safe (recommended), full, minimal

# Custom redaction patterns
redact_patterns = [
    "sk-[a-zA-Z0-9]{48}",  # OpenAI keys
    "xai-[a-zA-Z0-9]{48}",  # xAI keys
]

[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
append = true
rotate = true
max_size_mb = 100

[export.otlp]
enabled = false
endpoint = "https://otel.example.com:4317"
# headers = { "x-api-key" = "YOUR_API_KEY" }

[export.kafka]
enabled = false
brokers = ["kafka1.example.com:9092"]
topic = "oisp-events"

[export.webhook]
enabled = false
url = "https://your-webhook.example.com/oisp"

[web]
enabled = true
host = "127.0.0.1"  # Use 0.0.0.0 for remote access (secure with firewall!)
port = 7777
```

### Environment Variables

Override config with env vars:

```bash
# Set in systemd service
Environment=OISP_CONFIG=/etc/oisp/config.toml
Environment=RUST_LOG=info  # debug, info, warn, error
Environment=RUST_BACKTRACE=1
```

---

## Security

### Run as Non-Root (Recommended)

After package installation, the sensor runs as root by default. To run as non-root:

```bash
# 1. Set capabilities on binary
sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/bin/oisp-sensor

# 2. Create service user (done by package installer)
sudo useradd -r -s /bin/false oisp

# 3. Update systemd service
sudo systemctl edit oisp-sensor

# Add:
[Service]
User=oisp
Group=oisp

# 4. Restart
sudo systemctl daemon-reload
sudo systemctl restart oisp-sensor
```

### Firewall Configuration

```bash
# Allow Web UI access (if exposing externally)
sudo ufw allow 7777/tcp

# Or restrict to specific IP
sudo ufw allow from 192.168.1.0/24 to any port 7777
```

### TLS for Web UI

For production, use a reverse proxy with TLS:

**nginx example:**
```nginx
server {
    listen 443 ssl http2;
    server_name oisp.example.com;

    ssl_certificate /etc/letsencrypt/live/oisp.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/oisp.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:7777;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Data Privacy

1. **Enable Redaction:** Use `mode = "safe"` or `mode = "full"`
2. **Custom Patterns:** Add sensitive patterns to `redact_patterns`
3. **File Permissions:** Restrict log file access
   ```bash
   sudo chmod 640 /var/log/oisp/events.jsonl
   sudo chown oisp:oisp /var/log/oisp/events.jsonl
   ```
4. **Rotate Logs:** Enable `rotate = true` in JSONL config

---

## Monitoring

### Systemd Service Management

```bash
# Start service
sudo systemctl start oisp-sensor

# Enable on boot
sudo systemctl enable oisp-sensor

# Check status
sudo systemctl status oisp-sensor

# View logs
sudo journalctl -u oisp-sensor -f

# Reload configuration
sudo systemctl reload oisp-sensor  # Sends SIGHUP

# Restart
sudo systemctl restart oisp-sensor
```

### Health Checks

```bash
# 1. Check process is running
ps aux | grep oisp-sensor

# 2. Check Web UI is responding
curl http://localhost:7777/health

# 3. Check event capture
tail -f /var/log/oisp/events.jsonl

# 4. Check eBPF programs loaded
sudo bpftool prog list | grep oisp
```

### Metrics

The sensor exposes metrics via the Web UI:

- **Events/sec:** Real-time event throughput
- **Buffer usage:** eBPF ring buffer utilization
- **CPU overhead:** Typically <3%
- **Memory usage:** ~100-200MB

Access: `http://localhost:7777/metrics`

### Log Rotation

**Automatic (using logrotate):**

Create `/etc/logrotate.d/oisp-sensor`:

```
/var/log/oisp/events.jsonl {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 oisp oisp
    postrotate
        /bin/systemctl reload oisp-sensor > /dev/null 2>&1 || true
    endscript
}
```

Test:
```bash
sudo logrotate -f /etc/logrotate.d/oisp-sensor
```

---

## Troubleshooting

### Common Issues

#### 1. "Permission denied" when starting

**Solution:**
```bash
# Run with sudo
sudo oisp-sensor record

# Or set capabilities
sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/bin/oisp-sensor
```

#### 2. "BTF not found"

**Solution:**
```bash
# Check if BTF exists
ls /sys/kernel/btf/vmlinux

# If missing, install kernel headers
sudo apt-get install linux-headers-$(uname -r)  # Ubuntu/Debian
sudo dnf install kernel-devel  # RHEL/Fedora
```

#### 3. "No SSL libraries found"

**Solution:**
```bash
# Check for OpenSSL
ldconfig -p | grep libssl

# Install if missing
sudo apt-get install libssl3  # Ubuntu/Debian
sudo dnf install openssl-libs  # RHEL/Fedora
```

#### 4. "Failed to attach eBPF program"

**Solution:**
```bash
# Check kernel version
uname -r  # Must be >= 4.18, preferably >= 5.8

# Check dmesg for errors
sudo dmesg | grep -i bpf

# Increase rlimit
ulimit -l unlimited
```

#### 5. NVM Node.js not captured

**Solution:**

Add Node.js binary path to config:

```toml
[capture]
ssl_binary_paths = [
    "~/.nvm/versions/node/v20.10.0/bin/node"
]
```

### Debug Mode

Enable verbose logging:

```bash
# Via environment variable
RUST_LOG=debug oisp-sensor record

# Or in systemd service
sudo systemctl edit oisp-sensor
# Add:
Environment=RUST_LOG=debug
```

View detailed logs:
```bash
sudo journalctl -u oisp-sensor -f -o verbose
```

### Get Support

1. Run diagnostics:
   ```bash
   oisp-sensor check > system-check.txt
   oisp-sensor ssl-info > ssl-info.txt
   ```

2. Collect logs:
   ```bash
   sudo journalctl -u oisp-sensor --since "1 hour ago" > sensor-logs.txt
   ```

3. Report issue: https://github.com/oximyHQ/oisp-sensor/issues

---

## Performance Tuning

### High-Throughput Environments

For systems with >1000 req/sec:

**1. Increase ring buffer size**

Rebuild with larger buffer (requires recompilation):
```c
// In bpf/sslsniff.bpf.c
#define RING_BUFFER_SIZE (8 << 20)  // 8MB instead of 2MB
```

**2. Adjust systemd limits**

```bash
sudo systemctl edit oisp-sensor

# Add:
[Service]
LimitNOFILE=1000000
LimitMEMLOCK=infinity
CPUQuota=200%  # Limit to 2 cores
```

**3. Use batched exports**

For OTLP/Kafka, enable batching:

```toml
[export.otlp]
batch_size = 100
batch_timeout_ms = 1000
```

**4. Disable TUI/Web UI in production**

```bash
oisp-sensor record --no-web --output /var/log/oisp/events.jsonl
```

### Resource Limits

Typical resource usage:

| Metric | Light Load | Heavy Load |
|--------|-----------|------------|
| CPU | <1% | <5% |
| Memory | 100MB | 300MB |
| Disk I/O | <1 MB/s | <10 MB/s |

Monitor with:
```bash
# CPU and memory
top -p $(pgrep oisp-sensor)

# Disk I/O
iotop -p $(pgrep oisp-sensor)
```

---

## Multi-Node Deployment

### Centralized Logging

**Option 1: OTLP to OpenTelemetry Collector**

```toml
[export.otlp]
enabled = true
endpoint = "http://otel-collector.example.com:4317"
```

**Option 2: Kafka**

```toml
[export.kafka]
enabled = true
brokers = ["kafka1:9092", "kafka2:9092", "kafka3:9092"]
topic = "oisp-events-{sensor_name}"
```

### Configuration Management

Use Ansible/Puppet/Chef to deploy config:

**Ansible example:**
```yaml
- name: Deploy OISP Sensor
  hosts: all
  tasks:
    - name: Install OISP Sensor
      apt:
        deb: https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor_0.2.0_amd64.deb

    - name: Configure sensor
      template:
        src: templates/oisp-config.toml.j2
        dest: /etc/oisp/config.toml
        mode: 0640

    - name: Enable and start service
      systemd:
        name: oisp-sensor
        enabled: yes
        state: started
```

---

## Checklist: Production Readiness

- [ ] Kernel >= 5.8 with BTF support
- [ ] OpenSSL library installed
- [ ] Capabilities set or running as root
- [ ] Configuration file created at `/etc/oisp/config.toml`
- [ ] Redaction mode set to `safe` or `full`
- [ ] Systemd service enabled (`systemctl enable oisp-sensor`)
- [ ] Firewall configured (if exposing Web UI)
- [ ] TLS reverse proxy configured (if remote access)
- [ ] Log rotation configured
- [ ] Monitoring/alerting set up
- [ ] Backup strategy for event logs
- [ ] Tested failover/restart behavior
- [ ] Documentation for your team

---

## Additional Resources

- **Documentation:** https://sensor.oisp.dev
- **GitHub:** https://github.com/oximyHQ/oisp-sensor
- **Support:** https://github.com/oximyHQ/oisp-sensor/issues
- **OISP Spec:** https://spec.oisp.dev
