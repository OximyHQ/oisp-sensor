---
title: macOS Overview
description: OISP Sensor on macOS - Preview status and capabilities
---


⚠️ **Preview Status** - macOS support is in preview with metadata capture only.

## Current Status

OISP Sensor on macOS currently supports **metadata capture** without full SSL/TLS interception:

**What works:**
- Process execution/termination events
- Network connection metadata
- File operation events
- Basic AI provider detection (from network traffic)

**What doesn't work yet:**
- Full SSL/TLS content capture
- Complete request/response bodies
- Tool call content

## Event Support

| Event Type | Support | Notes |
|------------|---------|-------|
| `ai.request` | Metadata | Provider, timing, size only |
| `ai.response` | Metadata | Provider, timing, size only |
| `process.exec` | Full | Complete process events |
| `file.write` | Full | Complete file events |
| `network.connect` | Full | Complete network events |

## Roadmap

**Full SSL/TLS capture coming soon** via macOS System Extension (Endpoint Security + Network Extension frameworks).

Expected timeline: Q1 2025

## Next Steps

- [Installation](./installation) - Install on macOS
- [Quick Start](./quick-start) - Get started
- [Limitations](./limitations) - Current limitations
