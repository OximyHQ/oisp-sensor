# OISP Sensor Windows Installer

This directory contains the NSIS installer script and resources for creating the OISP Sensor Windows installer.

## Prerequisites

- [NSIS 3.x](https://nsis.sourceforge.io/) - Nullsoft Scriptable Install System
- Windows SDK (for code signing, optional)

Install NSIS:
```powershell
winget install NSIS.NSIS
```

## Building the Installer

### Quick Build

```powershell
cd windows\installer
.\build-installer.ps1
```

This will:
1. Build Rust binaries (`oisp-sensor.exe`, `oisp-redirector.exe`)
2. Build .NET application (`OISPApp.exe`)
3. Package everything with NSIS
4. Output `oisp-sensor-setup.exe`

### Options

```powershell
# Skip rebuild (use existing binaries)
.\build-installer.ps1 -SkipBuild

# Specify version
.\build-installer.ps1 -Version "1.0.0"
```

### Manual Build

```powershell
# 1. Build Rust binaries
cargo build --release --bin oisp-sensor --bin oisp-redirector

# 2. Build .NET app
cd windows\OISPApp
dotnet build -c Release

# 3. Run NSIS
cd windows\installer
makensis.exe oisp-sensor.nsi
```

## Installer Contents

The installer packages:

| File | Source | Description |
|------|--------|-------------|
| `oisp-sensor.exe` | `target/release/` | Event processing |
| `oisp-redirector.exe` | `target/release/` | Packet capture |
| `OISPApp.exe` | `OISPApp/bin/Release/` | System tray app |
| `OISPApp.dll` | `OISPApp/bin/Release/` | .NET dependencies |
| `WinDivert.dll` | `deps/WinDivert-*/x64/` | Packet capture library |
| `WinDivert64.sys` | `deps/WinDivert-*/x64/` | Kernel driver |

## Resources

| File | Purpose |
|------|---------|
| `resources/oisp-icon.ico` | Installer icon |
| `resources/welcome.bmp` | MUI welcome page image (164x314) |
| `resources/LICENSE.txt` | License agreement text |
| `resources/README.txt` | Post-install readme |

## Installation Location

Default: `C:\Program Files\OISP Sensor\`

User data: `%LOCALAPPDATA%\OISP\`

## Silent Installation

```powershell
# Silent install
oisp-sensor-setup.exe /S

# Silent install to custom location
oisp-sensor-setup.exe /S /D=D:\OISP

# Silent uninstall
"C:\Program Files\OISP Sensor\uninstall.exe" /S
```

## Code Signing (Production)

For production releases, sign the installer:

```powershell
# Sign the executable
signtool sign /f certificate.pfx /p password /t http://timestamp.digicert.com oisp-sensor-setup.exe

# Or with Azure SignTool
AzureSignTool sign -kvu https://vault.azure.net -kvc certificate-name oisp-sensor-setup.exe
```

## Winget Submission

A winget manifest template is provided at `winget/OISP.Sensor.yaml`.

To submit to winget-pkgs:

1. Build and sign the installer
2. Upload to GitHub Releases
3. Calculate SHA256: `Get-FileHash oisp-sensor-setup.exe`
4. Update the manifest with URL and hash
5. Submit PR to microsoft/winget-pkgs

```powershell
# Calculate hash
$hash = (Get-FileHash oisp-sensor-setup.exe -Algorithm SHA256).Hash
Write-Host "InstallerSha256: $hash"
```

## Customization

### Changing the Icon

Replace `resources/oisp-icon.ico` with your icon. Use a multi-resolution ICO file (16x16, 32x32, 48x48, 256x256).

### Changing Welcome Image

Replace `resources/welcome.bmp` with a 164x314 pixel BMP image.

### Version Number

Version is embedded in the NSIS script and can be overridden:

```powershell
makensis.exe /DVERSION=1.2.3 oisp-sensor.nsi
```

## Troubleshooting

### "WinDivert not found"

Run `setup-dev.ps1` to download WinDivert:
```powershell
cd windows\Scripts
.\setup-dev.ps1
```

### NSIS Errors

- Ensure NSIS 3.x is installed
- Check that all source files exist
- Run NSIS from the installer directory

### Antivirus Blocking

Some antivirus software may flag:
- WinDivert driver (kernel driver for packet capture)
- The installer itself (uses NSIS which is sometimes flagged)

Solutions:
- Code sign the installer with an EV certificate
- Add exclusions for `%ProgramFiles%\OISP Sensor`
