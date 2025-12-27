---
title: Windows Installation
description: Install OISP Sensor on Windows
---


Install OISP Sensor on Windows for metadata capture.

## winget (Recommended)

```powershell
winget install Oximy.OISPSensor
```

## Manual Download

1. Download the `.msi` from [Releases](https://github.com/oximyHQ/oisp-sensor/releases)
2. Run the installer
3. Follow the prompts

## Binary Installation

```powershell
# Download
Invoke-WebRequest -Uri https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-x86_64-pc-windows-msvc.zip -OutFile oisp-sensor.zip

# Extract
Expand-Archive oisp-sensor.zip -DestinationPath C:\Program Files\OISP

# Add to PATH (run as Administrator)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\OISP", "Machine")
```

## Verify Installation

```powershell
oisp-sensor.exe --version
```

## Next Steps

- [Quick Start](./quick-start) - Get started
