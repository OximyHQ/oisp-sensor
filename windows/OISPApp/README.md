# OISP Sensor System Tray Application

A Windows system tray application for managing the OISP Sensor.

## Features

- **System Tray Icon**: Runs minimized in the system tray
- **Start/Stop Capture**: Easy control over capture sessions
- **CA Certificate Management**: One-click CA certificate installation
- **Settings**: Configure output path, filters, and behavior
- **Process Management**: Automatically manages sensor and redirector processes

## Prerequisites

- .NET 8.0 SDK or Runtime
- Windows 10/11
- OISP Sensor binaries (`oisp-sensor.exe`, `oisp-redirector.exe`)

## Building

```powershell
# From the OISPApp directory
dotnet build

# For release
dotnet build -c Release

# Publish self-contained
dotnet publish -c Release -r win-x64 --self-contained
```

## Icons

Place icon files in the `Resources` directory:
- `oisp-icon.ico` - Default tray icon (gray/inactive)
- `oisp-icon-active.ico` - Active capture tray icon (green/active)

Icons should be multi-resolution ICO files (16x16, 32x32, 48x48, 256x256).

To generate placeholder icons, run:
```powershell
cd windows\Scripts
.\generate-icons.ps1
```

## Usage

1. **Launch**: Double-click `OISPApp.exe` or add to startup
2. **Install CA**: Right-click tray icon → "Install CA Certificate"
3. **Start Capture**: Right-click tray icon → "Start Capture"
4. **View Logs**: Right-click tray icon → "View Logs"
5. **Settings**: Double-click tray icon or right-click → "Settings..."
6. **Exit**: Right-click tray icon → "Exit"

## Architecture

```
OISPApp.exe (WPF Tray App)
    │
    ├── Manages: oisp-sensor.exe (Normal user)
    │   └── Receives events via Named Pipe
    │   └── Writes to events.jsonl
    │
    └── Manages: oisp-redirector.exe (Administrator)
        └── Captures traffic via WinDivert
        └── TLS MITM for HTTPS
        └── Sends events via Named Pipe
```

## Settings Location

Settings are stored at:
```
%LOCALAPPDATA%\OISP\settings.json
```

CA certificate is stored at:
```
%LOCALAPPDATA%\OISP\oisp-ca.crt
```

## Troubleshooting

### "Could not find oisp-sensor.exe"
Place the sensor binaries in one of these locations:
- Same directory as OISPApp.exe
- In system PATH
- Configure in Settings → Advanced

### UAC Prompt
The redirector requires administrator privileges for packet capture.
A UAC prompt will appear when starting capture.

### CA Certificate Not Trusted
If HTTPS interception doesn't work:
1. Click "Install CA Certificate" in tray menu
2. Or manually install from `%LOCALAPPDATA%\OISP\oisp-ca.crt`
3. Add to "Trusted Root Certification Authorities" store
