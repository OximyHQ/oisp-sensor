---
title: Python + OpenAI Simple
description: Basic chat completion with OpenAI and OISP Sensor
---


The simplest example - capture events from OpenAI API calls.

## Overview

This cookbook demonstrates:
- Making a basic chat completion request to OpenAI
- Capturing AI request and response events
- Viewing and validating captured data

**Complexity:** ⭐ Beginner
**Time:** 5 minutes

## Repository

```bash
git clone https://github.com/oximyhq/oisp-cookbook.git
cd oisp-cookbook/python/01-openai-simple
```

## Files

```
python/01-openai-simple/
├── app.py                 # Simple OpenAI chat
├── requirements.txt       # Dependencies (openai)
├── docker-compose.yml     # Run with sensor
├── expected-events.json   # Validation rules
└── README.md              # Instructions
```

## Application Code

**app.py:**

```python
import openai

client = openai.OpenAI()

response = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[
        {"role": "user", "content": "Say hello in a creative way!"}
    ]
)

print(response.choices[0].message.content)
print(f"Tokens used: {response.usage.total_tokens}")
```

## Running

### With Docker Compose

```bash
# Set your API key
export OPENAI_API_KEY=sk-...

# Run
docker-compose up
```

**What happens:**
1. OISP Sensor starts and loads eBPF programs
2. Python app runs and makes OpenAI API call
3. Sensor captures SSL traffic and extracts events
4. Events written to `output/events.jsonl`

### Without Docker

```bash
# Terminal 1: Start sensor
sudo oisp-sensor record --output output/events.jsonl

# Terminal 2: Run app
pip install openai
export OPENAI_API_KEY=sk-...
python app.py
```

## Expected Output

### Application Output

```
Hello there! 👋 I hope your day is filled with sunshine and laughter!
Tokens used: 25
```

### Captured Events

```bash
cat output/events.jsonl | jq -r '.event_type' | sort | uniq -c
```

**Expected:**

```
  1 ai.request
  1 ai.response
```

### Event Details

**ai.request event:**

```json
{
  "event_type": "ai.request",
  "ts": "2024-12-26T14:32:15.123456Z",
  "process": {
    "pid": 12345,
    "exe": "/usr/bin/python3",
    "cmdline": "python3 app.py"
  },
  "data": {
    "provider": "OpenAI",
    "model": "gpt-4o-mini",
    "messages": [
      {
        "role": "user",
        "content": "Say hello in a creative way!"
      }
    ],
    "temperature": null,
    "max_tokens": null
  }
}
```

**ai.response event:**

```json
{
  "event_type": "ai.response",
  "ts": "2024-12-26T14:32:16.456789Z",
  "process": {
    "pid": 12345,
    "exe": "/usr/bin/python3"
  },
  "data": {
    "provider": "OpenAI",
    "model": "gpt-4o-mini",
    "choices": [
      {
        "message": {
          "role": "assistant",
          "content": "Hello there! 👋 I hope your day is filled..."
        }
      }
    ],
    "usage": {
      "prompt_tokens": 15,
      "completion_tokens": 18,
      "total_tokens": 33
    }
  }
}
```

## Validation

```bash
# Validate events match expected schema
python ../shared/scripts/validate.py \
  --events output/events.jsonl \
  --expected expected-events.json
```

**Expected output:**

```
✅ Found 2 events
✅ Event types match: ['ai.request', 'ai.response']
✅ Provider detected: OpenAI
✅ Model detected: gpt-4o-mini
✅ All validations passed
```

## What Gets Captured

| Event | What's Captured |
|-------|-----------------|
| **ai.request** | Model, messages, parameters, timestamp |
| **ai.response** | Response content, token usage, timing |
| **network.connect** | Connection to api.openai.com:443 |
| **process.exec** | python3 execution |

## Analysis

```bash
# Cost estimate
cat output/events.jsonl | jq -r 'select(.event_type=="ai.response") | .data.usage.total_tokens'

# Response times
cat output/events.jsonl | \
  jq -r 'select(.event_type=="ai.response") |
         (.ts as $end |
          (.data.request_ts // $end) as $start |
          ($end | fromdateiso8601) - ($start | fromdateiso8601))'

# Provider summary
oisp-sensor analyze output/events.jsonl
```

## Try It Yourself

Modify `app.py` to:

1. **Use different models:**
   ```python
   model="gpt-4o"  # More capable, more expensive
   ```

2. **Add system prompt:**
   ```python
   messages=[
       {"role": "system", "content": "You are a pirate."},
       {"role": "user", "content": "Say hello!"}
   ]
   ```

3. **Make multiple requests:**
   ```python
   for topic in ["space", "ocean", "forest"]:
       response = client.chat.completions.create(...)
   ```

## Next Steps

- **[LiteLLM](./litellm/)** - Use multiple AI providers
- **[LangChain Agent](./langchain-agent/)** - Build agents with tools
- **[FastAPI Service](./fastapi-service/)** - Production API service
