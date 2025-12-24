---
title: "Python + OpenAI (Simple Script)"
description: "Capture AI activity from a simple Python script calling OpenAI"
---

# Python + OpenAI (Simple Script)

This quickstart shows you how to capture AI activity from a basic Python script that calls the OpenAI API.

## What You'll Capture

- `ai.request` events with model, messages, and parameters
- `ai.response` events with completions, token usage, and latency
- Process attribution (which Python process made the call)

## Time to Complete

**5-10 minutes**

## Prerequisites

- Linux with kernel 5.0+ (`uname -r` to check)
- Python 3.8+
- OpenAI API key
- Root access (for eBPF)

## Step 1: Install OISP Sensor

```bash
# One-line install
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh

# Or download directly
wget https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-linux-x86_64
chmod +x oisp-sensor-linux-x86_64
sudo mv oisp-sensor-linux-x86_64 /usr/local/bin/oisp-sensor
```

Verify installation:
```bash
oisp-sensor --version
# oisp-sensor 0.1.0
```

## Step 2: Create Sample Python Script

Create `test_openai.py`:

```python
#!/usr/bin/env python3
"""Simple OpenAI API test for OISP Sensor capture validation."""

import os
from openai import OpenAI

def main():
    client = OpenAI(api_key=os.environ.get("OPENAI_API_KEY"))
    
    print("Sending request to OpenAI...")
    
    response = client.chat.completions.create(
        model="gpt-4o-mini",
        messages=[
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "What is 2 + 2? Reply in one word."}
        ],
        max_tokens=10,
        temperature=0.0
    )
    
    print(f"Response: {response.choices[0].message.content}")
    print(f"Tokens used: {response.usage.total_tokens}")

if __name__ == "__main__":
    main()
```

Install the OpenAI library:
```bash
pip install openai
```

## Step 3: Start OISP Sensor

Start the sensor as a background daemon:

```bash
# Start the daemon (runs in background)
sudo oisp-sensor daemon start

# Check it's running
oisp-sensor daemon status
```

You should see:
```
OISP Sensor daemon
Status: running
PID: 12345
Output: /var/log/oisp-sensor/events.jsonl
```

Alternatively, run in foreground for debugging:
```bash
# Foreground mode (blocks terminal)
sudo oisp-sensor --output /tmp/ai-events.jsonl

# Or filter to just python processes
sudo oisp-sensor --comm python3 --output /tmp/ai-events.jsonl
```

## Step 4: Run Your Script

Now run your Python script:

```bash
export OPENAI_API_KEY="sk-..."
python3 test_openai.py
```

## Step 5: View Captured Events

Check the captured events:

```bash
# If using daemon mode (default location)
cat /var/log/oisp-sensor/events.jsonl | jq .

# Or if you specified a custom output
cat /tmp/ai-events.jsonl | jq .

# Follow events in real-time
oisp-sensor daemon logs --follow
```

### Expected ai.request Event

```json
{
  "oisp_version": "0.1",
  "event_id": "01JGXYZ...",
  "event_type": "ai.request",
  "ts": "2025-12-24T10:30:00.123Z",
  "process": {
    "pid": 12345,
    "exe": "/usr/bin/python3",
    "cmdline": "python3 test_openai.py"
  },
  "data": {
    "provider": {
      "name": "openai",
      "endpoint": "https://api.openai.com/v1/chat/completions"
    },
    "model": {
      "id": "gpt-4o-mini",
      "family": "gpt-4"
    },
    "request_type": "chat",
    "streaming": false,
    "messages_count": 2,
    "has_system_prompt": true,
    "parameters": {
      "temperature": 0.0,
      "max_tokens": 10
    }
  }
}
```

### Expected ai.response Event

```json
{
  "oisp_version": "0.1",
  "event_id": "01JGXYZ...",
  "event_type": "ai.response",
  "ts": "2025-12-24T10:30:01.456Z",
  "process": {
    "pid": 12345,
    "exe": "/usr/bin/python3"
  },
  "data": {
    "request_id": "01JGXYZ...",
    "provider": {
      "name": "openai"
    },
    "model": {
      "id": "gpt-4o-mini"
    },
    "success": true,
    "finish_reason": "stop",
    "usage": {
      "prompt_tokens": 25,
      "completion_tokens": 2,
      "total_tokens": 27
    }
  }
}
```

## Troubleshooting

### No events captured

1. **Check kernel version:**
   ```bash
   uname -r
   # Need 5.0 or higher
   ```

2. **Check SSL library is probed:**
   ```bash
   # Find which SSL your Python uses
   ldd $(which python3) | grep ssl
   # Should show: libssl.so.3 => /lib/x86_64-linux-gnu/libssl.so.3
   ```

3. **Run sensor with debug logging:**
   ```bash
   RUST_LOG=debug sudo oisp-sensor
   ```

### Using pyenv or conda Python

If you use pyenv/conda, the SSL library might not be in standard paths:

```bash
# Find your Python's SSL
python3 -c "import ssl; print(ssl.OPENSSL_VERSION)"
ldd $(python3 -c "import sys; print(sys.executable)") | grep ssl
```

Add to config file (`~/.config/oisp-sensor/config.toml`):
```toml
[capture.ssl]
binary_paths = [
    "~/.pyenv/versions/*/lib/libssl.so*",
    "~/miniconda3/lib/libssl.so*",
]
```

### Partial or missing response data

If you see the request but not the response:
- The response might be streaming (SSE) - we capture these too
- Check if the API call actually succeeded
- Large responses may take longer to process

## Next Steps

- [Capture LangChain applications](/quickstarts/python-langchain)
- [Monitor multiple Python processes](/quickstarts/multi-process-python)
- [Deploy with Docker](/quickstarts/docker-deployment)

## Validation Script

Save as `validate.sh`:

```bash
#!/bin/bash
# Validate OISP Sensor captured the OpenAI call

OUTPUT_FILE="/tmp/ai-events.jsonl"

# Check for ai.request
if grep -q '"event_type":"ai.request"' "$OUTPUT_FILE"; then
    echo "[OK] ai.request event captured"
else
    echo "[FAIL] No ai.request event found"
    exit 1
fi

# Check for ai.response
if grep -q '"event_type":"ai.response"' "$OUTPUT_FILE"; then
    echo "[OK] ai.response event captured"
else
    echo "[FAIL] No ai.response event found"
    exit 1
fi

# Check for OpenAI provider
if grep -q '"name":"openai"' "$OUTPUT_FILE"; then
    echo "[OK] OpenAI provider detected"
else
    echo "[FAIL] OpenAI provider not detected"
    exit 1
fi

echo ""
echo "All validations passed!"
```

