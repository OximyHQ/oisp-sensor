---
title: Windows Installation
description: Install OISP Sensor on Windows with full SSL/TLS capture
---

Install OISP Sensor on Windows for full SSL/TLS content capture of AI API traffic.

## Option 1: Installer (Recommended)

1. Download `oisp-sensor-setup.exe` from the [Releases](https://github.com/oximyHQ/oisp-sensor/releases)
2. Run the installer (requires Administrator)
3. Launch OISP from the Start Menu
4. Right-click tray icon → "Install CA Certificate"
5. Right-click tray icon → "Start Capture"

## Option 2: Portable ZIP

```powershell
# Download
Invoke-WebRequest -Uri https://github.com/oximyHQ/oisp-sensor/releases/latest/download/oisp-sensor-x86_64-pc-windows-msvc.zip -OutFile oisp-sensor.zip

# Extract
Expand-Archive oisp-sensor.zip -DestinationPath C:\OISP

# Run the tray app
C:\OISP\OISPApp.exe
```

## Option 3: winget (Coming Soon)

```powershell
winget install OISP.Sensor
```

## Verify Installation

```powershell
# Check version
C:\OISP\oisp-sensor.exe --version

# Check help
C:\OISP\oisp-sensor.exe --help
```

## CA Certificate Installation

For HTTPS interception, the OISP CA certificate must be trusted:

### Via System Tray App (Recommended)

Right-click the OISP tray icon → "Install CA Certificate"

### Via Command Line

```powershell
# The CA certificate is stored at:
# %LOCALAPPDATA%\OISP\oisp-ca.crt

# View certificate store
certmgr.msc
# Navigate to: Trusted Root Certification Authorities → Certificates
# Look for "OISP Sensor CA"
```

## Configuration Location

User settings are stored at:
```
%LOCALAPPDATA%\OISP\settings.json
```

CA certificate and key:
```
%LOCALAPPDATA%\OISP\oisp-ca.crt
%LOCALAPPDATA%\OISP\oisp-ca.key
```

Captured events (default):
```
%USERPROFILE%\Documents\OISP\events.jsonl
```

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (1.83 or later)
- [.NET 8 SDK](https://dotnet.microsoft.com/download/dotnet/8.0)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) (for Windows SDK)

```powershell
# Install via winget
winget install Rustlang.Rustup
winget install Microsoft.DotNet.SDK.8
winget install Microsoft.VisualStudio.2022.BuildTools
```

### Build Steps

```powershell
# Clone the repository
git clone https://github.com/oximyHQ/oisp-sensor.git
cd oisp-sensor

# Run one-time setup (downloads WinDivert)
.\windows\Scripts\setup-dev.ps1

# Build everything
.\windows\Scripts\build-app.ps1

# Or build manually:
cargo build --release --bin oisp-sensor --bin oisp-redirector
cd windows\OISPApp && dotnet build -c Release
```

## Next Steps

- [Quick Start](./quick-start) - Get started
- [Overview](./overview) - Architecture and capabilities
