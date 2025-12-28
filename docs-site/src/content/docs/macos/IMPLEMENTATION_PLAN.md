# OISP macOS Implementation Plan

## Executive Summary

This document outlines the complete implementation plan for OISP (Observability for Intelligent Systems Platform) on macOS. The goal is to capture AI/LLM API traffic (OpenAI, Anthropic, Google, etc.) from ANY application on macOS - browsers, Python scripts, Node.js apps, CLI tools like Claude Code, etc.

**Target:** Full Linux feature parity with identical event output format.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Repository Structure](#2-repository-structure)
3. [Technical Approach](#3-technical-approach)
4. [Component Specifications](#4-component-specifications)
5. [Data Flow](#5-data-flow)
6. [Installation & Distribution](#6-installation--distribution)
7. [Development Phases](#7-development-phases)
8. [Parallel Workstreams](#8-parallel-workstreams)
9. [Testing Strategy](#9-testing-strategy)
10. [Security Considerations](#10-security-considerations)

---

## 1. Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            macOS System                                  │
│                                                                          │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│   │  Chrome  │  │  Python  │  │  Node.js │  │  Claude  │  │   curl   │ │
│   │          │  │          │  │          │  │   Code   │  │          │ │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
│        │             │             │             │             │        │
│        └─────────────┴──────┬──────┴─────────────┴─────────────┘        │
│                             │                                            │
│                             ▼                                            │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │              OISPNetworkExtension.appex                          │   │
│   │              (System Extension)                                  │   │
│   │                                                                  │   │
│   │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │   │
│   │  │ Transparent     │  │  TLS MITM       │  │  Process        │  │   │
│   │  │ Proxy Provider  │  │  Engine         │  │  Attribution    │  │   │
│   │  │                 │  │                 │  │                 │  │   │
│   │  │ - Intercept TCP │  │ - Local CA cert │  │ - Map flow→PID  │  │   │
│   │  │ - Filter by SNI │  │ - Decrypt TLS   │  │ - Get process   │  │   │
│   │  │ - Route to MITM │  │ - Re-encrypt    │  │   metadata      │  │   │
│   │  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │   │
│   │           │                    │                    │           │   │
│   │           └────────────────────┼────────────────────┘           │   │
│   │                                │                                │   │
│   │                                ▼                                │   │
│   │                    ┌─────────────────────┐                      │   │
│   │                    │  Event Emitter      │                      │   │
│   │                    │  (XPC/Unix Socket)  │                      │   │
│   │                    └──────────┬──────────┘                      │   │
│   └───────────────────────────────┼─────────────────────────────────┘   │
│                                   │                                      │
│                                   │ RawCaptureEvent                      │
│                                   │ (SslRead/SslWrite + plaintext)       │
│                                   ▼                                      │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                      oisp-sensor (Rust)                          │   │
│   │                      (Existing - shared with Linux)              │   │
│   │                                                                  │   │
│   │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │   │
│   │  │HttpDecoder  │  │  Enrichers  │  │   Actions   │  │ Export  │ │   │
│   │  │             │  │             │  │             │  │         │ │   │
│   │  │- Parse HTTP │  │- Host info  │  │- Redaction  │  │- JSONL  │ │   │
│   │  │- Detect AI  │  │- Process    │  │- Filtering  │  │- WebSoc │ │   │
│   │  │- Correlate  │  │  tree       │  │             │  │- Kafka  │ │   │
│   │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────┘ │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                   │                                      │
│                                   ▼                                      │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                    OISP Menu Bar App                             │   │
│   │                    (SwiftUI)                                     │   │
│   │                                                                  │   │
│   │  ┌──────────────────────────────────────────────────────────┐   │   │
│   │  │  [●] Capturing    47 requests    ⚙️                       │   │   │
│   │  │  ─────────────────────────────────────────────────────── │   │   │
│   │  │  Recent:                                                  │   │   │
│   │  │  • OpenAI gpt-4 - 1.2s - 450 tokens                      │   │   │
│   │  │  • Anthropic claude-3 - 2.1s - 892 tokens                │   │   │
│   │  │  ─────────────────────────────────────────────────────── │   │   │
│   │  │  [Open Dashboard]  [Pause]  [Settings]                   │   │   │
│   │  └──────────────────────────────────────────────────────────┘   │   │
│   └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Comparison with Linux Implementation

| Aspect | Linux | macOS |
|--------|-------|-------|
| **Capture Method** | eBPF uprobes on SSL_read/SSL_write | Network Extension + TLS MITM |
| **Capture Point** | Inside process (function call) | Network stack (packet level) |
| **Certificate Required** | No | Yes (local CA must be trusted) |
| **Root/Admin Required** | Yes (CAP_BPF) | No (user approval for extension) |
| **Process Attribution** | Direct from uprobe context | Via NEAppProxyFlow metadata |
| **Output Format** | RawCaptureEvent | RawCaptureEvent (identical) |
| **Decode Pipeline** | oisp-decode (shared) | oisp-decode (shared) |
| **Event Schema** | OISP events (shared) | OISP events (shared) |

### 1.3 Key Design Principles

1. **Shared Decode Layer**: 100% code reuse for HTTP parsing, AI detection, event generation
2. **Platform-Specific Capture**: Only the capture layer differs between platforms
3. **Identical Output**: Same `AiRequestEvent`/`AiResponseEvent` schema on both platforms
4. **Minimal User Friction**: One-time setup, then transparent operation
5. **Privacy First**: Local-only processing, optional redaction, no cloud dependency

---

## 2. Repository Structure

### 2.1 New Directory Layout

```
oisp/
├── oisp-sensor/                    # Existing Rust codebase
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── oisp-core/              # ✅ Shared - event types, traits
│   │   ├── oisp-decode/            # ✅ Shared - HTTP/AI parsing
│   │   ├── oisp-export/            # ✅ Shared - output destinations
│   │   ├── oisp-capture-ebpf/      # ✅ Linux only
│   │   ├── oisp-capture-macos/     # 🆕 macOS capture plugin
│   │   │   ├── Cargo.toml
│   │   │   ├── src/
│   │   │   │   ├── lib.rs          # Plugin interface
│   │   │   │   ├── extension_bridge.rs  # XPC/socket to extension
│   │   │   │   └── process_info.rs # macOS process metadata
│   │   │   └── build.rs
│   │   ├── oisp-tui/               # ✅ Shared - terminal UI
│   │   └── oisp-web/               # ✅ Shared - web dashboard
│   └── src/
│       └── main.rs                 # CLI entry point
│
├── oisp-macos/                     # 🆕 macOS-specific components
│   ├── IMPLEMENTATION_PLAN.md      # This document
│   ├── OISPApp/                    # 🆕 Menu bar app (Swift/SwiftUI)
│   │   ├── OISPApp.xcodeproj/
│   │   ├── OISPApp/
│   │   │   ├── OISPApp.swift       # @main entry point
│   │   │   ├── AppDelegate.swift   # Menu bar setup
│   │   │   ├── MenuBarView.swift   # Dropdown UI
│   │   │   ├── DashboardWindow.swift
│   │   │   ├── Settings/
│   │   │   │   ├── GeneralSettings.swift
│   │   │   │   ├── CertificateSettings.swift
│   │   │   │   └── ProviderSettings.swift
│   │   │   ├── Services/
│   │   │   │   ├── ExtensionManager.swift
│   │   │   │   ├── CertificateManager.swift
│   │   │   │   └── SensorBridge.swift
│   │   │   └── Resources/
│   │   │       ├── Assets.xcassets
│   │   │       └── Info.plist
│   │   ├── OISPAppTests/
│   │   └── OISPAppUITests/
│   │
│   ├── OISPNetworkExtension/       # 🆕 System Extension (Swift)
│   │   ├── Info.plist
│   │   ├── OISPNetworkExtension.entitlements
│   │   ├── TransparentProxyProvider.swift
│   │   ├── TLSInterceptor.swift
│   │   ├── ConnectionManager.swift
│   │   ├── ProcessAttribution.swift
│   │   ├── EventEmitter.swift
│   │   └── AIEndpointFilter.swift
│   │
│   ├── OISPCore/                   # 🆕 Shared Swift framework
│   │   ├── Sources/
│   │   │   ├── Models/
│   │   │   │   ├── RawCaptureEvent.swift
│   │   │   │   └── Configuration.swift
│   │   │   ├── Networking/
│   │   │   │   ├── TLSEngine.swift
│   │   │   │   └── CertificateAuthority.swift
│   │   │   └── IPC/
│   │   │       ├── XPCProtocol.swift
│   │   │       └── UnixSocketBridge.swift
│   │   └── Tests/
│   │
│   ├── Scripts/                    # 🆕 Build & distribution scripts
│   │   ├── build-release.sh
│   │   ├── create-dmg.sh
│   │   ├── notarize.sh
│   │   └── update-homebrew.sh
│   │
│   └── Distribution/               # 🆕 Installer resources
│       ├── dmg-background.png
│       ├── entitlements.plist
│       └── homebrew-formula.rb
│
├── oisp-spec/                      # ✅ Existing - event specifications
├── oisp-cookbook/                  # ✅ Existing - examples
└── docs/
    └── macos/                      # 🆕 macOS-specific docs
        ├── installation.md
        ├── troubleshooting.md
        └── development.md
```

### 2.2 File Responsibilities

| File | Responsibility | Dependencies |
|------|---------------|--------------|
| `TransparentProxyProvider.swift` | Intercept TCP connections, route to MITM | NetworkExtension framework |
| `TLSInterceptor.swift` | Decrypt/re-encrypt TLS traffic | Security framework, Network framework |
| `ProcessAttribution.swift` | Map network flows to PIDs | libproc, NEAppProxyFlow |
| `EventEmitter.swift` | Send RawCaptureEvent to Rust sensor | XPC or Unix socket |
| `CertificateAuthority.swift` | Generate and manage local CA | Security framework |
| `ExtensionManager.swift` | Install/enable/disable system extension | SystemExtensions framework |
| `MenuBarView.swift` | User-facing status and controls | SwiftUI |

---

## 3. Technical Approach

### 3.1 Network Extension Strategy

We use `NETransparentProxyProvider` (not `NEAppProxyProvider`) because:

1. **Transparent**: Apps don't need to be proxy-aware
2. **System-wide**: Captures all TCP traffic
3. **Selective**: We filter to only AI API endpoints

```swift
// NETransparentProxyProvider lifecycle
class OISPTransparentProxy: NETransparentProxyProvider {

    // Called when extension starts
    override func startProxy(options: [String : Any]?, completionHandler: @escaping (Error?) -> Void)

    // Called for EVERY new TCP connection
    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool

    // Called when extension stops
    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void)
}
```

### 3.2 TLS MITM Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                      TLS MITM Flow                               │
│                                                                  │
│  Client (Python)              OISP Extension           OpenAI    │
│       │                            │                      │      │
│       │  ──── TLS ClientHello ───► │                      │      │
│       │                            │                      │      │
│       │  ◄─── TLS ServerHello ──── │                      │      │
│       │       (OISP CA signed      │                      │      │
│       │        cert for            │                      │      │
│       │        api.openai.com)     │                      │      │
│       │                            │                      │      │
│       │  ════ TLS Established ════ │                      │      │
│       │       (to OISP)            │                      │      │
│       │                            │  ── TLS ClientHello ─►│      │
│       │                            │                      │      │
│       │                            │  ◄─ TLS ServerHello ──│      │
│       │                            │                      │      │
│       │                            │  ═══ TLS Established ═│      │
│       │                            │      (to OpenAI)     │      │
│       │                            │                      │      │
│       │  ──── HTTP Request ──────► │                      │      │
│       │       (plaintext to OISP)  │                      │      │
│       │                            │  ── HTTP Request ────►│      │
│       │                            │     (re-encrypted)   │      │
│       │                            │                      │      │
│       │                            │  ◄─ HTTP Response ───│      │
│       │                            │     (encrypted)      │      │
│       │  ◄─── HTTP Response ────── │                      │      │
│       │       (plaintext to OISP)  │                      │      │
│       │                            │                      │      │
│       │                       [CAPTURE POINT]             │      │
│       │                       Emit RawCaptureEvent        │      │
│       │                       with plaintext HTTP         │      │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 AI Endpoint Filtering

Only intercept traffic to known AI API endpoints:

```swift
struct AIEndpointFilter {
    static let endpoints = [
        // OpenAI
        "api.openai.com",

        // Anthropic
        "api.anthropic.com",

        // Google
        "generativelanguage.googleapis.com",
        "aiplatform.googleapis.com",

        // Azure OpenAI
        "*.openai.azure.com",

        // AWS Bedrock
        "bedrock-runtime.*.amazonaws.com",

        // Cohere
        "api.cohere.ai",

        // Mistral
        "api.mistral.ai",

        // Groq
        "api.groq.com",

        // Together AI
        "api.together.xyz",

        // Fireworks
        "api.fireworks.ai",

        // Perplexity
        "api.perplexity.ai",

        // OpenRouter
        "openrouter.ai",

        // Local/Self-hosted (configurable)
        // User can add custom endpoints
    ]

    static func shouldIntercept(host: String) -> Bool {
        endpoints.contains { pattern in
            if pattern.contains("*") {
                // Wildcard matching
                let regex = pattern.replacingOccurrences(of: ".", with: "\\.")
                                   .replacingOccurrences(of: "*", with: ".*")
                return host.range(of: regex, options: .regularExpression) != nil
            } else {
                return host == pattern || host.hasSuffix("." + pattern)
            }
        }
    }
}
```

### 3.4 Process Attribution

Getting the PID for each network connection:

```swift
// NEAppProxyFlow provides app metadata
func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
    guard let tcpFlow = flow as? NEAppProxyTCPFlow else { return false }

    // Get process information from flow metadata
    let appAuditToken = flow.metaData.sourceAppAuditToken
    let pid = getPidFromAuditToken(appAuditToken)

    // Get additional process info
    let processInfo = ProcessInfo(pid: pid)
    // - executable path
    // - process name
    // - parent PID
    // - user ID

    // Store for correlation when emitting events
    connectionManager.associate(flow: tcpFlow, process: processInfo)

    return true
}

// Using libproc to get process details
func getProcessInfo(pid: pid_t) -> ProcessMetadata {
    var pathBuffer = [CChar](repeating: 0, count: Int(PROC_PIDPATHINFO_MAXSIZE))
    proc_pidpath(pid, &pathBuffer, UInt32(pathBuffer.count))
    let path = String(cString: pathBuffer)

    var info = proc_bsdinfo()
    proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, Int32(MemoryLayout<proc_bsdinfo>.size))

    return ProcessMetadata(
        pid: UInt32(pid),
        ppid: UInt32(info.pbi_ppid),
        exe: path,
        comm: String(cString: &info.pbi_name.0),
        uid: info.pbi_uid
    )
}
```

### 3.5 Event Bridge to Rust

Communication between Swift extension and Rust sensor:

```
Option A: XPC Service (Preferred)
─────────────────────────────────
┌─────────────────┐     XPC      ┌─────────────────┐
│ Network         │ ◄──────────► │ OISP App        │
│ Extension       │              │ (contains XPC   │
│ (.appex)        │              │  service)       │
└─────────────────┘              └────────┬────────┘
                                          │
                                          │ Unix Socket / FFI
                                          ▼
                                 ┌─────────────────┐
                                 │ oisp-sensor     │
                                 │ (Rust daemon)   │
                                 └─────────────────┘

Option B: Unix Domain Socket (Simpler)
──────────────────────────────────────
┌─────────────────┐   Unix Socket    ┌─────────────────┐
│ Network         │ ────────────────►│ oisp-sensor     │
│ Extension       │                  │ (Rust daemon)   │
│ (.appex)        │                  │                 │
└─────────────────┘                  └─────────────────┘
                    /tmp/oisp.sock
```

Event format over the bridge (JSON for simplicity):

```json
{
  "id": "01HQXYZ...",
  "timestamp_ns": 1703123456789000000,
  "kind": "SslRead",
  "pid": 12345,
  "tid": 12346,
  "data": "base64-encoded-plaintext-http",
  "metadata": {
    "comm": "python3",
    "exe": "/usr/bin/python3",
    "uid": 501,
    "fd": 5
  }
}
```

---

## 4. Component Specifications

### 4.1 Network Extension Component

**File:** `OISPNetworkExtension/TransparentProxyProvider.swift`

```swift
import NetworkExtension

class OISPTransparentProxyProvider: NETransparentProxyProvider {

    private let tlsInterceptor = TLSInterceptor()
    private let connectionManager = ConnectionManager()
    private let eventEmitter = EventEmitter()
    private let filter = AIEndpointFilter()

    // MARK: - Lifecycle

    override func startProxy(options: [String : Any]?, completionHandler: @escaping (Error?) -> Void) {
        // 1. Load configuration
        // 2. Initialize TLS engine with CA cert
        // 3. Connect to oisp-sensor
        // 4. Start accepting connections
        completionHandler(nil)
    }

    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        // 1. Close all active connections
        // 2. Disconnect from oisp-sensor
        // 3. Cleanup
        completionHandler()
    }

    // MARK: - Flow Handling

    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        guard let tcpFlow = flow as? NEAppProxyTCPFlow else { return false }

        // Get destination
        guard let endpoint = tcpFlow.remoteEndpoint as? NWHostEndpoint else { return false }
        let host = endpoint.hostname
        let port = endpoint.port

        // Only intercept HTTPS to AI endpoints
        guard port == "443" && filter.shouldIntercept(host: host) else {
            return false  // Let it pass through unmodified
        }

        // Get process info
        let processInfo = getProcessInfo(from: tcpFlow)

        // Start MITM session
        Task {
            await interceptConnection(flow: tcpFlow, host: host, process: processInfo)
        }

        return true  // We're handling this flow
    }

    // MARK: - MITM

    private func interceptConnection(flow: NEAppProxyTCPFlow, host: String, process: ProcessInfo) async {
        do {
            // 1. Open flow for reading/writing
            flow.open(withLocalEndpoint: nil) { error in
                if let error = error {
                    print("Failed to open flow: \(error)")
                    return
                }
            }

            // 2. Perform TLS handshake with client (using our CA)
            let clientSession = try await tlsInterceptor.acceptClient(flow: flow, serverName: host)

            // 3. Connect to real server
            let serverSession = try await tlsInterceptor.connectToServer(host: host, port: 443)

            // 4. Relay data, capturing plaintext
            await relayWithCapture(
                client: clientSession,
                server: serverSession,
                host: host,
                process: process
            )

        } catch {
            print("MITM failed for \(host): \(error)")
        }
    }

    private func relayWithCapture(client: TLSSession, server: TLSSession, host: String, process: ProcessInfo) async {
        // Bidirectional relay with capture

        // Client → Server (requests)
        Task {
            while let data = try? await client.read() {
                // Emit as SslWrite event
                eventEmitter.emit(RawCaptureEvent(
                    kind: .sslWrite,
                    pid: process.pid,
                    data: data,
                    metadata: process.metadata
                ))

                // Forward to server
                try? await server.write(data)
            }
        }

        // Server → Client (responses)
        Task {
            while let data = try? await server.read() {
                // Emit as SslRead event
                eventEmitter.emit(RawCaptureEvent(
                    kind: .sslRead,
                    pid: process.pid,
                    data: data,
                    metadata: process.metadata
                ))

                // Forward to client
                try? await client.write(data)
            }
        }
    }
}
```

### 4.2 TLS Interceptor Component

**File:** `OISPNetworkExtension/TLSInterceptor.swift`

```swift
import Foundation
import Network
import Security

class TLSInterceptor {

    private let certificateAuthority: CertificateAuthority
    private var generatedCerts: [String: SecIdentity] = [:]  // Cache

    init() {
        self.certificateAuthority = CertificateAuthority.shared
    }

    // MARK: - Client-side TLS (we act as server)

    func acceptClient(flow: NEAppProxyTCPFlow, serverName: String) async throws -> TLSSession {
        // 1. Generate certificate for this hostname (signed by our CA)
        let identity = try getOrCreateCertificate(for: serverName)

        // 2. Create TLS parameters
        let tlsOptions = NWProtocolTLS.Options()
        sec_protocol_options_set_local_identity(
            tlsOptions.securityProtocolOptions,
            sec_identity_create(identity)!
        )

        // 3. Perform TLS handshake
        // ... NWConnection setup with flow's file descriptors

        return TLSSession(/* ... */)
    }

    // MARK: - Server-side TLS (we act as client)

    func connectToServer(host: String, port: UInt16) async throws -> TLSSession {
        let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!)

        let tlsOptions = NWProtocolTLS.Options()
        // Use system trust store for server validation

        let parameters = NWParameters(tls: tlsOptions)
        let connection = NWConnection(to: endpoint, using: parameters)

        return try await withCheckedThrowingContinuation { continuation in
            connection.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    continuation.resume(returning: TLSSession(connection: connection))
                case .failed(let error):
                    continuation.resume(throwing: error)
                default:
                    break
                }
            }
            connection.start(queue: .global())
        }
    }

    // MARK: - Certificate Generation

    private func getOrCreateCertificate(for hostname: String) throws -> SecIdentity {
        if let cached = generatedCerts[hostname] {
            return cached
        }

        let identity = try certificateAuthority.generateCertificate(
            commonName: hostname,
            subjectAltNames: [hostname]
        )

        generatedCerts[hostname] = identity
        return identity
    }
}

// TLS Session wrapper
class TLSSession {
    private let connection: NWConnection

    init(connection: NWConnection) {
        self.connection = connection
    }

    func read() async throws -> Data {
        return try await withCheckedThrowingContinuation { continuation in
            connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, _, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: NSError(domain: "TLSSession", code: -1))
                }
            }
        }
    }

    func write(_ data: Data) async throws {
        return try await withCheckedThrowingContinuation { continuation in
            connection.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }
}
```

### 4.3 Certificate Authority Component

**File:** `OISPCore/Sources/Networking/CertificateAuthority.swift`

```swift
import Foundation
import Security

class CertificateAuthority {
    static let shared = CertificateAuthority()

    private var caPrivateKey: SecKey?
    private var caCertificate: SecCertificate?

    private let keychainTag = "com.oisp.ca.privatekey"
    private let certLabel = "OISP Local CA"

    // MARK: - Initialization

    func initialize() throws {
        // Check if CA already exists in keychain
        if let existingKey = try? loadCAFromKeychain() {
            caPrivateKey = existingKey.privateKey
            caCertificate = existingKey.certificate
            return
        }

        // Generate new CA
        try generateNewCA()
    }

    // MARK: - CA Generation

    private func generateNewCA() throws {
        // 1. Generate RSA key pair
        let keyParams: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeRSA,
            kSecAttrKeySizeInBits as String: 4096,
            kSecAttrIsPermanent as String: true,
            kSecAttrApplicationTag as String: keychainTag.data(using: .utf8)!
        ]

        var error: Unmanaged<CFError>?
        guard let privateKey = SecKeyCreateRandomKey(keyParams as CFDictionary, &error) else {
            throw error!.takeRetainedValue()
        }

        // 2. Generate self-signed CA certificate
        let certificate = try generateCACertificate(privateKey: privateKey)

        // 3. Store in keychain
        try storeCertificate(certificate)

        caPrivateKey = privateKey
        caCertificate = certificate
    }

    private func generateCACertificate(privateKey: SecKey) throws -> SecCertificate {
        // Using Security framework to create X.509 certificate
        // Subject: CN=OISP Local CA, O=OISP
        // Validity: 10 years
        // Extensions: CA:TRUE, keyUsage: keyCertSign, cRLSign

        // This requires either:
        // 1. Using openssl via Process()
        // 2. Using a Swift X.509 library
        // 3. Building ASN.1 manually

        // For production, recommend using swift-certificates package
        // https://github.com/apple/swift-certificates

        fatalError("Implement with swift-certificates")
    }

    // MARK: - Certificate Generation

    func generateCertificate(commonName: String, subjectAltNames: [String]) throws -> SecIdentity {
        guard let caKey = caPrivateKey, let caCert = caCertificate else {
            throw NSError(domain: "CertificateAuthority", code: 1, userInfo: [NSLocalizedDescriptionKey: "CA not initialized"])
        }

        // 1. Generate key pair for this certificate
        let keyParams: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeRSA,
            kSecAttrKeySizeInBits as String: 2048
        ]

        var error: Unmanaged<CFError>?
        guard let privateKey = SecKeyCreateRandomKey(keyParams as CFDictionary, &error) else {
            throw error!.takeRetainedValue()
        }

        // 2. Create certificate signed by CA
        // Subject: CN={commonName}
        // SAN: DNS:{subjectAltNames}
        // Validity: 1 year
        // Issuer: Our CA

        let certificate = try createSignedCertificate(
            privateKey: privateKey,
            commonName: commonName,
            subjectAltNames: subjectAltNames,
            signingKey: caKey,
            issuerCert: caCert
        )

        // 3. Create identity (cert + private key)
        return try createIdentity(certificate: certificate, privateKey: privateKey)
    }

    // MARK: - Trust Management

    func installCATrust() throws {
        guard let cert = caCertificate else {
            throw NSError(domain: "CertificateAuthority", code: 2, userInfo: [NSLocalizedDescriptionKey: "CA not initialized"])
        }

        // Add to system keychain with trust settings
        // This will prompt user for password
        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassCertificate,
            kSecValueRef as String: cert,
            kSecAttrLabel as String: certLabel
        ]

        var status = SecItemAdd(addQuery as CFDictionary, nil)
        if status == errSecDuplicateItem {
            // Already exists, update trust
            status = noErr
        }

        if status != noErr {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
        }

        // Set trust settings (requires admin)
        // SecTrustSettingsSetTrustSettings(cert, .admin, trustSettings)
    }

    var isCATrusted: Bool {
        guard let cert = caCertificate else { return false }

        var trustSettings: CFArray?
        let status = SecTrustSettingsCopyTrustSettings(cert, .user, &trustSettings)

        return status == noErr
    }
}
```

### 4.4 Menu Bar App Component

**File:** `OISPApp/MenuBarView.swift`

```swift
import SwiftUI

struct MenuBarView: View {
    @StateObject private var viewModel = MenuBarViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Status Header
            HStack {
                Circle()
                    .fill(viewModel.isCapturing ? Color.green : Color.gray)
                    .frame(width: 10, height: 10)

                Text(viewModel.isCapturing ? "Capturing" : "Paused")
                    .font(.headline)

                Spacer()

                Text("\(viewModel.requestCount) requests")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }

            // Stats
            if viewModel.isCapturing {
                HStack(spacing: 20) {
                    StatView(label: "Tokens", value: formatNumber(viewModel.totalTokens))
                    StatView(label: "Cost", value: formatCost(viewModel.totalCost))
                    StatView(label: "Avg Latency", value: "\(viewModel.avgLatencyMs)ms")
                }
                .font(.caption)
            }

            Divider()

            // Recent Requests
            Text("Recent Requests")
                .font(.subheadline)
                .foregroundColor(.secondary)

            ForEach(viewModel.recentRequests.prefix(5)) { request in
                RecentRequestRow(request: request)
            }

            if viewModel.recentRequests.isEmpty {
                Text("No requests captured yet")
                    .foregroundColor(.secondary)
                    .font(.caption)
            }

            Divider()

            // Actions
            HStack {
                Button(action: viewModel.toggleCapture) {
                    Label(
                        viewModel.isCapturing ? "Pause" : "Resume",
                        systemImage: viewModel.isCapturing ? "pause.fill" : "play.fill"
                    )
                }

                Spacer()

                Button(action: viewModel.openDashboard) {
                    Label("Dashboard", systemImage: "chart.bar")
                }
            }

            // Extension Status
            if !viewModel.isExtensionEnabled {
                Divider()

                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("Extension not enabled")
                        .font(.caption)
                    Spacer()
                    Button("Enable") {
                        viewModel.enableExtension()
                    }
                    .font(.caption)
                }
            }

            // CA Trust Status
            if !viewModel.isCATrusted {
                HStack {
                    Image(systemName: "lock.open.fill")
                        .foregroundColor(.orange)
                    Text("CA certificate not trusted")
                        .font(.caption)
                    Spacer()
                    Button("Trust") {
                        viewModel.trustCA()
                    }
                    .font(.caption)
                }
            }

            Divider()

            // Footer
            HStack {
                Button(action: viewModel.openSettings) {
                    Label("Settings", systemImage: "gear")
                }

                Spacer()

                Button(action: { NSApp.terminate(nil) }) {
                    Label("Quit", systemImage: "power")
                }
            }
        }
        .padding()
        .frame(width: 320)
    }
}

struct RecentRequestRow: View {
    let request: RecentRequest

    var body: some View {
        HStack {
            Image(systemName: request.provider.icon)
                .foregroundColor(request.provider.color)

            VStack(alignment: .leading, spacing: 2) {
                Text(request.model)
                    .font(.caption)
                    .fontWeight(.medium)

                Text(request.preview)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text("\(request.latencyMs)ms")
                    .font(.caption2)

                Text("\(request.tokens) tok")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
    }
}

// ViewModel
class MenuBarViewModel: ObservableObject {
    @Published var isCapturing = false
    @Published var requestCount = 0
    @Published var totalTokens = 0
    @Published var totalCost: Double = 0.0
    @Published var avgLatencyMs = 0
    @Published var recentRequests: [RecentRequest] = []
    @Published var isExtensionEnabled = false
    @Published var isCATrusted = false

    private let extensionManager = ExtensionManager()
    private let sensorBridge = SensorBridge()

    init() {
        checkStatus()
        subscribeToEvents()
    }

    func checkStatus() {
        isExtensionEnabled = extensionManager.isEnabled
        isCATrusted = CertificateAuthority.shared.isCATrusted
    }

    func toggleCapture() {
        if isCapturing {
            sensorBridge.pause()
        } else {
            sensorBridge.resume()
        }
        isCapturing.toggle()
    }

    func enableExtension() {
        extensionManager.requestActivation { [weak self] success in
            self?.isExtensionEnabled = success
        }
    }

    func trustCA() {
        do {
            try CertificateAuthority.shared.installCATrust()
            isCATrusted = true
        } catch {
            // Show error
        }
    }

    func openDashboard() {
        // Open web dashboard or native window
        if let url = URL(string: "http://localhost:3000") {
            NSWorkspace.shared.open(url)
        }
    }

    func openSettings() {
        // Open settings window
    }

    private func subscribeToEvents() {
        sensorBridge.onEvent = { [weak self] event in
            DispatchQueue.main.async {
                self?.handleEvent(event)
            }
        }
    }

    private func handleEvent(_ event: OISPEvent) {
        requestCount += 1

        if let tokens = event.usage?.totalTokens {
            totalTokens += Int(tokens)
        }

        if let cost = event.usage?.totalCostUsd {
            totalCost += cost
        }

        let recent = RecentRequest(
            id: event.id,
            provider: event.provider,
            model: event.model ?? "Unknown",
            preview: event.preview ?? "",
            latencyMs: event.latencyMs ?? 0,
            tokens: Int(event.usage?.totalTokens ?? 0)
        )

        recentRequests.insert(recent, at: 0)
        if recentRequests.count > 20 {
            recentRequests.removeLast()
        }
    }
}
```

---

## 5. Data Flow

### 5.1 Complete Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DATA FLOW                                       │
│                                                                              │
│  1. Application makes HTTPS request                                          │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ import openai                                                        │ │
│     │ client = openai.OpenAI()                                            │ │
│     │ response = client.chat.completions.create(                          │ │
│     │     model="gpt-4",                                                  │ │
│     │     messages=[{"role": "user", "content": "Hello"}]                 │ │
│     │ )                                                                   │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  2. Network Extension intercepts TCP to api.openai.com:443                  │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ handleNewFlow(_ flow: NEAppProxyTCPFlow)                            │ │
│     │   → host = "api.openai.com"                                         │ │
│     │   → AIEndpointFilter.shouldIntercept("api.openai.com") == true      │ │
│     │   → Start MITM session                                              │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  3. TLS MITM captures plaintext HTTP                                        │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ Plaintext HTTP Request:                                              │ │
│     │ POST /v1/chat/completions HTTP/1.1                                  │ │
│     │ Host: api.openai.com                                                │ │
│     │ Authorization: Bearer sk-...                                        │ │
│     │ Content-Type: application/json                                      │ │
│     │                                                                     │ │
│     │ {"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}    │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  4. EventEmitter sends RawCaptureEvent to oisp-sensor                       │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ {                                                                   │ │
│     │   "id": "01HQX...",                                                 │ │
│     │   "timestamp_ns": 1703123456789000000,                              │ │
│     │   "kind": "SslWrite",                                               │ │
│     │   "pid": 12345,                                                     │ │
│     │   "tid": 12346,                                                     │ │
│     │   "data": "UE9TVC...(base64)",                                      │ │
│     │   "metadata": {                                                     │ │
│     │     "comm": "python3",                                              │ │
│     │     "exe": "/usr/bin/python3",                                      │ │
│     │     "uid": 501                                                      │ │
│     │   }                                                                 │ │
│     │ }                                                                   │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  5. oisp-sensor HttpDecoder parses and emits AiRequestEvent                 │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ // In Rust (oisp-decode/src/decoder.rs)                             │ │
│     │ fn decode_ssl_write(&self, raw: &RawCaptureEvent)                   │ │
│     │   → parse_request(&raw.data)  // HTTP parsing                       │ │
│     │   → detect_provider("api.openai.com")  // OpenAI                    │ │
│     │   → parse_ai_request(&json)  // Extract model, messages, etc        │ │
│     │   → Emit AiRequestEvent                                             │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  6. AiRequestEvent output (identical on Linux and macOS)                    │
│     ┌─────────────────────────────────────────────────────────────────────┐ │
│     │ {                                                                   │ │
│     │   "event_type": "ai.request",                                       │ │
│     │   "ts": "2024-01-15T10:30:00Z",                                     │ │
│     │   "request_id": "req_abc123",                                       │ │
│     │   "provider": {"name": "openai", "endpoint": "api.openai.com"},     │ │
│     │   "model": {"id": "gpt-4", "family": "gpt"},                        │ │
│     │   "messages": [...],                                                │ │
│     │   "streaming": false,                                               │ │
│     │   "process": {"pid": 12345, "name": "python3", "exe": "..."}        │ │
│     │ }                                                                   │ │
│     └─────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Response Flow (Similar)

```
Server Response → TLS Decrypt → RawCaptureEvent(SslRead) → HttpDecoder → AiResponseEvent
```

---

## 6. Installation & Distribution

### 6.1 Distribution Options

| Method | Pros | Cons | Recommended For |
|--------|------|------|-----------------|
| **DMG Installer** | Simple, visual, universal, no dependencies | Manual updates | Everyone (PRIMARY) |
| **Homebrew Cask** | Easy updates, familiar to devs | Extra step, requires Homebrew | Developers (optional) |
| **PKG Installer** | Can run scripts, MDM-friendly | More complex to build | Enterprise (future) |

**Why NOT App Store:**
- System Extensions are explicitly **prohibited** on App Store
- Network Extensions with MITM capabilities would never be approved
- We need entitlements Apple doesn't grant to App Store apps
- No review delays - ship when ready

### 6.2 Primary Distribution: DMG

**DMG Download** (for everyone - primary method)
- Download `OISP-x.x.x.dmg` from GitHub releases or website
- Open DMG, drag OISP.app to Applications
- First launch triggers extension approval + CA trust
- Works for developers AND non-technical users
- Notarized by Apple for Gatekeeper approval

**Optional: Homebrew Cask** (convenience for developers)
```bash
brew tap oisp/tap
brew install --cask oisp
```
The Homebrew cask just automates downloading and installing the same DMG.

### 6.3 DMG Creation

**File:** `Scripts/create-dmg.sh`

```bash
#!/bin/bash
set -e

VERSION="${1:-0.1.0}"
APP_NAME="OISP"
DMG_NAME="OISP-${VERSION}.dmg"

# Build release
xcodebuild -project OISPApp/OISPApp.xcodeproj \
           -scheme "OISP" \
           -configuration Release \
           -archivePath build/OISP.xcarchive \
           archive

# Export app
xcodebuild -exportArchive \
           -archivePath build/OISP.xcarchive \
           -exportOptionsPlist Distribution/export-options.plist \
           -exportPath build/

# Create DMG
create-dmg \
    --volname "OISP" \
    --volicon "Distribution/icon.icns" \
    --background "Distribution/dmg-background.png" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --icon "OISP.app" 150 190 \
    --hide-extension "OISP.app" \
    --app-drop-link 450 190 \
    "build/${DMG_NAME}" \
    "build/OISP.app"

# Notarize
xcrun notarytool submit "build/${DMG_NAME}" \
    --keychain-profile "AC_PASSWORD" \
    --wait

# Staple
xcrun stapler staple "build/${DMG_NAME}"

echo "Created: build/${DMG_NAME}"
```

### 6.4 Homebrew Formula

**File:** `Distribution/homebrew-formula.rb`

```ruby
cask "oisp" do
  version "0.1.0"
  sha256 "abc123..."  # Update with actual hash

  url "https://github.com/oisp/oisp-macos/releases/download/v#{version}/OISP-#{version}.dmg"
  name "OISP"
  desc "AI/LLM API Observability for macOS"
  homepage "https://github.com/oisp/oisp"

  depends_on macos: ">= :ventura"  # macOS 13+

  app "OISP.app"

  postflight do
    # Remind user about extension approval
    ohai "After first launch, approve the system extension in:"
    ohai "System Settings → Privacy & Security → Security"
  end

  zap trash: [
    "~/Library/Application Support/OISP",
    "~/Library/Preferences/com.oisp.app.plist",
    "~/Library/Caches/com.oisp.app",
  ]

  caveats <<~EOS
    OISP requires a system extension to capture AI API traffic.

    After installation:
    1. Launch OISP from Applications
    2. Click "Enable Extension" when prompted
    3. Approve in System Settings → Privacy & Security
    4. Trust the OISP CA certificate when prompted

    For CLI usage, install the sensor separately:
      brew install oisp/tap/oisp-sensor
  EOS
end
```

### 6.5 First-Launch Experience

```
┌─────────────────────────────────────────────────────────────────┐
│                     OISP Setup                                   │
│                                                                  │
│  Welcome to OISP! 👋                                             │
│                                                                  │
│  To capture AI API traffic, we need to:                          │
│                                                                  │
│  1. Enable the Network Extension                                 │
│     ┌─────────────────────────────────────────────────────────┐ │
│     │ ⚠️ System Extension Blocked                              │ │
│     │                                                          │ │
│     │ "OISP" tried to load new system extensions.              │ │
│     │                                                          │ │
│     │ [Open System Settings]                                   │ │
│     └─────────────────────────────────────────────────────────┘ │
│                                                                  │
│  2. Trust the OISP CA Certificate                                │
│     This allows OISP to inspect HTTPS traffic to AI APIs.        │
│     The certificate is LOCAL ONLY and never leaves your Mac.     │
│                                                                  │
│     [Trust Certificate]  (requires password)                     │
│                                                                  │
│  ─────────────────────────────────────────────────────────────── │
│                                                                  │
│  [Skip for Now]                              [Complete Setup]    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 6.6 System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| macOS Version | 13.0 (Ventura) | 14.0+ (Sonoma) |
| Architecture | Apple Silicon or Intel | Apple Silicon |
| RAM | 4 GB | 8 GB+ |
| Disk Space | 100 MB | 500 MB (with logs) |
| Network | Any | Any |

**Why macOS 13+?**
- `NETransparentProxyProvider` improvements
- Better System Extension UX
- SwiftUI 4 features
- Network framework async/await support

---

## 7. Development Phases

### 7.1 Phase Overview

```
Phase 1: Foundation (Weeks 1-2)
├── Project setup & build system
├── CA certificate generation
└── Basic menu bar app shell

Phase 2: Network Extension (Weeks 3-5)
├── TransparentProxyProvider skeleton
├── Flow interception
├── AI endpoint filtering
└── Process attribution

Phase 3: TLS MITM (Weeks 6-8)
├── TLS client-side (act as server)
├── TLS server-side (act as client)
├── Certificate generation per-host
└── Bidirectional relay

Phase 4: Integration (Weeks 9-10)
├── Event bridge to Rust sensor
├── Full decode pipeline testing
├── Error handling & recovery
└── Performance optimization

Phase 5: Polish (Weeks 11-12)
├── Menu bar UI completion
├── Settings & configuration
├── Installer & distribution
└── Documentation

Phase 6: Testing & Release (Week 13+)
├── Beta testing
├── Bug fixes
├── App Store Connect (if applicable)
└── Public release
```

### 7.2 Detailed Phase Breakdown

#### Phase 1: Foundation (Weeks 1-2)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| Create Xcode project structure | - | 2h | None |
| Configure signing & entitlements | - | 4h | Apple Developer account |
| Implement CertificateAuthority | - | 16h | Security framework knowledge |
| CA certificate tests | - | 8h | CertificateAuthority |
| Menu bar app shell | - | 8h | SwiftUI |
| CI/CD setup (GitHub Actions) | - | 8h | Xcode project |

**Deliverables:**
- [ ] Xcode project that builds
- [ ] CA certificate generation working
- [ ] Basic menu bar app appears in system tray

#### Phase 2: Network Extension (Weeks 3-5)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| Network Extension target setup | - | 4h | Phase 1 |
| TransparentProxyProvider lifecycle | - | 8h | NetworkExtension framework |
| Flow interception (handleNewFlow) | - | 16h | Provider lifecycle |
| AI endpoint filter | - | 4h | Endpoint list |
| Process attribution (libproc) | - | 16h | Flow interception |
| Extension enable/disable UI | - | 8h | Menu bar app |
| Integration tests | - | 16h | All above |

**Deliverables:**
- [ ] Extension loads and activates
- [ ] Intercepts TCP connections to AI endpoints
- [ ] Can identify source process for each connection
- [ ] Non-AI traffic passes through unmodified

#### Phase 3: TLS MITM (Weeks 6-8)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| TLS server implementation (for clients) | - | 24h | CertificateAuthority |
| TLS client implementation (to servers) | - | 16h | Network framework |
| Per-host certificate generation | - | 8h | TLS server |
| Certificate caching | - | 4h | Per-host certs |
| Bidirectional relay | - | 16h | Both TLS sides |
| Plaintext capture | - | 8h | Relay |
| Error handling & connection cleanup | - | 16h | All above |

**Deliverables:**
- [ ] Can MITM a TLS connection to api.openai.com
- [ ] Plaintext HTTP visible in logs
- [ ] Connections work end-to-end (app gets real response)
- [ ] Proper cleanup on connection close

#### Phase 4: Integration (Weeks 9-10)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| RawCaptureEvent Swift model | - | 4h | Event spec |
| Unix socket bridge (Swift side) | - | 8h | None |
| Unix socket bridge (Rust side) | - | 8h | oisp-capture-macos |
| End-to-end capture test | - | 16h | Both bridges |
| HttpDecoder testing with macOS events | - | 8h | Capture working |
| Performance profiling | - | 16h | E2E working |
| Memory leak detection | - | 8h | E2E working |

**Deliverables:**
- [ ] Events flow from extension → Rust sensor
- [ ] AiRequestEvent/AiResponseEvent generated correctly
- [ ] Performance acceptable (<5ms added latency)
- [ ] No memory leaks under load

#### Phase 5: Polish (Weeks 11-12)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| Menu bar UI - full implementation | - | 16h | Phase 4 |
| Settings UI | - | 8h | Configuration model |
| CA trust UI flow | - | 8h | CertificateAuthority |
| Dashboard window (optional) | - | 16h | Event stream |
| DMG creation script | - | 8h | Built app |
| Homebrew formula | - | 4h | DMG working |
| Notarization workflow | - | 8h | Apple Developer |
| User documentation | - | 16h | All features |

**Deliverables:**
- [ ] Complete, polished menu bar app
- [ ] Working DMG installer
- [ ] Homebrew cask formula
- [ ] User guide & troubleshooting docs

#### Phase 6: Testing & Release (Week 13+)

| Task | Owner | Estimate | Dependencies |
|------|-------|----------|--------------|
| Internal beta testing | - | 40h | Phase 5 |
| Bug fixes from beta | - | Variable | Beta feedback |
| External beta (TestFlight if possible) | - | 40h | Internal beta |
| Final bug fixes | - | Variable | External beta |
| Release preparation | - | 8h | All fixes |
| Public announcement | - | 4h | Release ready |

**Deliverables:**
- [ ] Stable release
- [ ] GitHub release with DMG
- [ ] Homebrew tap updated
- [ ] Documentation published

---

## 8. Parallel Workstreams

### 8.1 Workstream Diagram

```
                           Week 1    Week 2    Week 3    Week 4    Week 5    Week 6    Week 7    Week 8
                           ├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤

Workstream A: Foundation   ████████████████████
(CA, Project Setup)              │
                                 │
Workstream B: Extension          │    ████████████████████████████████████
(Network Extension)              │         │         │
                                 │         │         │
Workstream C: TLS                │         │         │    ████████████████████████████████████
(MITM Engine)                    │         │         │         │
                                 │         │         │         │
Workstream D: UI                 │         │    ██████████████████████████████████████████████████
(Menu Bar App)                   │         │              │
                                 │         │              │
Workstream E: Bridge             │         │              │    ████████████████████████
(Swift↔Rust)                     │         │              │         │
                                 │         │              │         │
Workstream F: Docs               │         │              │         │    ████████████████████████
(Documentation)                  │         │              │         │
                                 ▼         ▼              ▼         ▼

                              CA Done   Extension    TLS Done   Integration
                                       Intercepts              Complete
```

### 8.2 Workstream Details

#### Workstream A: Foundation (Can start immediately)

**Owner:** 1 engineer
**Duration:** Weeks 1-2
**Blocking:** B, C

Tasks:
1. Xcode project creation
2. Entitlements & signing configuration
3. CertificateAuthority implementation
4. CA tests
5. CI/CD setup

**Output:** Working CA certificate generation, buildable project

---

#### Workstream B: Network Extension (Starts after A delivers CA)

**Owner:** 1-2 engineers
**Duration:** Weeks 2-5
**Blocked by:** A (needs project structure)
**Blocking:** E (bridge needs extension)

Tasks:
1. Network Extension target
2. TransparentProxyProvider implementation
3. Flow interception
4. AI endpoint filtering
5. Process attribution

**Output:** Extension that intercepts AI API connections

---

#### Workstream C: TLS MITM (Starts after A delivers CA)

**Owner:** 1-2 engineers
**Duration:** Weeks 3-8
**Blocked by:** A (needs CertificateAuthority)
**Blocking:** None (can be developed in isolation)

Tasks:
1. TLS server (client-facing)
2. TLS client (server-facing)
3. Per-host certificate generation
4. Bidirectional relay
5. Plaintext capture

**Output:** Standalone TLS MITM library/module

---

#### Workstream D: Menu Bar UI (Can start immediately)

**Owner:** 1 engineer
**Duration:** Weeks 2-10
**Blocked by:** None (can use mock data)
**Blocking:** None

Tasks:
1. Menu bar app shell
2. Status display
3. Recent requests list
4. Settings UI
5. CA trust flow
6. Extension enable flow

**Output:** Complete menu bar app (ready to connect to real data)

---

#### Workstream E: Swift↔Rust Bridge (Starts Week 6)

**Owner:** 1 engineer
**Duration:** Weeks 6-9
**Blocked by:** B (needs extension producing events), C (needs plaintext)
**Blocking:** Final integration

Tasks:
1. RawCaptureEvent Swift model
2. Unix socket server (Rust)
3. Unix socket client (Swift)
4. Serialization/deserialization
5. Error handling

**Output:** Events flow from Swift to Rust

---

#### Workstream F: Documentation (Ongoing)

**Owner:** All engineers (part-time)
**Duration:** Weeks 4-12
**Blocked by:** Features being documented
**Blocking:** Release

Tasks:
1. Architecture docs
2. Installation guide
3. Troubleshooting guide
4. Developer guide
5. API reference

**Output:** Complete documentation

---

### 8.3 Integration Points

| Integration | From | To | When | How to Test |
|-------------|------|-----|------|-------------|
| CA → Extension | A | B | Week 3 | Extension uses CA for certs |
| CA → TLS | A | C | Week 3 | TLS uses CA for signing |
| Extension → TLS | B | C | Week 6 | Extension routes connections through TLS |
| TLS → Bridge | C | E | Week 7 | Plaintext events sent over bridge |
| Extension → Bridge | B | E | Week 7 | Process info sent over bridge |
| Bridge → Decoder | E | (Rust) | Week 8 | Full event decode works |
| UI → Extension | D | B | Week 8 | UI controls extension state |
| UI → Bridge | D | E | Week 9 | UI shows live events |

---

## 9. Testing Strategy

### 9.1 Test Categories

| Category | Scope | Tools | Owner |
|----------|-------|-------|-------|
| Unit Tests | Individual functions | XCTest, Rust tests | Each workstream |
| Integration Tests | Component interactions | XCTest, pytest | Integration team |
| End-to-End Tests | Full capture flow | pytest, curl | QA |
| Performance Tests | Latency, throughput | Instruments, custom | Performance team |
| Security Tests | CA, TLS, permissions | Manual + automated | Security review |

### 9.2 Test Scenarios

#### Unit Tests

```swift
// CA Certificate Tests
func testCAGeneration() {
    let ca = CertificateAuthority()
    XCTAssertNoThrow(try ca.initialize())
    XCTAssertNotNil(ca.caCertificate)
}

func testCertificateGeneration() {
    let ca = CertificateAuthority()
    try! ca.initialize()

    let identity = try! ca.generateCertificate(
        commonName: "api.openai.com",
        subjectAltNames: ["api.openai.com"]
    )

    XCTAssertNotNil(identity)
}

// Endpoint Filter Tests
func testEndpointFiltering() {
    XCTAssertTrue(AIEndpointFilter.shouldIntercept(host: "api.openai.com"))
    XCTAssertTrue(AIEndpointFilter.shouldIntercept(host: "api.anthropic.com"))
    XCTAssertFalse(AIEndpointFilter.shouldIntercept(host: "google.com"))
    XCTAssertFalse(AIEndpointFilter.shouldIntercept(host: "api.stripe.com"))
}

// Process Attribution Tests
func testProcessInfo() {
    let info = ProcessInfo.current(pid: getpid())
    XCTAssertEqual(info.pid, getpid())
    XCTAssertNotNil(info.exe)
    XCTAssertNotNil(info.comm)
}
```

#### Integration Tests

```python
# test_capture_openai.py
import openai
import subprocess
import json
import time

def test_openai_request_captured():
    """Test that OpenAI requests are captured by OISP"""

    # Start oisp-sensor in test mode
    sensor = subprocess.Popen(
        ["oisp-sensor", "record", "--output", "/tmp/test-events.jsonl"],
        stdout=subprocess.PIPE
    )
    time.sleep(2)  # Wait for sensor to start

    # Make OpenAI request
    client = openai.OpenAI()
    response = client.chat.completions.create(
        model="gpt-4",
        messages=[{"role": "user", "content": "Say hello"}]
    )

    time.sleep(2)  # Wait for capture
    sensor.terminate()

    # Verify capture
    with open("/tmp/test-events.jsonl") as f:
        events = [json.loads(line) for line in f]

    # Should have request and response
    request_events = [e for e in events if e["event_type"] == "ai.request"]
    response_events = [e for e in events if e["event_type"] == "ai.response"]

    assert len(request_events) >= 1
    assert len(response_events) >= 1

    # Verify request content
    req = request_events[0]
    assert req["model"]["id"] == "gpt-4"
    assert req["provider"]["name"] == "openai"
    assert len(req["messages"]) == 1

    # Verify response content
    resp = response_events[0]
    assert resp["request_id"] == req["request_id"]
    assert resp["status_code"] == 200
    assert resp["latency_ms"] > 0
```

#### End-to-End Tests

```bash
#!/bin/bash
# test_e2e.sh

# Test 1: OpenAI Python
echo "Test 1: OpenAI Python SDK"
python3 -c "
import openai
client = openai.OpenAI()
r = client.chat.completions.create(model='gpt-4o-mini', messages=[{'role':'user','content':'hi'}])
print(f'Response: {r.choices[0].message.content}')
"

# Test 2: Anthropic Python
echo "Test 2: Anthropic Python SDK"
python3 -c "
import anthropic
client = anthropic.Anthropic()
r = client.messages.create(model='claude-3-haiku-20240307', max_tokens=100, messages=[{'role':'user','content':'hi'}])
print(f'Response: {r.content[0].text}')
"

# Test 3: curl to OpenAI
echo "Test 3: curl to OpenAI"
curl -s https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"hi"}]}'

# Test 4: Node.js
echo "Test 4: Node.js OpenAI"
node -e "
const OpenAI = require('openai');
const client = new OpenAI();
client.chat.completions.create({model:'gpt-4o-mini',messages:[{role:'user',content:'hi'}]})
  .then(r => console.log(r.choices[0].message.content));
"

# Verify all captured
echo "Checking captured events..."
EVENTS=$(cat /tmp/oisp-test-events.jsonl | wc -l)
echo "Captured $EVENTS events"
```

### 9.3 Performance Benchmarks

| Metric | Target | Method |
|--------|--------|--------|
| Added latency | <5ms | Time request with/without OISP |
| Memory usage | <50MB | Instruments memory profiling |
| CPU usage (idle) | <1% | Activity Monitor |
| CPU usage (active) | <5% | Instruments CPU profiling |
| Throughput | >100 req/s | Load test with concurrent requests |

---

## 10. Security Considerations

### 10.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| CA private key exposure | Stored in Keychain with ACL, not exportable |
| Rogue certificate issuance | CA only signs for AI endpoints, short validity |
| Traffic interception by malware | Extension signed & notarized, SIP protected |
| Plaintext data in memory | Events streamed, not buffered long-term |
| User data exfiltration | No network upload, local-only by default |

### 10.2 Privacy Controls

1. **Local-only processing**: All data stays on device by default
2. **Redaction**: Sensitive content can be redacted before logging
3. **Selective capture**: Only AI API endpoints, user can customize
4. **Data retention**: Configurable, default 7 days
5. **Export control**: User must explicitly enable any cloud export

### 10.3 Security Checklist

- [ ] CA private key uses Keychain with `kSecAttrAccessibleAfterFirstUnlock`
- [ ] CA private key marked as non-exportable
- [ ] Generated certificates have short validity (24h)
- [ ] Extension signed with hardened runtime
- [ ] No `com.apple.security.cs.disable-library-validation`
- [ ] App sandboxed where possible
- [ ] No plaintext secrets in logs
- [ ] API keys redacted by default
- [ ] Audit log for configuration changes

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **eBPF** | Extended Berkeley Packet Filter - Linux kernel technology for programmable packet/event processing |
| **NETransparentProxyProvider** | Apple API for transparent network proxy system extensions |
| **MITM** | Man-in-the-Middle - intercepting communication between two parties |
| **CA** | Certificate Authority - issues TLS certificates |
| **SNI** | Server Name Indication - TLS extension that indicates target hostname |
| **XPC** | Cross-Process Communication - Apple IPC mechanism |
| **System Extension** | Apple's replacement for kernel extensions (.kext) |
| **Notarization** | Apple's malware scanning service for non-App Store apps |
| **SIP** | System Integrity Protection - macOS security feature |

---

## Appendix B: Reference Links

- [Apple Network Extension Framework](https://developer.apple.com/documentation/networkextension)
- [NETransparentProxyProvider](https://developer.apple.com/documentation/networkextension/netransparentproxyprovider)
- [System Extensions](https://developer.apple.com/documentation/systemextensions)
- [Security Framework](https://developer.apple.com/documentation/security)
- [swift-certificates](https://github.com/apple/swift-certificates)
- [OISP Event Specification](../oisp-spec/)

---

## Appendix C: FAQ

**Q: Why not just use a regular HTTP proxy like mitmproxy?**
A: A regular proxy requires apps to be proxy-aware or requires manual system proxy configuration. Many CLI tools and some apps ignore system proxy settings. The Network Extension approach captures ALL traffic transparently.

**Q: Will this break certificate pinning?**
A: Yes, apps that pin certificates (e.g., some banking apps) will fail to connect. However, AI APIs (OpenAI, Anthropic, etc.) do not use certificate pinning, so this doesn't affect our use case.

**Q: Can I use this without the menu bar app?**
A: Yes, you can run just the Rust `oisp-sensor` CLI if the extension is already enabled. The menu bar app is for convenience.

**Q: Does this work on iOS?**
A: No. iOS requires a VPN profile for network extension, and the user experience is very different. This implementation is macOS-only.

**Q: What about Apple Silicon vs Intel?**
A: The app will be built as a Universal binary supporting both architectures.

---

*Document Version: 1.0*
*Last Updated: 2024-01-15*
*Authors: OISP Team*
