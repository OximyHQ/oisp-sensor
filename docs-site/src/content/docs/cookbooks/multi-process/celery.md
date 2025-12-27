---
title: Python Celery Workers
description: Monitor AI tasks in distributed Celery workers
---

# Python Celery Workers

Monitor AI processing across distributed Celery workers.

## Overview

**What this demonstrates:**
- Celery distributed task queue
- Multi-worker AI processing
- Task-level event correlation
- Background job monitoring

**Repository:** `oisp-cookbook/multi-process/python-celery`

## Key Code

```python
from celery import Celery
import openai

app = Celery('tasks', broker='redis://localhost')

@app.task
def process_with_ai(prompt):
    response = openai.ChatCompletion.create(
        model="gpt-4o-mini",
        messages=[{"role": "user", "content": prompt}]
    )
    return response.choices[0].message.content
```

## Running

```bash
cd oisp-cookbook/multi-process/python-celery
docker-compose up --scale worker=3
```

Starts:
- 1 Redis broker
- 3 Celery workers
- 1 OISP Sensor (monitors all workers)

## Captured Events

- Events from all workers
- Process-level attribution
- Task queue patterns
- Worker performance

## Use Cases

- Background job processing
- Distributed AI tasks
- Batch processing
- Worker pool monitoring
