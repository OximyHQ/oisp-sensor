---
title: "n8n Self-Hosted"
description: "Capture AI activity from self-hosted n8n workflows"
---

# n8n Self-Hosted

This quickstart shows you how to capture AI activity from self-hosted n8n, including OpenAI, Anthropic, and other AI nodes.

## What You'll Capture

- AI requests from n8n AI nodes (OpenAI, Anthropic, etc.)
- Model and provider information
- Token usage for cost tracking
- Workflow execution context (via process attribution)

## Time to Complete

**15-20 minutes**

## Prerequisites

- Linux with kernel 5.0+ and Docker
- OpenAI API key (or other AI provider key)
- Root access

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Host Machine                      │
│                                                      │
│  ┌──────────────┐    ┌──────────────────────────┐   │
│  │ OISP Sensor  │    │   n8n Container          │   │
│  │ (privileged) │───▶│   - Node.js runtime      │   │
│  │              │    │   - AI node executions   │   │
│  │ Captures TLS │    │   - HTTPS to AI APIs     │   │
│  │ from all     │    └──────────────────────────┘   │
│  │ processes    │                                   │
│  └──────────────┘                                   │
└─────────────────────────────────────────────────────┘
```

## Option A: Docker Compose (Recommended)

### Step 1: Create docker-compose.yml

```yaml
version: '3.8'

services:
  n8n:
    image: n8nio/n8n:latest
    restart: unless-stopped
    ports:
      - "5678:5678"
    environment:
      - N8N_BASIC_AUTH_ACTIVE=true
      - N8N_BASIC_AUTH_USER=admin
      - N8N_BASIC_AUTH_PASSWORD=changeme
      - N8N_HOST=localhost
      - N8N_PORT=5678
      - N8N_PROTOCOL=http
      - WEBHOOK_URL=http://localhost:5678/
      # Add your AI provider keys
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - n8n_data:/home/node/.n8n

  oisp-sensor:
    image: ghcr.io/oximyhq/oisp-sensor:latest
    privileged: true
    pid: host
    network_mode: host
    volumes:
      - /sys:/sys:ro
      - /usr:/usr:ro
      - /lib:/lib:ro
      - /lib64:/lib64:ro
      - /proc:/proc:ro
      - ./output:/output
    command: >
      --output /output/n8n-ai-events.jsonl
      --no-ui
    restart: unless-stopped

volumes:
  n8n_data:
```

### Step 2: Start the Stack

```bash
# Set your API key
export OPENAI_API_KEY="sk-..."

# Start everything
docker-compose up -d

# Check sensor is running
docker-compose logs oisp-sensor
```

### Step 3: Create an AI Workflow in n8n

1. Open n8n at `http://localhost:5678`
2. Login with admin/changeme
3. Create a new workflow
4. Add nodes:
   - **Manual Trigger** (to start manually)
   - **OpenAI** node (Chat completion)
     - Model: gpt-4o-mini
     - Prompt: "What is the capital of France?"

5. Connect: Manual Trigger → OpenAI
6. Save the workflow

### Step 4: Execute the Workflow

Click "Execute Workflow" in n8n.

### Step 5: View Captured Events

```bash
# View captured events
cat ./output/n8n-ai-events.jsonl | jq .

# Filter to just AI events
cat ./output/n8n-ai-events.jsonl | jq 'select(.event_type | startswith("ai."))'
```

### Expected Output

```json
{
  "oisp_version": "0.1",
  "event_type": "ai.request",
  "ts": "2025-12-24T10:30:00.123Z",
  "process": {
    "pid": 1234,
    "exe": "/usr/local/bin/node",
    "cmdline": "node /usr/local/lib/node_modules/n8n/..."
  },
  "data": {
    "provider": {
      "name": "openai",
      "endpoint": "https://api.openai.com/v1/chat/completions"
    },
    "model": {
      "id": "gpt-4o-mini"
    },
    "streaming": false,
    "messages_count": 1
  }
}
```

---

## Option B: Sensor on Host, n8n in Docker

If you prefer to run the sensor directly on the host:

### Step 1: Install OISP Sensor on Host

```bash
curl -fsSL https://sensor.oisp.dev/install.sh | sudo sh
```

### Step 2: Start n8n

```bash
docker run -d \
  --name n8n \
  -p 5678:5678 \
  -e OPENAI_API_KEY="$OPENAI_API_KEY" \
  -v n8n_data:/home/node/.n8n \
  n8nio/n8n
```

### Step 3: Start Sensor

```bash
# The sensor on the host can see inside containers
sudo oisp-sensor --output /tmp/n8n-events.jsonl
```

### Step 4: Trigger Workflow and Check

Same as above - create workflow, execute, check output.

---

## Capturing Different AI Nodes

n8n supports multiple AI providers. Here's how each is captured:

| n8n Node | Provider Detected | Full Parsing |
|----------|------------------|:------------:|
| OpenAI | openai | **Yes** |
| OpenAI Chat Model | openai | **Yes** |
| Anthropic | anthropic | **Yes** |
| Azure OpenAI | azure_openai | **Yes** |
| Google Gemini | google | **Yes** |
| Hugging Face | huggingface | Basic |
| Ollama | ollama | **Yes** |
| Groq | groq | Basic |

## Correlating Events to Workflows

Currently, we capture the process information but not the specific n8n workflow ID. To correlate:

1. **By timestamp**: Match event timestamps to workflow execution times
2. **By content**: Look at the prompt content (if not redacted)
3. **Future**: We plan to add correlation via n8n's execution ID

## Monitoring Multiple Workflows

The sensor captures ALL AI calls from the n8n container. If you have multiple workflows:

```bash
# Count AI calls per minute
cat ./output/n8n-ai-events.jsonl | \
  jq -r '.ts[:16]' | \
  sort | uniq -c

# Group by model
cat ./output/n8n-ai-events.jsonl | \
  jq -r 'select(.event_type == "ai.request") | .data.model.id' | \
  sort | uniq -c
```

## Token Usage Tracking

Extract token usage for cost calculation:

```bash
# Sum total tokens
cat ./output/n8n-ai-events.jsonl | \
  jq -s '[.[] | select(.event_type == "ai.response") | .data.usage.total_tokens // 0] | add'

# Tokens by model
cat ./output/n8n-ai-events.jsonl | \
  jq -r 'select(.event_type == "ai.response") | 
    "\(.data.model.id // "unknown"): \(.data.usage.total_tokens // 0)"' | \
  sort
```

## Troubleshooting

### No events captured from n8n container

1. **Check privileged mode**: The sensor container must be `privileged: true`

2. **Check network mode**: Use `network_mode: host` for the sensor

3. **Check host mounts**: These are required:
   ```yaml
   volumes:
     - /sys:/sys:ro
     - /usr:/usr:ro
     - /lib:/lib:ro
     - /proc:/proc:ro
   ```

4. **Check sensor logs**:
   ```bash
   docker-compose logs oisp-sensor
   ```

### n8n uses bundled Node.js with static OpenSSL

If n8n's container uses a Node.js with static OpenSSL, you may need additional configuration. Check:

```bash
# Enter the n8n container
docker exec -it n8n sh

# Check Node.js SSL
node -e "console.log(process.versions.openssl)"

# Check linked libraries
ldd $(which node) | grep ssl
```

If `ldd` shows no SSL library, Node.js has static SSL and we need to add the binary path.

### Events captured but missing response data

This can happen with streaming responses. n8n typically uses non-streaming for simplicity, but check:

```bash
# Check if requests are streaming
cat ./output/n8n-ai-events.jsonl | \
  jq 'select(.event_type == "ai.request") | .data.streaming'
```

## Production Deployment

For production n8n deployments:

### 1. Persistent Output

Mount a persistent volume for events:

```yaml
oisp-sensor:
  volumes:
    - /var/log/oisp-sensor:/output
```

### 2. Log Rotation

Add log rotation config:

```toml
# oisp-sensor config
[export.jsonl]
path = "/output/events.jsonl"
rotate = "daily"
max_size = "100MB"
max_files = 7
```

### 3. Send to External System

Forward events to your observability stack:

```yaml
oisp-sensor:
  command: >
    --output /output/events.jsonl
    --export otlp
    --otlp-endpoint http://otel-collector:4317
```

## Next Steps

- [Monitor LiteLLM proxy](/quickstarts/litellm-proxy)
- [Deploy on Kubernetes](/quickstarts/kubernetes-deployment)
- [Token cost calculation](/guides/cost-tracking)

