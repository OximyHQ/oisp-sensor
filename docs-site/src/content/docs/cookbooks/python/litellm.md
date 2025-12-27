---
title: Python + LiteLLM
description: Use multiple AI providers with LiteLLM and OISP Sensor
---


Use any AI provider (OpenAI, Anthropic, Azure, AWS, etc.) with unified API via LiteLLM.

## Overview

**What this demonstrates:**
- Using LiteLLM for provider abstraction
- Switching between providers seamlessly
- Capturing events from any provider

**Repository:** `oisp-cookbook/python/02-litellm`

## Key Code

```python
from litellm import completion

# Works with OpenAI
response = completion(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Hello!"}]
)

# Works with Anthropic
response = completion(
    model="claude-3-haiku-20240307",
    messages=[{"role": "user", "content": "Hello!"}]
)

# Works with any provider
providers = ["gpt-4o-mini", "claude-3-haiku", "gemini-pro"]
for model in providers:
    response = completion(model=model, messages=[...])
```

## Captured Events

OISP Sensor detects the actual provider used:
- `gpt-4o-mini` → `provider: "OpenAI"`
- `claude-3-haiku` → `provider: "Anthropic"`
- `gemini-pro` → `provider: "Google"`

## Running

```bash
cd oisp-cookbook/python/02-litellm
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
docker-compose up
```

## Use Cases

- Multi-provider failover
- Cost optimization (route to cheapest)
- A/B testing models
- Provider-agnostic applications
