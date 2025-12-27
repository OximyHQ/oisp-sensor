---
title: Troubleshooting
description: Common issues and solutions for OISP Sensor on Linux
---

# Troubleshooting

Solve common issues with OISP Sensor on Linux.

## Quick Diagnostics

Before troubleshooting, run the built-in diagnostics:

```bash
# System compatibility check
oisp-sensor check

# SSL library detection
oisp-sensor ssl-info

# Service status (if running as service)
sudo systemctl status oisp-sensor

# View recent logs
sudo journalctl -u oisp-sensor -n 100
```

---

## Installation Issues

### "Permission denied" when running

**Problem:** Cannot run sensor without sudo.

**Cause:** Sensor requires kernel capabilities for eBPF.

**Solutions:**

1. **Run with sudo (simplest):**
   ```bash
   sudo oisp-sensor
   ```

2. **Set capabilities (run without sudo):**
   ```bash
   sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep /usr/bin/oisp-sensor
   oisp-sensor  # Now works without sudo
   ```

3. **Verify capabilities:**
   ```bash
   getcap /usr/bin/oisp-sensor
   # Should show: cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin=ep
   ```

### "BTF not found"

**Problem:** System check fails with BTF not found.

**Cause:** Kernel doesn't have BTF (BPF Type Format) support enabled.

**Check:**

```bash
ls /sys/kernel/btf/vmlinux
# If file doesn't exist, BTF is not available
```

**Solutions:**

**Ubuntu/Debian:**

```bash
# Install kernel headers
sudo apt-get install linux-headers-$(uname -r)

# Or upgrade to a kernel with BTF (5.8+)
sudo apt-get update
sudo apt-get upgrade linux-image-generic
sudo reboot
```

**RHEL/Rocky/Fedora:**

```bash
# Install kernel-devel
sudo dnf install kernel-devel

# Or upgrade kernel
sudo dnf update kernel
sudo reboot
```

**Verify after reboot:**

```bash
uname -r  # Check kernel version (should be 5.8+)
ls /sys/kernel/btf/vmlinux  # Should exist
```

### "No SSL libraries found"

**Problem:** System check or SSL info shows no OpenSSL libraries.

**Cause:** OpenSSL not installed or not in standard location.

**Check:**

```bash
ldconfig -p | grep libssl
```

**Solutions:**

**Ubuntu/Debian:**

```bash
sudo apt-get install libssl3
# Or for older systems:
sudo apt-get install libssl1.1
```

**RHEL/Rocky/Fedora:**

```bash
sudo dnf install openssl-libs
```

**Verify:**

```bash
oisp-sensor ssl-info
# Should show found libraries
```

### "Failed to attach eBPF program"

**Problem:** Sensor fails to start with eBPF attach error.

**Causes:**
1. Kernel too old (< 4.18)
2. Missing kernel config options
3. Resource limits too low

**Solutions:**

1. **Check kernel version:**
   ```bash
   uname -r  # Should be >= 4.18, preferably >= 5.8
   ```

2. **Check dmesg for errors:**
   ```bash
   sudo dmesg | grep -i bpf
   ```

3. **Increase resource limits:**
   ```bash
   ulimit -l unlimited
   ```

4. **Or via systemd:**
   ```bash
   sudo systemctl edit oisp-sensor
   ```
   Add:
   ```ini
   [Service]
   LimitMEMLOCK=infinity
   ```

5. **Verify BPF filesystem:**
   ```bash
   ls /sys/fs/bpf  # Should exist
   ```

### Package installation fails

**Problem:** `dpkg` or `dnf` installation fails.

**Ubuntu/Debian - Missing dependencies:**

```bash
sudo apt-get install -f
# This fixes missing dependencies
```

**RHEL/Fedora - GPG key issues:**

```bash
# Install without GPG check (not recommended for production)
sudo dnf install --nogpgcheck ./oisp-sensor-0.2.0-1.x86_64.rpm
```

---

## Runtime Issues

### No events appearing

**Problem:** Sensor running but no events captured.

**Diagnostics:**

1. **Check if sensor is actually running:**
   ```bash
   ps aux | grep oisp-sensor
   ```

2. **Check if eBPF programs are loaded:**
   ```bash
   sudo bpftool prog list | grep oisp
   # Should show uprobe programs
   ```

3. **Check logs for errors:**
   ```bash
   sudo journalctl -u oisp-sensor -n 50
   ```

4. **Verify AI activity is happening:**
   - Are you making actual API calls?
   - Is your API key set? (`echo $OPENAI_API_KEY`)

5. **Check if SSL capture is working:**
   ```bash
   # Enable debug logging
   RUST_LOG=debug sudo oisp-sensor record --output /tmp/test.jsonl

   # In another terminal, make HTTPS call
   curl -v https://api.openai.com/v1/models
   ```

6. **Check SSL library detection:**
   ```bash
   oisp-sensor ssl-info
   # Should show detected libraries
   ```

**Common causes:**

- **NVM Node.js or pyenv Python** - See [Edge Cases](#edge-cases-nvmpyenv) below
- **Go applications** - Use different TLS implementation (not yet supported)
- **Applications using rustls** - Rust-native TLS (not yet supported)

### Events captured but incomplete

**Problem:** Events appear but are missing content.

**Cause:** Redaction mode is set to `safe` or `minimal`.

**Check config:**

```bash
cat /etc/oisp/config.toml | grep mode
```

**Solutions:**

1. **Enable full capture (for debugging):**
   ```bash
   sudo oisp-sensor record --redaction-mode full --output /tmp/test.jsonl
   ```

2. **Or update config:**
   ```toml
   [redaction]
   mode = "full"  # safe, full, or minimal
   ```

**Note:** Only use `full` mode in trusted environments!

### High CPU usage

**Problem:** Sensor using >10% CPU.

**Diagnostics:**

```bash
# Check CPU usage
top -p $(pgrep oisp-sensor)

# Check event rate
sudo journalctl -u oisp-sensor -f | grep "Captured"
```

**Causes:**
- Very high event rate (>1000 events/sec)
- Too many processes being monitored
- TUI/Web UI enabled in high-load environment

**Solutions:**

1. **Filter by specific processes:**
   ```bash
   sudo oisp-sensor --comm python3,node
   ```

2. **Disable UI in production:**
   ```bash
   sudo oisp-sensor record --no-web --output /var/log/oisp/events.jsonl
   ```

3. **Increase systemd CPU limit:**
   ```bash
   sudo systemctl edit oisp-sensor
   ```
   Add:
   ```ini
   [Service]
   CPUQuota=200%  # Limit to 2 cores
   ```

### High memory usage

**Problem:** Sensor using >1GB memory.

**Diagnostics:**

```bash
ps aux | grep oisp-sensor
```

**Causes:**
- Very large events (huge prompts/responses)
- Memory leak (rare, please report!)
- Buffer not draining to export

**Solutions:**

1. **Check export is working:**
   ```bash
   ls -lh /var/log/oisp/events.jsonl
   # File should be growing
   ```

2. **Enable batched export:**
   ```toml
   [export.jsonl]
   batch_size = 100
   ```

3. **Set memory limit:**
   ```bash
   sudo systemctl edit oisp-sensor
   ```
   Add:
   ```ini
   [Service]
   MemoryMax=512M
   ```

### Service keeps restarting

**Problem:** Service crashes and restarts repeatedly.

**Diagnostics:**

```bash
# Check restart count
sudo systemctl status oisp-sensor

# Check recent logs
sudo journalctl -u oisp-sensor --since "1 hour ago"
```

**Common causes:**

1. **Out of memory:**
   ```bash
   sudo journalctl -u oisp-sensor | grep -i "killed"
   ```
   **Solution:** Increase memory or set limits

2. **Permission issues:**
   ```bash
   sudo journalctl -u oisp-sensor | grep -i "permission denied"
   ```
   **Solution:** Check file permissions, capabilities

3. **Config file errors:**
   ```bash
   # Test config manually
   sudo oisp-sensor record --config /etc/oisp/config.toml --output /tmp/test.jsonl
   ```

---

## Edge Cases: NVM/pyenv

### NVM Node.js not captured

**Problem:** Node.js installed via NVM doesn't show events.

**Cause:** NVM installs Node.js with statically linked OpenSSL in `~/.nvm/`.

**Solution 1: Specify binary path**

```bash
# Find your Node.js binary
which node
# Example: /home/user/.nvm/versions/node/v20.10.0/bin/node

# Run sensor with binary path
sudo oisp-sensor --binary-path /home/user/.nvm/versions/node/v20.10.0/bin/node
```

**Solution 2: Add to config**

```toml
# /etc/oisp/config.toml
[capture.ssl]
binary_paths = [
    "~/.nvm/versions/node/*/bin/node",
]
```

**Verify:**

```bash
# Check if sensor attaches to NVM Node
sudo journalctl -u oisp-sensor | grep "Attached uprobe"
# Should show: Attached uprobe to /home/user/.nvm/.../node
```

### pyenv Python not captured

**Problem:** Python installed via pyenv doesn't show events.

**Cause:** pyenv installs Python with bundled OpenSSL in `~/.pyenv/`.

**Solution:**

```toml
# /etc/oisp/config.toml
[capture.ssl]
binary_paths = [
    "~/.pyenv/versions/*/bin/python*",
]
```

Or run with:

```bash
sudo oisp-sensor --binary-path ~/.pyenv/versions/3.11.0/bin/python
```

### Docker containers not captured

**Problem:** Apps running in Docker containers not showing events.

**Cause:** Sensor running on host can't see into containers by default.

**Solution:** Run sensor IN the container or use privileged Docker mode.

See [Docker Guide](/platforms/docker/) for details.

---

## Export Issues

### Events not writing to file

**Problem:** JSONL file not being created or updated.

**Diagnostics:**

```bash
# Check if file exists
ls -lh /var/log/oisp/events.jsonl

# Check directory permissions
ls -ld /var/log/oisp/

# Check sensor is writing
sudo lsof | grep events.jsonl
```

**Solutions:**

1. **Create directory:**
   ```bash
   sudo mkdir -p /var/log/oisp
   sudo chown oisp:oisp /var/log/oisp
   ```

2. **Check permissions:**
   ```bash
   sudo chmod 755 /var/log/oisp
   ```

3. **Verify config:**
   ```toml
   [export.jsonl]
   enabled = true
   path = "/var/log/oisp/events.jsonl"
   ```

### OTLP export failing

**Problem:** Events not reaching OTLP collector.

**Diagnostics:**

```bash
# Check network connectivity
curl -v http://collector:4317

# Check sensor logs
sudo journalctl -u oisp-sensor | grep -i otlp
```

**Solutions:**

1. **Verify endpoint:**
   ```toml
   [export.otlp]
   enabled = true
   endpoint = "http://collector:4317"  # Not https!
   ```

2. **Check firewall:**
   ```bash
   # Test connection
   telnet collector 4317
   ```

3. **Enable debug logging:**
   ```bash
   RUST_LOG=debug sudo oisp-sensor record
   ```

---

## Debug Mode

Enable verbose logging for troubleshooting:

### Via command line

```bash
RUST_LOG=debug sudo oisp-sensor record --output /tmp/debug.jsonl
```

### Via systemd

```bash
sudo systemctl edit oisp-sensor
```

Add:

```ini
[Service]
Environment="RUST_LOG=debug"
```

Restart and view logs:

```bash
sudo systemctl daemon-reload
sudo systemctl restart oisp-sensor
sudo journalctl -u oisp-sensor -f
```

### Log levels

- `RUST_LOG=error` - Errors only
- `RUST_LOG=warn` - Warnings and errors
- `RUST_LOG=info` - Default level
- `RUST_LOG=debug` - Verbose debugging
- `RUST_LOG=trace` - Very verbose (includes eBPF events)

### Module-specific logging

```bash
# Only eBPF module debug logs
RUST_LOG=oisp_capture_ebpf=debug sudo oisp-sensor

# Multiple modules
RUST_LOG=oisp_capture_ebpf=debug,oisp_decode=debug sudo oisp-sensor
```

---

## Getting Help

### Collect Diagnostics

When reporting an issue, include:

1. **System check:**
   ```bash
   oisp-sensor check > system-check.txt
   ```

2. **SSL info:**
   ```bash
   oisp-sensor ssl-info > ssl-info.txt
   ```

3. **Service logs:**
   ```bash
   sudo journalctl -u oisp-sensor --since "1 hour ago" > sensor-logs.txt
   ```

4. **System info:**
   ```bash
   uname -a > system-info.txt
   cat /etc/os-release >> system-info.txt
   ```

5. **eBPF programs:**
   ```bash
   sudo bpftool prog list > bpf-programs.txt
   ```

### Report an Issue

**GitHub Issues:** https://github.com/oximyHQ/oisp-sensor/issues

Include:
- Output from diagnostic commands above
- Steps to reproduce
- Expected vs actual behavior
- Anonymized config file (remove secrets!)

### Community Support

- **GitHub Discussions:** https://github.com/oximyHQ/oisp-sensor/discussions
- **Documentation:** https://sensor.oisp.dev

---

## Next Steps

- **[Production Deployment](./production)** - Optimize for production
- **[Running as a Service](./service)** - Manage systemd service
- **[Distribution Support](./distributions)** - Check compatibility
