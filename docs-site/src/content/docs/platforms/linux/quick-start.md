---
title: Linux Quick Start
description: Get started with OISP Sensor on Linux in 5 minutes
---


See AI activity on your Linux machine in 5 minutes.

## Step 1: Install

```bash
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh
```

This will:
- Auto-detect your distribution
- Check system requirements
- Install OISP Sensor
- Configure systemd service

**Installation time:** ~30 seconds

---

## Step 2: Start the Sensor

### Option A: Terminal UI (TUI)

```bash
sudo oisp-sensor
```

You'll see a real-time dashboard:

```
┌─ OISP Sensor ─────────────────────────────────── v0.2.0 ────┐
│                                                              │
│  AI ACTIVITY (last 5 min)                                   │
│  ──────────────────────────────────────────────────────────│
│  (Waiting for AI events...)                                 │
│                                                              │
│  PROVIDERS          APPS USING AI          CONFIDENCE       │
│  ─────────────      ─────────────          ──────────       │
│  (none yet)                                                  │
│                                                              │
│  [t]imeline  [i]nventory  [p]rocess tree  [q]uit          │
└──────────────────────────────────────────────────────────────┘
```

**Keyboard shortcuts:**
- `t` - Timeline view
- `i` - Inventory (providers, apps)
- `p` - Process tree
- `q` - Quit

### Option B: Web UI

```bash
sudo oisp-sensor --web
```

Then open your browser to: **http://localhost:7777**

The Web UI shows:
- Real-time event stream
- Timeline view
- Process tree visualization
- Provider inventory
- Dashboard with stats

---

## Step 3: Generate AI Activity

In a **new terminal**, use any AI tool:

### Example 1: Python + OpenAI

```bash
# Install OpenAI SDK
pip install openai

# Run a simple chat
python3 << EOF
import openai
client = openai.OpenAI()
response = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Say hello!"}]
)
print(response.choices[0].message.content)
EOF
```

### Example 2: Curl to OpenAI API

```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Example 3: Use Cursor, Claude CLI, etc.

Just use your AI tools as normal. OISP Sensor will capture all activity automatically.

---

## Step 4: See the Events

Switch back to your sensor terminal/browser. You should see:

### In TUI:

```
┌─ OISP Sensor ─────────────────────────────────── v0.2.0 ────┐
│                                                              │
│  AI ACTIVITY (last 5 min)                                   │
│  ──────────────────────────────────────────────────────────│
│  14:32:15  ai.request   OpenAI gpt-4o-mini  python3 [FULL] │
│  14:32:16  ai.response  OpenAI gpt-4o-mini  python3 [FULL] │
│                                                              │
│  PROVIDERS          APPS USING AI          CONFIDENCE       │
│  ─────────────      ─────────────          ──────────       │
│  OpenAI       2     python3       2        Full: 100%       │
│                                                              │
│  [t]imeline  [i]nventory  [p]rocess tree  [q]uit          │
└──────────────────────────────────────────────────────────────┘
```

### In Web UI:

You'll see events appear in real-time with:
- Event type (`ai.request`, `ai.response`)
- Provider (OpenAI)
- Model (gpt-4o-mini)
- Process (python3)
- Full request/response content (if redaction mode = full)

---

## Step 5: Explore Event Data

### Export to File

Stop the sensor (Ctrl+C) and run with file output:

```bash
sudo oisp-sensor record --output /tmp/events.jsonl --no-ui
```

Generate some AI activity (see Step 3), then stop (Ctrl+C).

### Analyze Events

```bash
# Count event types
cat /tmp/events.jsonl | jq -r '.event_type' | sort | uniq -c

# View ai.request events
cat /tmp/events.jsonl | jq -r 'select(.event_type=="ai.request")'

# View ai.response events
cat /tmp/events.jsonl | jq -r 'select(.event_type=="ai.response")'

# See which providers were used
cat /tmp/events.jsonl | jq -r '.data.provider' | sort | uniq -c
```

### Use the `analyze` Command

```bash
oisp-sensor analyze /tmp/events.jsonl
```

This shows:
- Provider inventory (OpenAI, Anthropic, etc.)
- Model usage (gpt-4, claude-3, etc.)
- Cost estimates
- Top applications
- Timeline summary

---

## Common Patterns

### Monitor Development Environment

```bash
# Start sensor in background
sudo oisp-sensor record --output /var/log/oisp/dev.jsonl &

# Use Cursor, VS Code, Claude CLI, etc.
# All AI activity is captured automatically

# Later, analyze the log
oisp-sensor analyze /var/log/oisp/dev.jsonl
```

### Track Specific Process

```bash
# Monitor only Python processes
sudo oisp-sensor --comm python3

# Monitor specific PID
sudo oisp-sensor --pid 12345
```

### Export to Production Monitoring

```bash
# Export to OTLP (OpenTelemetry)
sudo oisp-sensor record --export otlp --otlp-endpoint http://collector:4317

# Or to Kafka
sudo oisp-sensor record --export kafka --kafka-brokers kafka1:9092
```

---

## What's Next?

### Run as a Service

For persistent monitoring, run OISP Sensor as a systemd service:

```bash
sudo systemctl enable oisp-sensor
sudo systemctl start oisp-sensor
sudo systemctl status oisp-sensor
```

See [Running as a Service](./service) for details.

### Production Deployment

For production environments:

1. **[Production Guide](./production)** - System requirements, security, monitoring
2. **[Configuration](../../configuration/config-file/)** - TOML config reference
3. **[Redaction & Privacy](../../configuration/redaction/)** - Safe mode, custom patterns

### Explore Cookbooks

Ready-to-run examples:

- **[Python + OpenAI](../../cookbooks/python/openai-simple/)** - Simple chat completion
- **[Python + LangChain](../../cookbooks/python/langchain-agent/)** - Agent with tools
- **[Node.js + OpenAI](../../cookbooks/node/openai-simple/)** - TypeScript chat app
- **[All Cookbooks](../../cookbooks/overview/)** - Browse all examples

---

## Troubleshooting

### No events appearing?

1. **Check you have API keys set:**
   ```bash
   echo $OPENAI_API_KEY  # Should not be empty
   ```

2. **Check sensor is capturing SSL:**
   ```bash
   sudo journalctl -u oisp-sensor -f
   # Look for "Attached uprobe" messages
   ```

3. **Try with debug logging:**
   ```bash
   RUST_LOG=debug sudo oisp-sensor
   ```

4. **Verify system requirements:**
   ```bash
   oisp-sensor check
   # All checks should show [OK]
   ```

See [Troubleshooting Guide](./troubleshooting) for more help.

---

**You're now capturing AI activity! Explore the docs to learn more.**
