# OISP macOS - Parallel Task Breakdown

This document breaks down all implementation tasks into independent units that can be worked on in parallel by multiple agents.

---

## Task Naming Convention

```
[WORKSTREAM]-[PHASE]-[NUMBER]: Task Name
```

Example: `CA-1-001: Create CertificateAuthority class skeleton`

---

## Workstream A: Certificate Authority (CA)

### CA-1: Foundation (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| CA-1-001 | Create CertificateAuthority class skeleton | 2h | None | `OISPCore/Sources/Networking/CertificateAuthority.swift` |
| CA-1-002 | Implement RSA key pair generation | 4h | CA-1-001 | Same file |
| CA-1-003 | Implement Keychain storage for CA private key | 4h | CA-1-002 | Same file |
| CA-1-004 | Implement Keychain retrieval for CA private key | 2h | CA-1-003 | Same file |
| CA-1-005 | Add ACL protection to private key | 2h | CA-1-003 | Same file |

### CA-2: Certificate Generation (Depends on CA-1)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| CA-2-001 | Research swift-certificates library usage | 2h | None | Notes/research doc |
| CA-2-002 | Implement self-signed CA certificate generation | 8h | CA-1-002, CA-2-001 | `CertificateAuthority.swift` |
| CA-2-003 | Implement per-host certificate generation | 6h | CA-2-002 | `CertificateAuthority.swift` |
| CA-2-004 | Implement certificate caching (in-memory) | 2h | CA-2-003 | `CertificateAuthority.swift` |
| CA-2-005 | Implement certificate expiry and rotation | 4h | CA-2-004 | `CertificateAuthority.swift` |
| CA-2-006 | Add Subject Alternative Names (SAN) support | 2h | CA-2-003 | `CertificateAuthority.swift` |

### CA-3: Trust Management (Depends on CA-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| CA-3-001 | Implement CA certificate export (for user trust) | 2h | CA-2-002 | `CertificateAuthority.swift` |
| CA-3-002 | Implement programmatic trust installation | 4h | CA-3-001 | `CertificateAuthority.swift` |
| CA-3-003 | Implement trust status check | 2h | CA-3-002 | `CertificateAuthority.swift` |
| CA-3-004 | Create trust installation UI flow | 4h | CA-3-002 | `CertificateSettings.swift` |

### CA-4: Testing (Can run in parallel with CA-2/CA-3)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| CA-4-001 | Unit tests for key generation | 2h | CA-1-002 | `OISPCoreTests/CertificateAuthorityTests.swift` |
| CA-4-002 | Unit tests for CA certificate generation | 2h | CA-2-002 | Same |
| CA-4-003 | Unit tests for per-host certificate generation | 2h | CA-2-003 | Same |
| CA-4-004 | Integration test: full certificate chain validation | 4h | CA-2-003 | Same |
| CA-4-005 | Unit tests for Keychain operations | 2h | CA-1-004 | Same |

---

## Workstream B: Network Extension

### EXT-1: Project Setup (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| EXT-1-001 | Create Network Extension target in Xcode | 2h | None | `OISPNetworkExtension/` directory |
| EXT-1-002 | Configure entitlements for transparent proxy | 2h | EXT-1-001 | `OISPNetworkExtension.entitlements` |
| EXT-1-003 | Configure Info.plist for system extension | 2h | EXT-1-001 | `Info.plist` |
| EXT-1-004 | Set up code signing for extension | 2h | EXT-1-001 | Xcode project settings |
| EXT-1-005 | Create NETransparentProxyProvider subclass skeleton | 2h | EXT-1-001 | `TransparentProxyProvider.swift` |

### EXT-2: Flow Interception (Depends on EXT-1)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| EXT-2-001 | Implement startProxy() lifecycle method | 4h | EXT-1-005 | `TransparentProxyProvider.swift` |
| EXT-2-002 | Implement stopProxy() lifecycle method | 2h | EXT-1-005 | `TransparentProxyProvider.swift` |
| EXT-2-003 | Implement handleNewFlow() for TCP connections | 4h | EXT-2-001 | `TransparentProxyProvider.swift` |
| EXT-2-004 | Implement flow.open() for reading/writing | 4h | EXT-2-003 | `TransparentProxyProvider.swift` |
| EXT-2-005 | Implement flow close/cleanup | 2h | EXT-2-004 | `TransparentProxyProvider.swift` |
| EXT-2-006 | Add error handling and logging | 4h | EXT-2-003 | `TransparentProxyProvider.swift` |

### EXT-3: AI Endpoint Filtering (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| EXT-3-001 | Create AIEndpointFilter class | 2h | None | `AIEndpointFilter.swift` |
| EXT-3-002 | Implement static endpoint list | 1h | EXT-3-001 | Same |
| EXT-3-003 | Implement wildcard matching for subdomains | 2h | EXT-3-001 | Same |
| EXT-3-004 | Implement shouldIntercept() method | 2h | EXT-3-002 | Same |
| EXT-3-005 | Add user-configurable custom endpoints | 4h | EXT-3-004 | Same + Configuration |
| EXT-3-006 | Unit tests for endpoint filtering | 2h | EXT-3-004 | `AIEndpointFilterTests.swift` |

### EXT-4: Process Attribution (No dependencies on EXT-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| EXT-4-001 | Research NEAppProxyFlow metadata APIs | 2h | None | Research notes |
| EXT-4-002 | Create ProcessInfo struct | 2h | None | `ProcessAttribution.swift` |
| EXT-4-003 | Implement PID extraction from audit token | 4h | EXT-4-001 | `ProcessAttribution.swift` |
| EXT-4-004 | Implement libproc calls for process details | 4h | EXT-4-003 | `ProcessAttribution.swift` |
| EXT-4-005 | Get executable path via proc_pidpath | 2h | EXT-4-004 | Same |
| EXT-4-006 | Get process name via proc_pidinfo | 2h | EXT-4-004 | Same |
| EXT-4-007 | Get parent PID and UID | 2h | EXT-4-004 | Same |
| EXT-4-008 | Unit tests for process attribution | 4h | EXT-4-007 | `ProcessAttributionTests.swift` |

### EXT-5: Connection Management (Depends on EXT-2, EXT-4)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| EXT-5-001 | Create ConnectionManager class | 2h | None | `ConnectionManager.swift` |
| EXT-5-002 | Implement connection tracking (flow → metadata) | 4h | EXT-5-001 | Same |
| EXT-5-003 | Implement connection cleanup on close | 2h | EXT-5-002 | Same |
| EXT-5-004 | Implement connection timeout handling | 4h | EXT-5-002 | Same |
| EXT-5-005 | Add connection statistics | 2h | EXT-5-002 | Same |

---

## Workstream C: TLS MITM Engine

### TLS-1: TLS Session Abstraction (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| TLS-1-001 | Create TLSSession protocol/class | 2h | None | `TLSEngine.swift` |
| TLS-1-002 | Implement async read() method | 4h | TLS-1-001 | Same |
| TLS-1-003 | Implement async write() method | 4h | TLS-1-001 | Same |
| TLS-1-004 | Implement close() method | 2h | TLS-1-001 | Same |
| TLS-1-005 | Add connection state tracking | 2h | TLS-1-001 | Same |

### TLS-2: Client-Side TLS (We act as server) (Depends on CA-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| TLS-2-001 | Create TLSInterceptor class | 2h | None | `TLSInterceptor.swift` |
| TLS-2-002 | Implement acceptClient() method skeleton | 2h | TLS-2-001 | Same |
| TLS-2-003 | Create NWProtocolTLS.Options for server mode | 4h | CA-2-003 | Same |
| TLS-2-004 | Set local identity from generated certificate | 4h | CA-2-003, TLS-2-003 | Same |
| TLS-2-005 | Implement TLS handshake with client | 8h | TLS-2-004 | Same |
| TLS-2-006 | Return TLSSession after successful handshake | 2h | TLS-1-001, TLS-2-005 | Same |
| TLS-2-007 | Handle handshake failures gracefully | 4h | TLS-2-005 | Same |

### TLS-3: Server-Side TLS (We act as client) (No CA dependency)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| TLS-3-001 | Implement connectToServer() method skeleton | 2h | TLS-2-001 | `TLSInterceptor.swift` |
| TLS-3-002 | Create NWConnection with TLS | 4h | TLS-3-001 | Same |
| TLS-3-003 | Configure TLS options (system trust store) | 2h | TLS-3-002 | Same |
| TLS-3-004 | Implement connection state handler | 4h | TLS-3-002 | Same |
| TLS-3-005 | Wrap in async/await with continuation | 4h | TLS-3-004 | Same |
| TLS-3-006 | Return TLSSession after connection ready | 2h | TLS-1-001, TLS-3-005 | Same |
| TLS-3-007 | Handle connection failures gracefully | 4h | TLS-3-005 | Same |

### TLS-4: Bidirectional Relay (Depends on TLS-2, TLS-3)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| TLS-4-001 | Create relay() async function | 2h | TLS-1-001 | `TLSInterceptor.swift` |
| TLS-4-002 | Implement client→server data forwarding | 4h | TLS-4-001 | Same |
| TLS-4-003 | Implement server→client data forwarding | 4h | TLS-4-001 | Same |
| TLS-4-004 | Add data capture hooks for both directions | 4h | TLS-4-002, TLS-4-003 | Same |
| TLS-4-005 | Implement concurrent bidirectional relay | 4h | TLS-4-002, TLS-4-003 | Same |
| TLS-4-006 | Handle connection close from either side | 4h | TLS-4-005 | Same |
| TLS-4-007 | Add error handling and recovery | 4h | TLS-4-005 | Same |

### TLS-5: Testing (Can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| TLS-5-001 | Create mock TLS server for testing | 4h | None | `TLSInterceptorTests.swift` |
| TLS-5-002 | Unit test acceptClient() | 4h | TLS-2-005 | Same |
| TLS-5-003 | Unit test connectToServer() | 4h | TLS-3-005 | Same |
| TLS-5-004 | Integration test: full MITM flow | 8h | TLS-4-005 | Same |
| TLS-5-005 | Test with real OpenAI endpoint | 4h | TLS-4-005 | Same |

---

## Workstream D: Menu Bar App UI

### UI-1: App Shell (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-1-001 | Create macOS app target in Xcode | 2h | None | `OISPApp/` directory |
| UI-1-002 | Configure as menu bar only app (LSUIElement) | 1h | UI-1-001 | `Info.plist` |
| UI-1-003 | Create AppDelegate with NSStatusItem | 2h | UI-1-001 | `AppDelegate.swift` |
| UI-1-004 | Create NSPopover for dropdown | 2h | UI-1-003 | Same |
| UI-1-005 | Add menu bar icon (SF Symbol) | 1h | UI-1-003 | Same |
| UI-1-006 | Implement popover toggle on click | 2h | UI-1-004 | Same |

### UI-2: Menu Bar Content (Depends on UI-1)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-2-001 | Create MenuBarView SwiftUI view | 2h | UI-1-004 | `MenuBarView.swift` |
| UI-2-002 | Create MenuBarViewModel | 2h | UI-2-001 | `MenuBarViewModel.swift` |
| UI-2-003 | Implement status header (capturing/paused) | 2h | UI-2-001 | `MenuBarView.swift` |
| UI-2-004 | Implement stats display (tokens, cost, latency) | 2h | UI-2-003 | Same |
| UI-2-005 | Create RecentRequest model | 1h | None | `Models/RecentRequest.swift` |
| UI-2-006 | Create RecentRequestRow view | 2h | UI-2-005 | `RecentRequestRow.swift` |
| UI-2-007 | Implement recent requests list | 2h | UI-2-006 | `MenuBarView.swift` |
| UI-2-008 | Add action buttons (Pause/Resume, Dashboard, Settings) | 2h | UI-2-001 | Same |
| UI-2-009 | Implement Quit functionality | 1h | UI-2-008 | Same |

### UI-3: Extension Status UI (Depends on UI-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-3-001 | Create ExtensionManager class | 2h | None | `Services/ExtensionManager.swift` |
| UI-3-002 | Implement extension status check | 4h | UI-3-001 | Same |
| UI-3-003 | Implement extension activation request | 4h | UI-3-001 | Same |
| UI-3-004 | Add extension status indicator to UI | 2h | UI-3-002, UI-2-001 | `MenuBarView.swift` |
| UI-3-005 | Add "Enable Extension" button | 2h | UI-3-003, UI-3-004 | Same |
| UI-3-006 | Handle System Settings deep link | 2h | UI-3-005 | Same |

### UI-4: CA Trust UI (Depends on UI-2, CA-3)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-4-001 | Add CA trust status indicator to UI | 2h | CA-3-003, UI-2-001 | `MenuBarView.swift` |
| UI-4-002 | Add "Trust Certificate" button | 2h | CA-3-002, UI-4-001 | Same |
| UI-4-003 | Create trust confirmation dialog | 2h | UI-4-002 | `CertificateSettings.swift` |
| UI-4-004 | Implement trust flow with password prompt | 4h | UI-4-003 | Same |

### UI-5: Settings Window (Can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-5-001 | Create Settings window | 2h | None | `SettingsWindow.swift` |
| UI-5-002 | Create GeneralSettings view | 4h | UI-5-001 | `Settings/GeneralSettings.swift` |
| UI-5-003 | Create CertificateSettings view | 4h | UI-5-001 | `Settings/CertificateSettings.swift` |
| UI-5-004 | Create ProviderSettings view | 4h | UI-5-001 | `Settings/ProviderSettings.swift` |
| UI-5-005 | Implement configuration persistence | 4h | UI-5-002 | `Configuration.swift` |
| UI-5-006 | Add launch at login toggle | 2h | UI-5-002 | Same |

### UI-6: Dashboard Window (Optional, can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| UI-6-001 | Create Dashboard window | 4h | None | `DashboardWindow.swift` |
| UI-6-002 | Implement request list view | 8h | UI-6-001 | Same |
| UI-6-003 | Implement request detail view | 8h | UI-6-002 | `RequestDetailView.swift` |
| UI-6-004 | Add filtering and search | 4h | UI-6-002 | Same |
| UI-6-005 | Add export functionality | 4h | UI-6-002 | Same |

---

## Workstream E: Swift↔Rust Bridge

### BRIDGE-1: Swift Side (No Rust dependency)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| BRIDGE-1-001 | Create RawCaptureEvent Swift model | 2h | None | `OISPCore/Sources/Models/RawCaptureEvent.swift` |
| BRIDGE-1-002 | Create RawEventKind enum | 1h | BRIDGE-1-001 | Same |
| BRIDGE-1-003 | Create RawEventMetadata struct | 1h | BRIDGE-1-001 | Same |
| BRIDGE-1-004 | Implement JSON serialization for events | 2h | BRIDGE-1-001 | Same |
| BRIDGE-1-005 | Create EventEmitter protocol | 2h | None | `OISPCore/Sources/IPC/EventEmitter.swift` |
| BRIDGE-1-006 | Create UnixSocketClient class | 4h | None | `OISPCore/Sources/IPC/UnixSocketBridge.swift` |
| BRIDGE-1-007 | Implement socket connection | 4h | BRIDGE-1-006 | Same |
| BRIDGE-1-008 | Implement event sending | 4h | BRIDGE-1-006, BRIDGE-1-004 | Same |
| BRIDGE-1-009 | Implement reconnection logic | 4h | BRIDGE-1-007 | Same |
| BRIDGE-1-010 | Implement EventEmitter using UnixSocketClient | 2h | BRIDGE-1-005, BRIDGE-1-008 | Same |

### BRIDGE-2: Rust Side (No Swift dependency)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| BRIDGE-2-001 | Create oisp-capture-macos crate | 2h | None | `oisp-sensor/crates/oisp-capture-macos/Cargo.toml` |
| BRIDGE-2-002 | Create MacOSCapture struct implementing CapturePlugin | 4h | BRIDGE-2-001 | `src/lib.rs` |
| BRIDGE-2-003 | Create Unix socket server | 4h | BRIDGE-2-001 | `src/socket_server.rs` |
| BRIDGE-2-004 | Implement async accept loop | 4h | BRIDGE-2-003 | Same |
| BRIDGE-2-005 | Implement JSON deserialization for events | 2h | BRIDGE-2-003 | Same |
| BRIDGE-2-006 | Convert JSON to RawCaptureEvent | 4h | BRIDGE-2-005 | Same |
| BRIDGE-2-007 | Emit to mpsc channel | 2h | BRIDGE-2-006 | Same |
| BRIDGE-2-008 | Handle reconnections | 4h | BRIDGE-2-004 | Same |
| BRIDGE-2-009 | Add error handling and logging | 4h | BRIDGE-2-004 | Same |

### BRIDGE-3: Integration (Depends on BRIDGE-1, BRIDGE-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| BRIDGE-3-001 | End-to-end test: Swift → Rust event flow | 4h | BRIDGE-1-010, BRIDGE-2-007 | Test files |
| BRIDGE-3-002 | Test event serialization round-trip | 2h | BRIDGE-3-001 | Same |
| BRIDGE-3-003 | Test reconnection behavior | 4h | BRIDGE-1-009, BRIDGE-2-008 | Same |
| BRIDGE-3-004 | Performance test: events per second | 4h | BRIDGE-3-001 | Same |
| BRIDGE-3-005 | Verify HttpDecoder processes macOS events | 4h | BRIDGE-3-001 | Same |

---

## Workstream F: Distribution & Packaging

**Primary Distribution: DMG** (not App Store - System Extensions are prohibited there)

### DIST-1: Build System (No dependencies)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-1-001 | Create build-release.sh script | 4h | None | `Scripts/build-release.sh` |
| DIST-1-002 | Configure archive and export in Xcode | 2h | None | Xcode project |
| DIST-1-003 | Set up GitHub Actions for CI | 8h | DIST-1-001 | `.github/workflows/ci.yml` |
| DIST-1-004 | Add version number automation | 2h | DIST-1-001 | Scripts |
| DIST-1-005 | Configure code signing for distribution | 4h | None | Xcode project |

### DIST-2: DMG Creation - PRIMARY (Depends on DIST-1)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-2-001 | Create DMG background image (drag-to-install visual) | 2h | None | `Distribution/dmg-background.png` |
| DIST-2-002 | Create app icon (1024x1024 + all sizes) | 4h | None | `OISPApp/Resources/Assets.xcassets` |
| DIST-2-003 | Create create-dmg.sh script using create-dmg tool | 4h | DIST-1-001 | `Scripts/create-dmg.sh` |
| DIST-2-004 | Configure DMG layout (icon positions, window size) | 2h | DIST-2-003 | Same |
| DIST-2-005 | Add Applications folder symlink to DMG | 1h | DIST-2-003 | Same |
| DIST-2-006 | Test DMG on fresh macOS install | 2h | DIST-2-003 | None |

### DIST-3: Notarization - REQUIRED for DMG (Depends on DIST-2)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-3-001 | Set up Apple Developer account ($99/year) | 2h | None | Account |
| DIST-3-002 | Create app-specific password for notarytool | 1h | DIST-3-001 | Keychain profile |
| DIST-3-003 | Create notarize.sh script | 4h | DIST-3-002 | `Scripts/notarize.sh` |
| DIST-3-004 | Implement notarytool submit --wait | 2h | DIST-3-003 | Same |
| DIST-3-005 | Implement xcrun stapler staple for DMG | 2h | DIST-3-004 | Same |
| DIST-3-006 | Add notarization to CI pipeline | 4h | DIST-3-005, DIST-1-003 | CI workflow |
| DIST-3-007 | Test Gatekeeper approval on fresh Mac | 2h | DIST-3-005 | None |

### DIST-4: GitHub Releases (Depends on DIST-3)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-4-001 | Create release workflow in GitHub Actions | 4h | DIST-3-006 | `.github/workflows/release.yml` |
| DIST-4-002 | Auto-upload DMG to GitHub releases on tag | 2h | DIST-4-001 | Same |
| DIST-4-003 | Generate release notes from commits | 2h | DIST-4-001 | Same |
| DIST-4-004 | Add SHA256 checksum to release | 1h | DIST-4-002 | Same |

### DIST-5: Homebrew Cask - OPTIONAL (Depends on DIST-4)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-5-001 | Create Homebrew tap repository (oisp/homebrew-tap) | 2h | None | Separate repo |
| DIST-5-002 | Create cask formula pointing to GitHub release DMG | 4h | DIST-5-001, DIST-4-002 | `Casks/oisp.rb` |
| DIST-5-003 | Implement update-homebrew.sh for version bumps | 2h | DIST-5-002 | `Scripts/update-homebrew.sh` |
| DIST-5-004 | Test `brew install --cask oisp/tap/oisp` | 2h | DIST-5-002 | None |

### DIST-6: Website/Landing Page - OPTIONAL (Can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DIST-6-001 | Create simple landing page with download button | 4h | None | Website repo or GitHub Pages |
| DIST-6-002 | Add installation instructions | 2h | DIST-6-001 | Same |
| DIST-6-003 | Add screenshots/demo GIF | 4h | UI complete | Same |

---

## Workstream G: Documentation

### DOCS-1: User Documentation (Can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DOCS-1-001 | Write installation guide | 4h | None | `docs/macos/installation.md` |
| DOCS-1-002 | Write quick start guide | 4h | None | `docs/macos/quickstart.md` |
| DOCS-1-003 | Write troubleshooting guide | 8h | None | `docs/macos/troubleshooting.md` |
| DOCS-1-004 | Write FAQ | 4h | None | `docs/macos/faq.md` |
| DOCS-1-005 | Create screenshots for docs | 4h | UI complete | `docs/macos/images/` |

### DOCS-2: Developer Documentation (Can run in parallel)

| Task ID | Task | Estimated Hours | Dependencies | Output Files |
|---------|------|-----------------|--------------|--------------|
| DOCS-2-001 | Write architecture overview | 4h | None | `docs/macos/architecture.md` |
| DOCS-2-002 | Write development setup guide | 4h | None | `docs/macos/development.md` |
| DOCS-2-003 | Write contributing guide | 2h | None | `CONTRIBUTING.md` |
| DOCS-2-004 | Document API/protocols | 4h | Code complete | `docs/macos/api.md` |
| DOCS-2-005 | Write testing guide | 4h | Tests written | `docs/macos/testing.md` |

---

## Task Dependency Graph

```
                                    ┌─────────────┐
                                    │   START     │
                                    └──────┬──────┘
                                           │
              ┌────────────────────────────┼────────────────────────────┐
              │                            │                            │
              ▼                            ▼                            ▼
       ┌──────────────┐            ┌──────────────┐            ┌──────────────┐
       │    CA-1      │            │    EXT-1     │            │    UI-1      │
       │  Foundation  │            │ Ext Setup    │            │  App Shell   │
       └──────┬───────┘            └──────┬───────┘            └──────┬───────┘
              │                            │                            │
              ▼                            ▼                            ▼
       ┌──────────────┐            ┌──────────────┐            ┌──────────────┐
       │    CA-2      │            │    EXT-2     │            │    UI-2      │
       │  Cert Gen    │            │ Flow Interc  │            │  Menu Bar    │
       └──────┬───────┘            └──────┬───────┘            └──────┬───────┘
              │                            │                            │
              │                    ┌───────┴───────┐                    │
              │                    │               │                    │
              ▼                    ▼               ▼                    │
       ┌──────────────┐    ┌──────────────┐ ┌──────────────┐           │
       │    TLS-2     │    │    EXT-3     │ │    EXT-4     │           │
       │ Client TLS   │    │   Filtering  │ │ Process Attr │           │
       └──────┬───────┘    └──────────────┘ └──────┬───────┘           │
              │                                    │                    │
              ├────────────────────────────────────┤                    │
              │                                    │                    │
              ▼                                    ▼                    ▼
       ┌──────────────┐                    ┌──────────────┐    ┌──────────────┐
       │    TLS-3     │                    │    EXT-5     │    │    UI-3      │
       │ Server TLS   │                    │ Conn Mgmt    │    │  Ext Status  │
       └──────┬───────┘                    └──────┬───────┘    └──────┬───────┘
              │                                    │                    │
              ▼                                    │                    │
       ┌──────────────┐                            │                    │
       │    TLS-4     │                            │                    │
       │   Relay      │◄───────────────────────────┘                    │
       └──────┬───────┘                                                 │
              │                                                         │
              │                    ┌──────────────┐                     │
              │                    │  BRIDGE-1    │                     │
              │                    │ Swift Side   │                     │
              │                    └──────┬───────┘                     │
              │                           │                             │
              │                    ┌──────┴───────┐                     │
              │                    │  BRIDGE-2    │                     │
              │                    │  Rust Side   │                     │
              │                    └──────┬───────┘                     │
              │                           │                             │
              └───────────────────────────┼─────────────────────────────┘
                                          │
                                          ▼
                                   ┌──────────────┐
                                   │  BRIDGE-3    │
                                   │ Integration  │
                                   └──────┬───────┘
                                          │
                                          ▼
                                   ┌──────────────┐
                                   │    DIST      │
                                   │ Distribution │
                                   └──────┬───────┘
                                          │
                                          ▼
                                   ┌──────────────┐
                                   │   RELEASE    │
                                   └──────────────┘
```

---

## Parallelization Strategy

### Maximum Parallelism (8+ agents)

At any given time, these workstreams can run completely independently:

| Time | Parallel Workstreams |
|------|---------------------|
| Week 1-2 | CA-1, EXT-1, EXT-3, EXT-4, UI-1, TLS-1, BRIDGE-1, BRIDGE-2, DOCS-1, DOCS-2 |
| Week 3-4 | CA-2, EXT-2, UI-2, TLS-2, TLS-3, UI-5, DIST-1 |
| Week 5-6 | CA-3, CA-4, EXT-5, TLS-4, UI-3, UI-4, DIST-2 |
| Week 7-8 | TLS-5, BRIDGE-3, UI-6, DIST-3 |
| Week 9+ | DIST-4, Final integration, Testing |

### Recommended Team Allocation

| Agent/Engineer | Primary Focus | Secondary Focus |
|----------------|---------------|-----------------|
| Agent 1 | CA (all) | TLS-2 |
| Agent 2 | EXT-1, EXT-2 | EXT-5 |
| Agent 3 | EXT-3, EXT-4 | - |
| Agent 4 | TLS-1, TLS-3 | TLS-4 |
| Agent 5 | UI-1, UI-2, UI-3 | - |
| Agent 6 | UI-4, UI-5, UI-6 | - |
| Agent 7 | BRIDGE-1, BRIDGE-2 | BRIDGE-3 |
| Agent 8 | DIST-1, DIST-2 | DIST-3, DIST-4 |
| Agent 9 | DOCS-1 | - |
| Agent 10 | DOCS-2 | - |
| Agent 11 | All tests (CA-4, TLS-5) | - |
| Agent 12 | Integration testing | - |

---

## Task Assignment Template

When assigning a task to an agent, use this template:

```markdown
## Task: [TASK_ID]

### Description
[Detailed description of what needs to be implemented]

### Input
- Dependencies: [List of completed task IDs this depends on]
- Files to read: [List of files to understand context]

### Output
- Files to create/modify: [Specific file paths]
- Tests to write: [Test file paths if applicable]

### Acceptance Criteria
- [ ] [Specific criteria 1]
- [ ] [Specific criteria 2]
- [ ] [Specific criteria 3]

### Context
[Any additional context, links to Apple docs, etc.]

### Estimated Time
[X hours]
```

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Total Tasks | ~130 |
| Total Estimated Hours | ~380h |
| Parallelizable at Week 1 | 10 workstreams |
| Critical Path Length | ~8 weeks |
| Maximum Agents Useful | 12 |

### Distribution Summary

| Method | Status | Effort |
|--------|--------|--------|
| **DMG** | PRIMARY | ~30h |
| **GitHub Releases** | Required | ~10h |
| **Notarization** | Required for DMG | ~15h |
| **Homebrew Cask** | Optional | ~10h |
| **App Store** | NOT POSSIBLE | N/A |

**Why DMG + Notarization:**
- System Extensions cannot be distributed via App Store
- Notarization allows Gatekeeper to approve without App Store
- Users download DMG → drag to Applications → approve extension
- Works for everyone, no technical knowledge required

With full parallelization, the implementation can be completed in approximately **8-10 weeks** with a team of 8-12 agents working simultaneously.
