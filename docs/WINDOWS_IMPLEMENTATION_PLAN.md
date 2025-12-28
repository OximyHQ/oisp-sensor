# OISP Sensor Windows Implementation Plan

> **Goal**: Implement Windows support equivalent to macOS, with incremental milestones that can be tested end-to-end at each phase.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│              OISP System Tray App (C# WPF)                      │
│  - Status display, settings, CA certificate management         │
│  - Start/Stop capture control                                   │
│  - Launches redirector with UAC elevation                       │
└─────────────────────────────────────────────────────────────────┘
              ↕ Named Pipe IPC (\\.\pipe\oisp-capture)
┌─────────────────────────────────────────────────────────────────┐
│              oisp-sensor.exe (Rust binary)                      │
│  - Event processing pipeline                                    │
│  - HTTP/AI API decoding                                         │
│  - TLS MITM Proxy (rustls + rcgen)                             │
│  - Exporters (JSONL, WebSocket, etc.)                          │
└─────────────────────────────────────────────────────────────────┘
              ↕ Named Pipe IPC
┌─────────────────────────────────────────────────────────────────┐
│              oisp-redirector.exe (Rust, runs elevated)          │
│  - WinDivert packet capture (NETWORK + SOCKET layers)          │
│  - Process attribution via socket events                        │
│  - Redirects intercepted traffic to TLS proxy                   │
│  - Based on mitmproxy_rs (MIT licensed)                         │
└─────────────────────────────────────────────────────────────────┘
              ↓ Packet interception
┌─────────────────────────────────────────────────────────────────┐
│              WinDivert Driver (Pre-signed, v2.2.2)              │
│  - Kernel-mode packet capture                                   │
│  - Already EV-signed by WinDivert project                       │
│  - No test signing mode required                                │
└─────────────────────────────────────────────────────────────────┘
```

## Key Dependencies (Verified Working)

| Dependency | Version | Purpose | License |
|------------|---------|---------|---------|
| `windivert` | 0.6.0 | Rust bindings for WinDivert | LGPL-3.0 |
| `internet-packet` | 0.2.2 | TCP/IP packet parsing | MIT |
| `tokio` | 1.42+ | Async runtime | MIT |
| `rustls` | 0.23 | TLS implementation | MIT/Apache-2.0 |
| `rcgen` | 0.13 | Certificate generation | MIT/Apache-2.0 |
| WinDivert | 2.2.2 | Packet capture driver | LGPL-3.0 |

**Reference Implementation**: [mitmproxy_rs](https://github.com/mitmproxy/mitmproxy_rs) (MIT licensed)

---

## Phase 1: Minimal WinDivert POC
**Goal**: Prove WinDivert works from Rust, capture any packet

### Milestone 1.1: Project Setup
- [ ] Create `windows/` directory structure
- [ ] Create `oisp-redirector` crate (separate binary)
- [ ] Download WinDivert 2.2.2-A to `windows/deps/`
- [ ] Configure `WINDIVERT_PATH` in build

### Milestone 1.2: Basic Packet Capture
- [ ] Implement minimal WinDivert network layer capture
- [ ] Log all TCP packets to console
- [ ] Test: Run as Admin, see packets from `curl https://api.openai.com`

### Milestone 1.3: Socket Layer (Process Attribution)
- [ ] Add WinDivert socket layer
- [ ] Map connections to PIDs
- [ ] Test: See PID alongside packets

**End-to-End Test**:
```powershell
# As Administrator
.\target\release\oisp-redirector.exe
# In another terminal
curl https://api.openai.com/v1/models
# Should see: "PID 1234 (curl.exe) -> api.openai.com:443"
```

---

## Phase 2: Named Pipe IPC
**Goal**: Redirector can send events to main sensor process

### Milestone 2.1: Named Pipe Server in Sensor
- [ ] Implement Named Pipe server in `oisp-capture-windows`
- [ ] Define protobuf/JSON message format for events
- [ ] Test: Sensor listens on `\\.\pipe\oisp-capture`

### Milestone 2.2: Named Pipe Client in Redirector
- [ ] Connect redirector to sensor's named pipe
- [ ] Send packet metadata (no payload yet)
- [ ] Test: Sensor receives and logs connection events

### Milestone 2.3: Full Event Pipeline
- [ ] Send full packet data over IPC
- [ ] Integrate with OISP event pipeline
- [ ] Test: Events appear in `events.jsonl`

**End-to-End Test**:
```powershell
# Terminal 1 (normal)
.\target\release\oisp-sensor.exe record --output events.jsonl
# Terminal 2 (Admin)
.\target\release\oisp-redirector.exe
# Terminal 3
python -c "import requests; requests.get('https://api.openai.com/v1/models')"
# Check events.jsonl has capture.raw events
```

---

## Phase 3: Traffic Redirection to Local Proxy
**Goal**: Redirect selected traffic to local TLS proxy

### Milestone 3.1: Basic Proxy Server
- [ ] Implement TCP listener for redirected connections
- [ ] Accept connections, log, forward to original destination
- [ ] Test: Traffic flows through proxy (no TLS termination yet)

### Milestone 3.2: WinDivert Packet Rewriting
- [ ] Modify destination of intercepted packets to localhost:proxy_port
- [ ] Handle NAT-like address translation
- [ ] Preserve original destination for proxy
- [ ] Test: Packets redirected, HTTP works (not HTTPS)

### Milestone 3.3: Connection State Management
- [ ] Track connection states (like mitmproxy_rs)
- [ ] Handle connection close, timeouts
- [ ] Test: Multiple concurrent connections work

**End-to-End Test**:
```powershell
# Traffic to port 80 (HTTP) is redirected and logged
curl http://httpbin.org/get
# Should see request/response in logs
```

---

## Phase 4: TLS MITM Proxy
**Goal**: Full SSL/TLS interception with dynamic certificates

### Milestone 4.1: Certificate Authority
- [ ] Generate root CA on first run
- [ ] Store CA in user-accessible location
- [ ] Implement `rcgen` cert generation for any hostname
- [ ] Test: Generate cert for `api.openai.com`, valid chain

### Milestone 4.2: TLS Termination
- [ ] Use `rustls` for TLS server (client-facing)
- [ ] Use `rustls` for TLS client (server-facing)
- [ ] Dynamic certificate generation per SNI
- [ ] Test: HTTPS works when CA manually trusted

### Milestone 4.3: Plaintext Capture
- [ ] Capture decrypted request/response data
- [ ] Send to OISP pipeline as `SslRead`/`SslWrite` events
- [ ] Test: Full HTTP request/response in events.jsonl

**End-to-End Test**:
```powershell
# 1. Trust the CA certificate
# 2. Run sensor and redirector
# 3. Make OpenAI API call
python -c "import openai; print(openai.OpenAI().models.list())"
# events.jsonl should have ai.request and ai.response events
```

---

## Phase 5: AI Endpoint Filtering
**Goal**: Only intercept known AI API endpoints

### Milestone 5.1: Load AI Endpoint Spec
- [ ] Load `oisp-spec-bundle.json` at startup
- [ ] Parse AI provider endpoint patterns
- [ ] Test: Spec loads correctly

### Milestone 5.2: Selective Interception
- [ ] Check destination against AI endpoints before redirect
- [ ] Pass through non-AI traffic unchanged
- [ ] Test: Only OpenAI/Anthropic traffic intercepted

### Milestone 5.3: Process Filtering (Optional)
- [ ] Allow filtering by process name/PID
- [ ] Configuration via command line or config file
- [ ] Test: Only intercept `python.exe` traffic

**End-to-End Test**:
```powershell
# Regular HTTPS works without interception
curl https://google.com  # Passes through
# AI traffic is intercepted
python -c "import openai; openai.OpenAI().chat.completions.create(...)"  # Captured
```

---

## Phase 6: System Tray Application
**Goal**: User-friendly Windows application

### Milestone 6.1: Basic Tray App
- [ ] Create WPF project in `windows/OISPApp/`
- [ ] System tray icon with context menu
- [ ] Start/Stop capture menu items
- [ ] Test: Icon appears, menu works

### Milestone 6.2: Sensor Integration
- [ ] Launch sensor process on start
- [ ] Launch redirector with UAC elevation
- [ ] IPC for status updates
- [ ] Test: Full capture starts from tray

### Milestone 6.3: Certificate Management
- [ ] "Install CA Certificate" menu item
- [ ] Opens certmgr or auto-installs to Trusted Root
- [ ] Status indicator for CA installation
- [ ] Test: One-click CA installation

### Milestone 6.4: Settings Window
- [ ] Process filter configuration
- [ ] Export path configuration
- [ ] Auto-start option
- [ ] Test: Settings persist across restarts

**End-to-End Test**:
```
1. Launch OISPApp.exe (double-click)
2. Right-click tray icon → "Install CA Certificate" → Accept UAC
3. Right-click tray icon → "Start Capture" → Accept UAC
4. Run Python OpenAI script
5. Right-click tray icon → "Open Dashboard"
6. See captured AI events in dashboard
```

---

## Phase 7: Installer & Distribution
**Goal**: Professional Windows distribution

### Milestone 7.1: NSIS Installer
- [ ] Create installer script in `windows/installer/`
- [ ] Bundle all binaries + WinDivert
- [ ] Start menu shortcuts
- [ ] Uninstaller
- [ ] Test: Clean install/uninstall

### Milestone 7.2: Code Signing (Optional for Dev)
- [ ] Document EV certificate procurement
- [ ] GitHub Actions signing workflow
- [ ] Test on clean Windows VM (SmartScreen)

### Milestone 7.3: winget Submission
- [ ] Create manifest files
- [ ] Submit PR to winget-pkgs
- [ ] Test: `winget install OISP.Sensor`

---

## Directory Structure

```
oisp-sensor/
├── crates/
│   ├── oisp-capture-windows/
│   │   ├── src/
│   │   │   ├── lib.rs                 # CapturePlugin implementation
│   │   │   ├── ipc/
│   │   │   │   ├── mod.rs
│   │   │   │   └── named_pipe.rs      # Named pipe server
│   │   │   ├── proxy/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── tls_mitm.rs        # TLS MITM proxy
│   │   │   │   └── certificate.rs     # CA & cert generation
│   │   │   └── filter.rs              # AI endpoint filter
│   │   └── Cargo.toml
│   │
│   └── oisp-redirector/               # NEW: Elevated redirector
│       ├── src/
│       │   ├── main.rs
│       │   ├── windivert.rs           # WinDivert wrapper
│       │   ├── connection.rs          # Connection state
│       │   └── ipc.rs                 # Named pipe client
│       └── Cargo.toml
│
└── windows/                           # NEW: Windows platform
    ├── OISPApp/                       # System Tray App (C# WPF)
    │   ├── App.xaml
    │   ├── MainWindow.xaml
    │   ├── TrayIcon.cs
    │   ├── Services/
    │   │   ├── SensorService.cs
    │   │   └── CertificateService.cs
    │   └── OISPApp.csproj
    │
    ├── deps/                          # Pre-downloaded dependencies
    │   └── WinDivert-2.2.2-A/
    │       ├── WinDivert.dll
    │       ├── WinDivert64.sys
    │       └── WinDivert.lib
    │
    ├── installer/
    │   ├── oisp-sensor.nsi
    │   └── resources/
    │
    └── Scripts/
        ├── build-release.ps1
        ├── setup-dev.ps1              # Downloads WinDivert, sets env
        └── test-capture.ps1           # End-to-end test script
```

---

## Local Development Setup

### Prerequisites
```powershell
# Install Rust
winget install Rustlang.Rustup

# Install .NET SDK (for tray app)
winget install Microsoft.DotNet.SDK.8

# Install Visual Studio Build Tools (for Windows SDK)
winget install Microsoft.VisualStudio.2022.BuildTools
```

### One-Time Setup
```powershell
# Run from oisp-sensor root
cd windows/Scripts
.\setup-dev.ps1
```

This script will:
1. Download WinDivert 2.2.2-A
2. Extract to `windows/deps/`
3. Set `WINDIVERT_PATH` environment variable
4. Verify everything works

### Build & Test
```powershell
# Build all
cargo build --release

# Run tests (no Admin needed)
cargo test --workspace

# Run end-to-end (Admin needed)
# Terminal 1:
.\target\release\oisp-sensor.exe record --output events.jsonl
# Terminal 2 (Run as Admin):
.\target\release\oisp-redirector.exe
# Terminal 3:
python -c "import openai; openai.OpenAI().chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'hi'}])"
# Check events.jsonl
```

---

## Testing Matrix

| Phase | Test | Expected Result |
|-------|------|-----------------|
| 1 | `oisp-redirector.exe` logs packets | TCP packets to console |
| 2 | Sensor receives IPC events | Events in pipeline |
| 3 | HTTP traffic redirected | Request/response logged |
| 4 | HTTPS traffic decrypted | Plaintext in events |
| 5 | Non-AI traffic passes through | Google.com works normally |
| 6 | Tray app starts capture | Full flow from GUI |
| 7 | Installer on clean VM | Works with SmartScreen |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| `windivert` crate is stale | Reference mitmproxy_rs, fork if needed |
| WinDivert driver signature expires | Monitor releases, bundle specific version |
| Antivirus flags WinDivert | Document common AV exclusions |
| Complex TCP reassembly | Use `internet-packet` crate like mitmproxy |
| EV cert cost | Delay signing until ready for public release |

---

## Success Criteria

### Phase 1-5 Complete
- [ ] Can capture AI API traffic on Windows
- [ ] Events match Linux/macOS format
- [ ] Works on Windows 10 and 11
- [ ] No test signing mode required
- [ ] Admin only needed for redirector

### Phase 6-7 Complete
- [ ] User-friendly tray application
- [ ] One-click CA installation
- [ ] Clean installer experience
- [ ] Available on winget
- [ ] No SmartScreen warnings (signed)

---

## Timeline (Estimates)

| Phase | Effort | Cumulative |
|-------|--------|------------|
| Phase 1 | 2-3 days | 2-3 days |
| Phase 2 | 2-3 days | 4-6 days |
| Phase 3 | 3-4 days | 7-10 days |
| Phase 4 | 4-5 days | 11-15 days |
| Phase 5 | 1-2 days | 12-17 days |
| Phase 6 | 3-4 days | 15-21 days |
| Phase 7 | 2-3 days | 17-24 days |

**Total**: ~3-4 weeks of focused development

---

## References

- [WinDivert Documentation](https://reqrypt.org/windivert-doc.html)
- [mitmproxy_rs Windows Implementation](https://github.com/mitmproxy/mitmproxy_rs/tree/main/mitmproxy-windows)
- [windivert-rust Crate](https://docs.rs/windivert)
- [mitmproxy Windows Local Capture Blog](https://www.mitmproxy.org/posts/local-capture/windows/)
- [OISP macOS Implementation](../macos/) (reference architecture)
