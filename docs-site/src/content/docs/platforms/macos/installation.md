---
title: macOS Installation
description: Install OISP Sensor on macOS
---


Install OISP Sensor on macOS for metadata capture.

## Homebrew (Recommended)

```bash
brew install oximy/tap/oisp-sensor
```

## Manual Download

1. Download the `.pkg` from [Releases](https://github.com/oximyHQ/oisp-sensor/releases)
2. Open the `.pkg` file
3. Follow the installer prompts

## Binary Installation

```bash
# Download for macOS (Intel)
curl -LO https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-x86_64-apple-darwin.tar.gz

# Or for Apple Silicon
curl -LO https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-aarch64-apple-darwin.tar.gz

# Extract and install
tar -xzf oisp-sensor-*.tar.gz
sudo mv oisp-sensor /usr/local/bin/
sudo chmod +x /usr/local/bin/oisp-sensor
```

## Verify Installation

```bash
oisp-sensor --version
```

## Next Steps

- [Quick Start](./quick-start) - Get started
- [Limitations](./limitations) - What works and what doesn't
