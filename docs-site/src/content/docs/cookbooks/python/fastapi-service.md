---
title: Python + FastAPI Service
description: Production API service with AI capabilities
---

# Python + FastAPI Service

Production-grade API service with AI endpoints and full observability.

## Overview

**What this demonstrates:**
- FastAPI production patterns
- Async AI requests
- Health checks and monitoring
- Structured logging
- Error handling

**Repository:** `oisp-cookbook/python/04-fastapi-service`

## Key Code

```python
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import openai

app = FastAPI(title="AI Service")

class ChatRequest(BaseModel):
    message: str
    model: str = "gpt-4o-mini"

@app.post("/chat")
async def chat(request: ChatRequest):
    try:
        response = openai.ChatCompletion.create(
            model=request.model,
            messages=[{"role": "user", "content": request.message}]
        )
        return {"response": response.choices[0].message.content}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.get("/health")
async def health():
    return {"status": "ok"}
```

## Running

```bash
cd oisp-cookbook/python/04-fastapi-service
docker-compose up
```

**Test endpoints:**

```bash
# Health check
curl http://localhost:8000/health

# Chat request
curl -X POST http://localhost:8000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!", "model": "gpt-4o-mini"}'
```

## Captured Events

- All API requests to AI providers
- Request/response correlation by process
- Async request timing
- Error events

## Use Cases

- Production API services
- Microservices architecture
- API gateways
- Internal tools
