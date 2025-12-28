---
title: Quick Start
description: Capture your first AI events in 5 minutes
---

import { Steps } from '@astrojs/starlight/components';
import { Aside } from '@astrojs/starlight/components';
import { Tabs, TabItem } from '@astrojs/starlight/components';

This guide will have you capturing AI events in under 5 minutes.

<Tabs>
  <TabItem label="Linux">

## Prerequisites

- OISP Sensor [installed](/getting-started/installation)
- Linux with kernel 5.8+ (for eBPF)
- An AI application to monitor (or use our demo)

## Start Capturing

<Steps>

1. **Start the sensor**

   ```bash
   sudo oisp-sensor record
   ```

   You should see:
   ```
   OISP Sensor v0.2.0
   Starting capture...
   eBPF capture started
   Web UI: http://localhost:7777
   ```

2. **Open the Web UI**

   Navigate to [http://localhost:7777](http://localhost:7777) in your browser.

   You'll see the real-time dashboard:
   - Event timeline
   - Process tree
   - AI provider breakdown
   - Token usage statistics

3. **Generate some AI activity**

   In another terminal, run any AI application. For example:

   ```bash
   # Python with OpenAI
   python -c "
   import openai
   client = openai.OpenAI()
   response = client.chat.completions.create(
       model='gpt-4',
       messages=[{'role': 'user', 'content': 'Hello!'}]
   )
   print(response.choices[0].message.content)
   "
   ```

   Or with curl:
   ```bash
   curl https://api.openai.com/v1/chat/completions \
     -H "Authorization: Bearer $OPENAI_API_KEY" \
     -H "Content-Type: application/json" \
     -d '{
       "model": "gpt-4",
       "messages": [{"role": "user", "content": "Say hello"}]
     }'
   ```

4. **View captured events**

   The Web UI will update in real-time showing:
   - The HTTP request to OpenAI
   - The AI request with model and messages
   - The AI response with tokens and latency
   - Process information for the calling application

</Steps>

## Try the Demo

If you don't have an AI application handy, use the built-in demo:

```bash
oisp-sensor demo
```

This generates synthetic AI events so you can explore the UI.

## Common Options

### Filter by Process

Only capture events from specific processes:

```bash
# By process name
sudo oisp-sensor record --process python,node

# By PID
sudo oisp-sensor record --pid 12345
```

### Save to File

Export events to JSONL:

```bash
sudo oisp-sensor record --output events.jsonl
```

### Change Web UI Port

```bash
sudo oisp-sensor record --port 8080
```

### Disable Web UI

Run headless:

```bash
sudo oisp-sensor record --no-web --output events.jsonl
```

  </TabItem>
  <TabItem label="Windows">

## Prerequisites

- OISP Sensor [installed](/getting-started/installation)
- Windows 10/11 (64-bit)
- Administrator privileges for packet capture
- CA certificate installed for HTTPS interception

## Start Capturing

<Steps>

1. **Launch OISP**

   Double-click `OISPApp.exe` or launch from the Start Menu.

2. **Install CA Certificate**

   Right-click the tray icon → **"Install CA Certificate"**

3. **Start Capture**

   Right-click the tray icon → **"Start Capture"**

   Accept the UAC prompt (Administrator required).

4. **Generate some AI activity**

   In PowerShell:

   ```powershell
   python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hello'}]).choices[0].message.content)"
   ```

5. **View captured events**

   Right-click tray icon → **"View Logs"**

   Or check `%USERPROFILE%\Documents\OISP\events.jsonl`

</Steps>

### Command Line Usage

```powershell
# Terminal 1: Start sensor
.\oisp-sensor.exe record --output events.jsonl

# Terminal 2 (Administrator): Start redirector
.\oisp-redirector.exe --tls-mitm
```

  </TabItem>
  <TabItem label="macOS">

## Prerequisites

- macOS 13+ (Ventura or later)
- Apple Developer Program ($99/year) for System Extension signing
- Admin access for extension approval

## Start Capturing

<Steps>

1. **Build the sensor**

   ```bash
   cargo build --release
   ```

2. **Build the macOS app**

   ```bash
   cd macos
   xcodegen generate
   xcodebuild -project OISP.xcodeproj -scheme OISP build
   ```

3. **Start sensor with JSONL export**

   ```bash
   ./target/release/oisp-sensor record --output ~/oisp-events.jsonl --web
   ```

4. **Launch the menu bar app and approve the Network Extension**

5. **Trust the OISP CA certificate when prompted**

6. **Generate AI activity**

   ```bash
   python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4', messages=[{'role':'user','content':'Hello'}]))"
   ```

</Steps>

  </TabItem>
</Tabs>

## Example Output

Here's what a captured AI request looks like:

```json
{
  "oisp_version": "0.1",
  "event_id": "01HXK...",
  "event_type": "ai.request",
  "ts": "2024-12-23T10:30:00Z",
  "process": {
    "pid": 12345,
    "name": "python",
    "exe": "/usr/bin/python3"
  },
  "data": {
    "request_id": "req_abc123",
    "provider": {
      "name": "openai",
      "endpoint": "https://api.openai.com/v1/chat/completions"
    },
    "model": {
      "id": "gpt-4",
      "family": "gpt"
    },
    "request_type": "completion",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "estimated_tokens": 50
  }
}
```

And the corresponding response:

```json
{
  "event_type": "ai.response",
  "data": {
    "request_id": "req_abc123",
    "status_code": 200,
    "success": true,
    "usage": {
      "prompt_tokens": 10,
      "completion_tokens": 25,
      "total_tokens": 35
    },
    "latency_ms": 1250,
    "finish_reason": "stop"
  }
}
```

## What's Next?

<Aside type="tip">
Want to see what providers and models are being used? Check out the Analytics view in the Web UI.
</Aside>

- [Configure exports](/configuration/exports) - Send events to OTLP, Kafka, etc.
- [Set up redaction](/configuration/redaction) - Protect sensitive data
- [Understand the architecture](/architecture/overview) - Learn how OISP Sensor works
- [CLI Reference](/reference/cli) - All available commands and options

