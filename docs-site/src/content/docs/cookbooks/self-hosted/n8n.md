---
title: n8n Workflow Automation
description: Monitor AI workflows in n8n with OISP Sensor
---

# n8n Workflow Automation

Monitor AI-powered n8n workflows.

## Overview

**What this demonstrates:**
- Self-hosted n8n with AI nodes
- Workflow execution tracing
- OpenAI/Anthropic integration
- Background workflow monitoring

**Repository:** `oisp-cookbook/self-hosted/n8n`

## Running

```bash
cd oisp-cookbook/self-hosted/n8n
docker-compose up
```

**Access n8n:** http://localhost:5678

## Sample Workflow

Create workflow with:
1. HTTP trigger
2. OpenAI Chat node
3. Send email/webhook

## Captured Events

- All AI requests from n8n workflows
- Workflow execution context
- Node-level tracing

## Use Cases

- No-code AI workflows
- Marketing automation
- Data processing
- Integration automation
