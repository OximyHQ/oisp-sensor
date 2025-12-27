---
title: pyenv Python
description: Capture AI activity from pyenv-managed Python installations
---

# pyenv Python

Capture AI activity from Python installed via pyenv.

## Overview

**What this demonstrates:**
- Handling pyenv-managed Python binaries
- Custom binary path configuration
- SSL capture from non-standard Python installations

**Repository:** `oisp-cookbook/edge-cases/pyenv-python`

---

## The Challenge

**Problem:** pyenv installs Python in user home directories (`~/.pyenv/versions/...`) with bundled OpenSSL, making automatic detection difficult.

**Solution:** Explicitly configure OISP Sensor to monitor pyenv binaries.

---

## Prerequisites

- Linux with eBPF support (kernel 5.8+)
- pyenv installed
- Python installed via pyenv
- OpenAI API key

---

## Running the Example

### With Docker Compose (Recommended)

```bash
cd oisp-cookbook/edge-cases/pyenv-python
docker-compose up
```

**What happens:**
1. Container installs pyenv and Python 3.11
2. OISP Sensor starts with custom binary path
3. Python script makes OpenAI API call
4. Events captured and validated

### Without Docker

**1. Find your pyenv Python binary:**

```bash
which python
# Example output: /home/user/.pyenv/versions/3.11.0/bin/python

# Or check active version
pyenv version
# Example: 3.11.0 (set by /home/user/.python-version)
```

**2. Start OISP Sensor with binary path:**

```bash
sudo oisp-sensor record \
  --output /tmp/pyenv-events.jsonl \
  --binary-path $(which python)
```

**3. In another terminal, run the example:**

```bash
cd oisp-cookbook/edge-cases/pyenv-python
pip install openai
export OPENAI_API_KEY="your-api-key"
python app.py
```

**4. Stop sensor (Ctrl+C) and validate:**

```bash
cat /tmp/pyenv-events.jsonl | jq -r '.event_type' | sort | uniq -c
```

---

## Expected Output

### Application Output

```
Response: Hello! How can I assist you today?
Tokens: 29
```

### Events Captured

```bash
$ cat output/events.jsonl | jq -r '.event_type' | sort | uniq -c
      1 ai.request
      1 ai.response
```

**Sample `ai.request` event:**

```json
{
  "event_type": "ai.request",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "provider": "OpenAI",
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Say hello!"}
    ]
  },
  "process": {
    "pid": 12345,
    "comm": "python3",
    "exe": "/home/user/.pyenv/versions/3.11.0/bin/python3.11"
  }
}
```

---

## Configuration

### Option 1: Command Line

```bash
sudo oisp-sensor --binary-path ~/.pyenv/versions/3.11.0/bin/python
```

### Option 2: Config File

**`/etc/oisp/config.toml`:**

```toml
[sensor]
name = "pyenv-dev-machine"

[capture]
ssl = true
process = true

[capture.ssl]
# Monitor all pyenv Python versions
binary_paths = [
    "~/.pyenv/versions/*/bin/python*",
]

[redaction]
mode = "full"  # For testing; use "safe" in production

[export.jsonl]
enabled = true
path = "/var/log/oisp/events.jsonl"
```

Then start sensor:

```bash
sudo oisp-sensor record --config /etc/oisp/config.toml
```

---

## How It Works

### Standard Python vs pyenv Python

**Standard installation:**
```
/usr/bin/python3 → links to system OpenSSL (libssl.so)
OISP Sensor auto-detects via library path
```

**pyenv installation:**
```
~/.pyenv/versions/3.11.0/bin/python3 → bundled OpenSSL
OISP Sensor needs explicit binary path
```

### OISP Sensor Attachment

When you specify `--binary-path`:

1. **Binary discovery** - Sensor locates the pyenv Python binary
2. **Symbol resolution** - Finds OpenSSL symbols (_ssl.so module)
3. **eBPF attachment** - Attaches uprobes to Python's SSL module
4. **SSL capture** - Intercepts TLS traffic from pyenv Python

### Verification

Check sensor logs to verify attachment:

```bash
sudo journalctl -u oisp-sensor | grep "Attached uprobe"
# Should show: Attached uprobe to /home/user/.pyenv/.../python3.11
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│  User Space                                         │
│                                                     │
│  ┌──────────────┐       ┌──────────────┐           │
│  │ pyenv Python │       │ OISP Sensor  │           │
│  │ (~/.pyenv/)  │       │              │           │
│  │              │       │ Config:      │           │
│  │ openai SDK   │       │ binary_paths │           │
│  │    │         │       │    │         │           │
│  │    ▼         │       │    ▼         │           │
│  │ _ssl.so      │◄──────┤ eBPF uprobes │           │
│  │ (bundled     │ SSL   │ (SSL_read/   │           │
│  │  OpenSSL)    │ calls │  SSL_write)  │           │
│  └──────────────┘       └──────────────┘           │
│         │                       │                   │
└─────────┼───────────────────────┼───────────────────┘
          │                       │
          ▼                       ▼
    ┌──────────┐            ┌─────────────┐
    │ OpenAI   │            │ events.jsonl│
    │ API      │            └─────────────┘
    └──────────┘
```

---

## Code Walkthrough

**`app.py`:**

```python
import openai

client = openai.OpenAI()

response = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Say hello!"}]
)

print("Response:", response.choices[0].message.content)
print("Tokens:", response.usage.total_tokens)
```

**What OISP Sensor Sees:**

1. **Process start** - `python3 app.py` starts
2. **SSL module load** - Python loads _ssl.so (bundled OpenSSL)
3. **SSL handshake** - TLS connection to api.openai.com
4. **Request** - JSON payload with messages
5. **Response** - Streamed completion
6. **Event export** - ai.request + ai.response events

---

## Validation

The cookbook includes automated validation:

```bash
./validate.sh
```

**Checks:**
- ✅ Events file exists
- ✅ Contains ai.request event
- ✅ Contains ai.response event
- ✅ Process name is "python" or "python3"
- ✅ Provider is "OpenAI"
- ✅ Binary path contains ".pyenv"

---

## Troubleshooting

### No events captured

**1. Verify binary path:**
```bash
which python
# Should show: /home/user/.pyenv/versions/3.11.0/bin/python

pyenv which python
# Alternative way to get path
```

**2. Check sensor attached:**
```bash
sudo journalctl -u oisp-sensor | grep "Attached uprobe"
```

**3. Try with debug logging:**
```bash
RUST_LOG=debug sudo oisp-sensor --binary-path $(which python)
```

### Wrong Python version

If you have multiple pyenv versions, ensure you're monitoring the active one:

```bash
pyenv version  # Shows active version
pyenv versions # Shows all installed versions
```

### SSL module not found

Verify Python was built with SSL support:

```python
python -c "import ssl; print(ssl.OPENSSL_VERSION)"
# Should print OpenSSL version, e.g., "OpenSSL 1.1.1w"
```

If SSL module is missing, rebuild Python with SSL:

```bash
# Install OpenSSL development headers first
sudo apt-get install libssl-dev  # Ubuntu/Debian
# or
sudo dnf install openssl-devel   # RHEL/Fedora

# Rebuild Python
pyenv install 3.11.0 --force
```

### Wildcard pattern not working

The wildcard pattern (`~/.pyenv/versions/*/bin/python*`) may not expand correctly. Use explicit path instead:

```bash
sudo oisp-sensor --binary-path ~/.pyenv/versions/3.11.0/bin/python
```

---

## Common pyenv Scenarios

### Global Python version

```bash
# Set global Python version
pyenv global 3.11.0

# OISP config
[capture.ssl]
binary_paths = ["~/.pyenv/versions/3.11.0/bin/python3.11"]
```

### Project-specific Python version

```bash
# In project directory
echo "3.11.0" > .python-version

# OISP config (monitor all versions)
[capture.ssl]
binary_paths = ["~/.pyenv/versions/*/bin/python*"]
```

### Virtual environments

pyenv virtual environments still use the base Python binary:

```bash
pyenv virtualenv 3.11.0 myproject
pyenv activate myproject

# Binary path is still
~/.pyenv/versions/3.11.0/bin/python3.11
```

---

## Related Examples

- **[NVM Node.js](../nvm-node/)** - Similar issue with NVM-managed Node.js
- **[Python + OpenAI Simple](../../python/openai-simple/)** - Standard Python installation
- **[Linux Troubleshooting](/platforms/linux/troubleshooting/#edge-cases-nvmpyenv)** - NVM/pyenv troubleshooting

---

## Production Recommendations

**For development:**
- Use explicit binary path in config
- Monitor all pyenv versions with wildcard

**For production:**
- Use system Python (not pyenv) for better detectability
- If pyenv required, pin version and use explicit path
- Add binary path to systemd service config

**Example systemd drop-in:**

```bash
sudo systemctl edit oisp-sensor
```

Add:

```ini
[Service]
ExecStart=
ExecStart=/usr/bin/oisp-sensor record \
  --output /var/log/oisp/events.jsonl \
  --binary-path /home/appuser/.pyenv/versions/3.11.0/bin/python3.11
```

**For containerized deployments:**

Use official Python Docker images instead of pyenv:

```dockerfile
# Good - uses system Python
FROM python:3.11-slim

# Avoid - adds pyenv complexity
FROM ubuntu:22.04
RUN curl https://pyenv.run | bash
```

---

## Multiple Python Versions

If your environment uses multiple Python versions:

```toml
[capture.ssl]
binary_paths = [
    "~/.pyenv/versions/3.11.0/bin/python3.11",
    "~/.pyenv/versions/3.12.0/bin/python3.12",
    "/usr/bin/python3",  # Also monitor system Python
]
```

Or use wildcard (expands at runtime):

```toml
[capture.ssl]
binary_paths = [
    "~/.pyenv/versions/*/bin/python*",
]
```

---

## Performance Considerations

**Impact of monitoring multiple binaries:**
- Each binary path adds one eBPF uprobe attachment
- Minimal overhead per attachment (<1% CPU)
- Safe to monitor 5-10 Python versions simultaneously

**Best practice:**
- Monitor only active versions in production
- Use wildcard for development environments

---

## Next Steps

- **[Linux Platform Guide](/platforms/linux/)** - Full Linux documentation
- **[Configuration](/configuration/config-file/)** - Config file reference
- **[All Cookbooks](/cookbooks/overview/)** - Browse all examples
