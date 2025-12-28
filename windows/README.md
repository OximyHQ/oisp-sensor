# OISP for Windows

OISP (Observability for Intelligent Systems Platform) captures AI API traffic from any application on your Windows PC, providing real-time visibility into LLM usage, costs, and performance.

## Architecture

OISP on Windows uses WinDivert for transparent packet interception and a TLS MITM proxy for HTTPS traffic decryption. The architecture consists of:

```
┌─────────────────────────────────────────────────────────────────┐
│                   OISP System Tray App (WPF)                     │
│  - Status display, settings, and process control                │
│  - Launches redirector with UAC elevation                       │
│  - One-click CA certificate installation                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                     Named Pipe IPC
                              │
┌─────────────────────────────────────────────────────────────────┐
│                    oisp-sensor.exe (Rust)                        │
│  Receives events, decodes HTTP, emits to dashboard/exports      │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                     Named Pipe IPC
                              │
┌─────────────────────────────────────────────────────────────────┐
│              oisp-redirector.exe (Elevated)                      │
│  - WinDivert packet capture and redirection                     │
│  - TLS MITM proxy (rustls + rcgen)                              │
│  - AI endpoint filtering                                        │
│  - Process attribution                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                    WinDivert Driver
                              │
┌─────────────────────────────────────────────────────────────────┐
│                Windows Network Stack                             │
│  - Kernel-mode packet interception                              │
│  - Pre-signed driver (no test signing required)                 │
└─────────────────────────────────────────────────────────────────┘
```

## Supported AI Providers

OISP automatically intercepts traffic to these AI endpoints:

| Provider | Endpoints |
|----------|-----------|
| OpenAI | api.openai.com |
| Anthropic | api.anthropic.com |
| Google AI | generativelanguage.googleapis.com, aiplatform.googleapis.com |
| Azure OpenAI | *.openai.azure.com |
| AWS Bedrock | bedrock-runtime.*.amazonaws.com |
| Cohere | api.cohere.ai, api.cohere.com |
| Mistral | api.mistral.ai |
| Groq | api.groq.com |
| Together AI | api.together.xyz, api.together.ai |
| Fireworks | api.fireworks.ai |
| Perplexity | api.perplexity.ai |
| OpenRouter | openrouter.ai, api.openrouter.ai |
| Replicate | api.replicate.com |
| Hugging Face | api-inference.huggingface.co |
| DeepSeek | api.deepseek.com |
| xAI (Grok) | api.x.ai |
| Local (Ollama) | localhost:11434, 127.0.0.1:11434 |
| Local (LM Studio) | localhost:1234, 127.0.0.1:1234 |

## Requirements

- Windows 10/11 (64-bit)
- Administrator privileges (for packet capture)
- ~50 MB disk space

## Installation

### Option 1: Installer (Recommended)

1. Download `oisp-sensor-setup.exe` from the [releases page](https://github.com/oximyHQ/oisp-sensor/releases)
2. Run the installer (requires Administrator)
3. Launch OISP from the Start Menu
4. Right-click tray icon → "Install CA Certificate"
5. Right-click tray icon → "Start Capture"

### Option 2: Portable ZIP

1. Download `oisp-sensor-x86_64-pc-windows-msvc.zip` from releases
2. Extract to a folder (e.g., `C:\OISP`)
3. Run `OISPApp.exe`

### Option 3: winget (Coming Soon)

```powershell
winget install OISP.Sensor
```

## Quick Start

### Using the System Tray App

1. **Launch OISP** - Double-click `OISPApp.exe` or from Start Menu
2. **Install CA Certificate** - Right-click tray icon → "Install CA Certificate"
3. **Start Capture** - Right-click tray icon → "Start Capture" (accept UAC prompt)
4. **Use AI Tools** - Run Python, Node.js, or any application that calls AI APIs
5. **View Events** - Right-click tray icon → "View Logs"

### Using Command Line

```powershell
# Terminal 1: Start the sensor (normal user)
.\oisp-sensor.exe record --output events.jsonl

# Terminal 2 (Run as Administrator): Start the redirector
.\oisp-redirector.exe --tls-mitm

# Terminal 3: Make AI API calls
python -c "import openai; print(openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'Hello'}]))"

# Events will appear in events.jsonl
```

### Command Line Options

```powershell
# oisp-sensor options
oisp-sensor.exe record --output events.jsonl    # Record to file
oisp-sensor.exe record --web                    # With web dashboard
oisp-sensor.exe record --verbose                # Verbose logging

# oisp-redirector options
oisp-redirector.exe --tls-mitm                  # Enable HTTPS interception
oisp-redirector.exe --all-traffic               # Capture all traffic (not just AI)
oisp-redirector.exe --port 8443                 # Custom proxy port
oisp-redirector.exe --verbose                   # Verbose logging
oisp-redirector.exe --help                      # Show all options
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

### Creating the Installer

Requires [NSIS](https://nsis.sourceforge.io/):

```powershell
winget install NSIS.NSIS

# Build installer
cd windows\installer
.\build-installer.ps1

# Output: oisp-sensor-setup.exe
```

## How It Works

### WinDivert Packet Capture

OISP uses [WinDivert](https://reqrypt.org/windivert.html) to intercept network packets:

1. **Packet Interception**: WinDivert captures outbound TCP connections to AI endpoints
2. **Traffic Redirection**: Connections are redirected to a local TLS proxy
3. **Process Attribution**: Connection ownership is determined via Windows TCP table APIs

### TLS MITM Proxy

For HTTPS traffic decryption:

1. **Certificate Authority**: OISP generates a local CA certificate on first run
2. **Certificate Trust**: The CA must be added to the Windows certificate store
3. **Per-host Certificates**: For each AI endpoint, OISP dynamically generates certificates
4. **Bidirectional Proxy**: The proxy terminates TLS and captures plaintext traffic

### AI Endpoint Filtering

OISP uses the embedded OISP Spec bundle to identify AI endpoints:

- **Domain Index**: Direct domain lookup (e.g., `api.openai.com`)
- **Regex Patterns**: Wildcard matching (e.g., `*.openai.azure.com`)
- **Non-AI Traffic**: Passes through unchanged (no interception)

## Configuration

### Settings Location

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

### Settings File Format

```json
{
  "OutputPath": "C:\\Users\\You\\Documents\\OISP\\events.jsonl",
  "AutoStartCapture": false,
  "EnableTlsMitm": true,
  "AiEndpointFilterEnabled": true,
  "ProcessFilter": "",
  "ProxyPort": 8443,
  "VerboseLogging": false
}
```

## Security Considerations

- **CA Private Key**: Never leaves your machine
- **Traffic Interception**: Only known AI endpoints are intercepted
- **Local Processing**: All data stays on your machine
- **Elevation**: Only `oisp-redirector.exe` requires Administrator

### WinDivert Driver

The WinDivert driver is:
- **Pre-signed**: EV code signed by the WinDivert project
- **No test signing required**: Works on standard Windows installations
- **Open source**: Licensed under LGPL-3.0

## Troubleshooting

### "Could not start capture"

1. Ensure you accepted the UAC prompt
2. Check if another application is using WinDivert
3. Temporarily disable antivirus and try again

### "HTTPS traffic not captured"

1. Ensure CA certificate is installed: Right-click tray → "Install CA Certificate"
2. Verify CA is in Trusted Root store: `certmgr.msc` → Trusted Root Certification Authorities
3. Some applications may use certificate pinning (cannot be intercepted)

### "WinDivert not found"

Run the setup script to download WinDivert:
```powershell
.\windows\Scripts\setup-dev.ps1
```

### Check Logs

```powershell
# View redirector logs
.\oisp-redirector.exe --verbose

# View sensor logs
.\oisp-sensor.exe record --verbose --output events.jsonl

# Check captured events
Get-Content events.jsonl | Select-Object -Last 10
```

### Reset CA Certificate

```powershell
# Remove existing CA
Remove-Item "$env:LOCALAPPDATA\OISP\oisp-ca.*"

# Restart capture to regenerate
# Or manually remove from certmgr.msc
```

## Known Limitations

- **Certificate Pinning**: Apps that pin certificates (like some browsers) cannot be intercepted
- **HTTP/2**: Currently limited HTTP/2 support (falls back to HTTP/1.1)
- **Non-TCP Traffic**: Only TCP traffic is intercepted (not UDP/QUIC)
- **Antivirus**: Some AV software may flag WinDivert driver

## Components

| Component | Description |
|-----------|-------------|
| `OISPApp.exe` | System tray application (.NET 8 WPF) |
| `oisp-sensor.exe` | Event processing and export (Rust) |
| `oisp-redirector.exe` | Packet capture and TLS proxy (Rust, requires elevation) |
| `WinDivert.dll` | Packet capture library |
| `WinDivert64.sys` | Kernel-mode driver |

## Contributing

### Development Without Admin

You can develop and test most code without Administrator privileges:

- ✅ Modify and test Rust code (`cargo build && cargo test`)
- ✅ Modify .NET code (`dotnet build`)
- ✅ Run unit tests
- ✅ Update documentation

### Full Testing Requires Admin

- ❌ Test packet capture end-to-end
- ❌ Test TLS MITM proxy
- ❌ Build signed releases

### CI/CD

GitHub Actions runs on every push/PR:
- Rust compilation on `windows-latest`
- .NET application build
- Artifact creation (unsigned)

## Privacy

OISP is designed with privacy in mind:

- **Local Only**: All processing happens on your PC
- **No Telemetry**: OISP does not phone home
- **Selective Interception**: Only AI API traffic is captured
- **User Control**: Pause/resume capture anytime

## License

See the main OISP repository for license information.

WinDivert is licensed under LGPL-3.0.
