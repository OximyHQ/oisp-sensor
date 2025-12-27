---
title: macOS Limitations
description: Current limitations of OISP Sensor on macOS
---

# macOS Limitations

## Current Limitations

OISP Sensor on macOS is in **preview** with the following limitations:

### ❌ No Full SSL/TLS Capture

**What this means:**
- Cannot see request/response **content**
- Cannot see prompts sent to AI providers
- Cannot see AI responses
- Cannot see tool call arguments/results

**What still works:**
- Provider detection (OpenAI, Anthropic, etc.)
- Connection timing and metadata
- Request/response sizes
- Process and file activity

### ❌ No System Extension Yet

macOS System Extensions (Endpoint Security + Network Extension) required for full capture are not yet implemented.

## What Works

✅ **Process Events**
- Process execution
- Process termination
- Command-line arguments
- Working directory

✅ **File Events**
- File reads
- File writes
- File paths

✅ **Network Events**
- Outbound connections
- Destination hosts/ports
- Connection timing

✅ **AI Provider Detection**
- Detects OpenAI, Anthropic, Google, etc.
- Based on destination domains
- Timing and size heuristics

## Workarounds

Until full SSL capture is available:

1. **Use Linux for full capture** - Run OISP Sensor on a Linux server or VM
2. **Use Docker** - Run Linux container with OISP Sensor
3. **Log at application level** - Instrument your code for full observability

## Roadmap

### Q1 2025: System Extension

**Planned:**
- macOS System Extension (.appex) with Endpoint Security
- Network Extension for SSL interception
- Full request/response capture
- Signed `.pkg` installer for easy installation

**Will require:**
- macOS 12+ (Monterey or later)
- User approval in System Preferences → Privacy & Security
- Notarized app from Oximy

### Future

- TUI/Web UI on macOS
- Native macOS app (optional)
- Keychain integration for secure storage

## Next Steps

- [Overview](./overview) - Current capabilities
- [Installation](./installation) - Install on macOS
- [Quick Start](./quick-start) - Get started
- **[Linux Guide](/platforms/linux/)** - For full capture

---

**Want to help?** macOS System Extension development is open for contributions! See [GitHub Issues](https://github.com/oximyHQ/oisp-sensor/issues?q=is%3Aissue+is%3Aopen+label%3Amacos).
