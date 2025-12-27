---
title: NVM Node.js
description: Capture AI activity from NVM-managed Node.js installations
---

# NVM Node.js

Capture AI activity from Node.js installed via NVM (Node Version Manager).

## Overview

**What this demonstrates:**
- Handling NVM-managed Node.js binaries
- Custom binary path configuration
- SSL capture from non-standard installations

**Repository:** `oisp-cookbook/edge-cases/nvm-node`

---

## The Challenge

**Problem:** NVM installs Node.js in user home directories (`~/.nvm/versions/node/...`) with statically linked OpenSSL, making automatic detection difficult.

**Solution:** Explicitly configure OISP Sensor to monitor NVM binaries.

---

## Prerequisites

- Linux with eBPF support (kernel 5.8+)
- NVM installed
- Node.js installed via NVM
- OpenAI API key

---

## Running the Example

### With Docker Compose (Recommended)

```bash
cd oisp-cookbook/edge-cases/nvm-node
docker-compose up
```

**What happens:**
1. Container installs NVM and Node.js v20
2. OISP Sensor starts with custom binary path
3. Node.js script makes OpenAI API call
4. Events captured and validated

### Without Docker

**1. Find your NVM Node.js binary:**

```bash
which node
# Example output: /home/user/.nvm/versions/node/v20.10.0/bin/node
```

**2. Start OISP Sensor with binary path:**

```bash
sudo oisp-sensor record \
  --output /tmp/nvm-events.jsonl \
  --binary-path $(which node)
```

**3. In another terminal, run the example:**

```bash
cd oisp-cookbook/edge-cases/nvm-node
npm install
export OPENAI_API_KEY="your-api-key"
node app.js
```

**4. Stop sensor (Ctrl+C) and validate:**

```bash
cat /tmp/nvm-events.jsonl | jq -r '.event_type' | sort | uniq -c
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
    "comm": "node",
    "exe": "/home/user/.nvm/versions/node/v20.10.0/bin/node"
  }
}
```

---

## Configuration

### Option 1: Command Line

```bash
sudo oisp-sensor --binary-path ~/.nvm/versions/node/v20.10.0/bin/node
```

### Option 2: Config File

**`/etc/oisp/config.toml`:**

```toml
[sensor]
name = "nvm-dev-machine"

[capture]
ssl = true
process = true

[capture.ssl]
# Monitor all NVM Node.js versions
binary_paths = [
    "~/.nvm/versions/node/*/bin/node",
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

### Standard Node.js vs NVM Node.js

**Standard installation:**
```
/usr/bin/node → links to system OpenSSL
OISP Sensor auto-detects via library path
```

**NVM installation:**
```
~/.nvm/versions/node/v20.10.0/bin/node → statically linked OpenSSL
OISP Sensor needs explicit binary path
```

### OISP Sensor Attachment

When you specify `--binary-path`:

1. **Binary discovery** - Sensor locates the NVM Node.js binary
2. **Symbol resolution** - Finds OpenSSL symbols (SSL_read, SSL_write)
3. **eBPF attachment** - Attaches uprobes to the binary
4. **SSL capture** - Intercepts TLS traffic from NVM Node.js

### Verification

Check sensor logs to verify attachment:

```bash
sudo journalctl -u oisp-sensor | grep "Attached uprobe"
# Should show: Attached uprobe to /home/user/.nvm/.../node
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│  User Space                                         │
│                                                     │
│  ┌──────────────┐       ┌──────────────┐           │
│  │ NVM Node.js  │       │ OISP Sensor  │           │
│  │ (~/.nvm/...) │       │              │           │
│  │              │       │ Config:      │           │
│  │ OpenAI SDK   │       │ binary_paths │           │
│  │    │         │       │    │         │           │
│  │    ▼         │       │    ▼         │           │
│  │ Statically   │◄──────┤ eBPF uprobes │           │
│  │ linked       │ SSL   │ (SSL_read/   │           │
│  │ OpenSSL      │ calls │  SSL_write)  │           │
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

**`app.js`:**

```javascript
import OpenAI from 'openai';

const client = new OpenAI();

async function main() {
  const response = await client.chat.completions.create({
    model: 'gpt-4o-mini',
    messages: [{ role: 'user', content: 'Say hello!' }],
  });

  console.log('Response:', response.choices[0].message.content);
  console.log('Tokens:', response.usage.total_tokens);
}

main();
```

**What OISP Sensor Sees:**

1. **Process start** - `node app.js` starts
2. **SSL handshake** - TLS connection to api.openai.com
3. **Request** - JSON payload with messages
4. **Response** - Streamed completion
5. **Event export** - ai.request + ai.response events

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
- ✅ Process name is "node"
- ✅ Provider is "OpenAI"
- ✅ Binary path contains ".nvm"

---

## Troubleshooting

### No events captured

**1. Verify binary path:**
```bash
which node
# Should show: /home/user/.nvm/versions/node/v20.10.0/bin/node
```

**2. Check sensor attached:**
```bash
sudo journalctl -u oisp-sensor | grep "Attached uprobe"
```

**3. Try with debug logging:**
```bash
RUST_LOG=debug sudo oisp-sensor --binary-path $(which node)
```

### Wrong Node.js version

If you have multiple NVM versions, ensure you're monitoring the active one:

```bash
nvm current  # Shows active version
which node   # Confirms binary path
```

### Wildcard pattern not working

The wildcard pattern (`~/.nvm/versions/node/*/bin/node`) may not expand correctly in some shells. Use explicit path instead:

```bash
sudo oisp-sensor --binary-path ~/.nvm/versions/node/v20.10.0/bin/node
```

---

## Related Examples

- **[pyenv Python](../pyenv-python/)** - Similar issue with pyenv-managed Python
- **[Node.js + OpenAI Simple](../../node/openai-simple/)** - Standard Node.js installation
- **[Linux Troubleshooting](/platforms/linux/troubleshooting/#edge-cases-nvmpyenv)** - NVM/pyenv troubleshooting

---

## Production Recommendations

**For development:**
- Use explicit binary path in config
- Monitor all NVM versions with wildcard

**For production:**
- Use system Node.js (not NVM) for better detectability
- If NVM required, pin version and use explicit path
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
  --binary-path /home/appuser/.nvm/versions/node/v20.10.0/bin/node
```

---

## Next Steps

- **[Linux Platform Guide](/platforms/linux/)** - Full Linux documentation
- **[Configuration](/configuration/config-file/)** - Config file reference
- **[All Cookbooks](/cookbooks/overview/)** - Browse all examples
