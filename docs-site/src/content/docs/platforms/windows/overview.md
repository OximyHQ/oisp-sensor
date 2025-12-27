---
title: Windows Overview
description: OISP Sensor on Windows - Preview status and capabilities
---

# Windows Overview

⚠️ **Preview Status** - Windows support is in preview with metadata capture only.

## Current Status

OISP Sensor on Windows currently supports **metadata capture** without full SSL/TLS interception.

**What works:**
- Process execution/termination events
- Network connection metadata
- File operation events
- Basic AI provider detection

**What doesn't work yet:**
- Full SSL/TLS content capture
- Complete request/response bodies

## Event Support

| Event Type | Support | Notes |
|------------|---------|-------|
| `ai.request` | Metadata | Provider, timing, size only |
| `ai.response` | Metadata | Provider, timing, size only |
| `process.exec` | Full | Complete process events |
| `file.write` | Full | Complete file events |
| `network.connect` | Full | Complete network events |

## Roadmap

**Full SSL/TLS capture coming soon** via ETW (Event Tracing for Windows) service.

Expected timeline: Q1-Q2 2025

## Next Steps

- [Installation](./installation) - Install on Windows
- [Quick Start](./quick-start) - Get started
