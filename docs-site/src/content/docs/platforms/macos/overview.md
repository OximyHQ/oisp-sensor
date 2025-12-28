---
title: macOS Overview
description: OISP Sensor on macOS - Full SSL/TLS capture via System Extension
---

## Current Status

OISP Sensor on macOS provides **full SSL/TLS capture** via a Network Extension that performs transparent TLS interception.

**What works:**
- Full SSL/TLS content capture (request & response bodies)
- Complete AI API request/response parsing
- Tool call content and function definitions
- Process execution/termination events
- Network connection metadata
- File operation events

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     OISP Menu Bar App                        │
│  (SwiftUI app for status, settings, and extension control)  │
└─────────────────────────────────────────────────────────────┘
                              │
                     Unix Domain Socket
                              │
┌─────────────────────────────────────────────────────────────┐
│                    oisp-sensor (Rust)                        │
│  Receives events, decodes HTTP, emits to dashboard/exports  │
└─────────────────────────────────────────────────────────────┘
                              ▲
                     Unix Domain Socket (/tmp/oisp.sock)
                              │
┌─────────────────────────────────────────────────────────────┐
│              OISP Network Extension                          │
│  NETransparentProxyProvider + TLS MITM + Event Emission     │
└─────────────────────────────────────────────────────────────┘
```

## Requirements

- **macOS 13.0** (Ventura) or later
- **Apple Silicon** (M1/M2/M3/M4) or Intel Mac
- **Apple Developer Program** ($99/year) - Required for System Extension signing
- Admin access (for extension approval)

## Event Support

| Event Type | Support | Notes |
|------------|---------|-------|
| `ai.request` | Full | Complete request bodies, prompts, tools |
| `ai.response` | Full | Complete response content, token usage |
| `agent.tool_call` | Full | Tool names, arguments, results |
| `process.exec` | Full | Complete process events |
| `file.write` | Full | Complete file events |
| `network.connect` | Full | Complete network events |

## How It Works

1. **Network Extension** intercepts HTTPS connections to AI provider domains
2. **TLS MITM** decrypts traffic using a locally-generated CA certificate
3. **Events sent** to Rust sensor via Unix domain socket (`/tmp/oisp.sock`)
4. **Sensor decodes** HTTP and extracts AI-specific fields
5. **Exports** to JSONL, Kafka, OTLP, or web dashboard

## Quick Start

```bash
# 1. Build the sensor
cargo build --release

# 2. Build the macOS app (requires Xcode + Developer ID)
cd macos
xcodegen generate
xcodebuild -project OISP.xcodeproj -scheme OISP build

# 3. Run sensor with JSONL export
./target/release/oisp-sensor record --output ~/oisp-events.jsonl --web

# 4. Launch the menu bar app and approve the Network Extension
# 5. Trust the OISP CA certificate when prompted
# 6. Make AI API calls - they'll appear in the dashboard!
```

## Next Steps

- [Installation](./installation) - Detailed installation guide
- [Quick Start](./quick-start) - Get started quickly
- [Troubleshooting](./troubleshooting) - Common issues and solutions
