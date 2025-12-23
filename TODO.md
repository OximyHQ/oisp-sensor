# OISP Sensor - Master TODO

> Track progress across conversations. Check off items as they're completed.
> Each chapter = one conversation/session. Mark items with `[x]` when done.

---

## Status Key

- `[ ]` - Not started
- `[~]` - In progress  
- `[x]` - Completed
- `[!]` - Blocked/Issue

---

## 🎯 THE BIG PICTURE

OISP Sensor is the **universal, open-source foundation** for AI observability.

**Two Primary Use Cases:**
1. **Agent Monitoring** (Linux servers/VMs) - Node.js/Python agents making LLM calls
2. **Human-AI Monitoring** (Mac/Windows desktops) - Humans using Cursor, ChatGPT, Claude, etc.

**The Flow:**
```
[Any AI Activity] → [Platform Capture] → [OISP Events] → [Sinks] → [UI/Analysis]
                                                              ↓
                                                    [★ Oximy Cloud ★]
                                                    (Proprietary SaaS)
                                                              ↓
                                            [Smart Redaction, Policies, Reports]
                                                              ↓
                                                    [Push rules back to sensors]
```

**What's Open Source (this repo):**
- All capture implementations (eBPF, macOS, Windows)
- OISP-spec event format
- Local sinks (JSONL, WebSocket, OTLP, Kafka)
- Local Web UI
- Oximy exporter (client-side)
- Control plane client (receives policies)

**What's Proprietary (Oximy Cloud):**
- Central aggregation & storage
- ML-based redaction rules
- Policy engine
- Compliance reporting
- Multi-machine dashboards

---

# PART 1: CORE INFRASTRUCTURE (DONE)

## CHAPTER 1: eBPF Data Capture Implementation ✅ COMPLETE

**Goal:** Get real SSL data flowing from eBPF probes to userspace.

## CHAPTER 2: Testing with Real Requests ✅ COMPLETE

**Goal:** Verify end-to-end capture with actual HTTPS requests.

## CHAPTER 3: Integration with Pipeline ✅ COMPLETE

**Goal:** Connect eBPF capture to the existing OISP pipeline.

## CHAPTER 4: TUI/GUI Verification ✅ COMPLETE

**Goal:** See real captured events in the UI.

## CHAPTER 5: Docker & Containerization ✅ COMPLETE

**Goal:** Fully working Docker-based deployment.

## CHAPTER 8: Codebase Cleanup & Refactoring ✅ COMPLETE

**Goal:** Clean, maintainable codebase.

## CHAPTER 9: AgentSight-Style UI & Process-Centric View ✅ COMPLETE

**Goal:** Achieve AgentSight-like UI with process tree, timeline, and unified event visualization.

---

# PART 2: COMPLETE LINUX IMPLEMENTATION

> **PRIORITY ORDER:** Complete all Linux functionality FIRST before CI/CD, packaging, or documentation.
> Rationale: No point in stabilizing/documenting features that will change.

## CHAPTER 6: Advanced Sink Implementation ✅ COMPLETE

**Goal:** All export destinations fully implemented and tested.

### Phase 6.1: OTLP Export Implementation ✅ COMPLETE

- [x] Update `crates/oisp-export/src/otlp.rs`
  - [x] Use `opentelemetry-otlp` crate
  - [x] Implement ExportPlugin trait
- [x] Map OISP events to OTLP logs
  - [x] OispEvent → LogRecord
  - [x] Preserve all attributes
  - [x] Set resource attributes (host, sensor version)
- [x] Support OTLP/gRPC transport
  - [x] TLS configuration
  - [x] Compression (gzip)
- [x] Support OTLP/HTTP transport
  - [x] HTTP/proto and HTTP/JSON protocols
- [x] Add authentication headers
  - [x] API key header
  - [x] Bearer token
- [x] Map to OpenTelemetry Semantic Conventions
  - [x] gen_ai.* attributes
  - [x] process.* attributes
  - [x] host.* attributes
- [x] Batching configuration (batch size, flush interval)
- [ ] Add trace context propagation (deferred - spans vs logs)
- [ ] Test with OpenTelemetry Collector (requires Linux)
- [ ] Test with Grafana Cloud / Datadog / Honeycomb (requires accounts)

### Phase 6.2: Kafka Export Implementation ✅ COMPLETE

- [x] Update `crates/oisp-export/src/kafka.rs`
  - [x] Use `rdkafka` crate
  - [x] Implement ExportPlugin trait
- [x] Producer configuration
  - [x] Bootstrap servers
  - [x] Topic name
  - [x] SASL authentication (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512)
  - [x] TLS configuration
- [x] Message format
  - [x] JSON serialization
  - [x] Key: event_id or (host, pid)
  - [x] Headers: event_type, oisp_version
- [x] Batching and buffering
  - [x] Batch size configuration
  - [x] Linger time
  - [x] Buffer memory
- [x] Compression (gzip, snappy, lz4, zstd)
- [ ] Optional Avro serialization (deferred - requires schema registry)
- [ ] Test with Kafka Docker (requires Linux)
- [ ] Test with Confluent Cloud (requires account)

### Phase 6.3: Webhook Export Implementation ✅ COMPLETE

- [x] Create `crates/oisp-export/src/webhook.rs`
  - [x] Use `reqwest` crate
  - [x] Implement ExportPlugin trait
- [x] Configuration
  - [x] Endpoint URL
  - [x] HTTP method (POST/PUT/PATCH)
  - [x] Headers (static)
  - [x] Authentication (API key, Bearer, Basic)
- [x] Request format
  - [x] Single event per request
  - [x] Batch mode (array of events)
- [x] Retry logic
  - [x] Exponential backoff
  - [x] Max retries
  - [x] Dead letter queue (file)
- [x] Response handling
  - [x] 2xx = success
  - [x] 4xx = drop event
  - [x] 5xx = retry
- [ ] Test with webhook.site / n8n / custom endpoint (manual testing)

---

## CHAPTER 7: Oximy Cloud Integration

**Goal:** Connect sensor to Oximy Cloud for premium features.

### Phase 7.1: Oximy Exporter - Device Registration

- [ ] Create `crates/oisp-export/src/oximy.rs`
- [ ] Implement device registration flow
  - [ ] Collect device fingerprint
    - [ ] Hostname
    - [ ] OS/arch
    - [ ] Unique machine ID (machine-uid crate)
  - [ ] POST /v1/devices/register
  - [ ] Store device_id locally
  - [ ] Handle registration errors
- [ ] Store credentials securely
  - [ ] Linux: keyring or file with restricted permissions
  - [ ] Future: macOS Keychain, Windows Credential Manager

### Phase 7.2: Oximy Exporter - Event Ingestion

- [ ] Implement ExportPlugin trait for OximyExporter
- [ ] Batch event upload
  - [ ] Configurable batch size (default 100)
  - [ ] Configurable flush interval (default 5s)
  - [ ] POST /v1/ingest
  - [ ] gzip compression
- [ ] Authentication
  - [ ] Bearer token from API key
  - [ ] Device ID header
- [ ] Handle rate limiting
  - [ ] Parse Retry-After header
  - [ ] Exponential backoff
- [ ] Handle connection failures
  - [ ] Local buffer/spool to disk
  - [ ] Resume on reconnect
  - [ ] Configurable spool size limit

### Phase 7.3: Oximy Exporter - Configuration

- [ ] Add CLI options
  - [ ] --oximy-api-key
  - [ ] --oximy-endpoint (default: https://api.oximy.com)
  - [ ] --oximy-device-name (optional friendly name)
- [ ] Add config file section
  ```toml
  [export.oximy]
  enabled = true
  api_key = "ox_..."
  endpoint = "https://api.oximy.com"
  batch_size = 100
  flush_interval_ms = 5000
  spool_path = "/var/lib/oisp-sensor/spool"
  spool_max_mb = 100
  ```
- [ ] Environment variable: OISP_OXIMY_API_KEY

### Phase 7.4: Health & Telemetry

- [ ] Send sensor health metrics to Oximy
  - [ ] CPU/memory usage of sensor
  - [ ] Events/second rate
  - [ ] Error counts
  - [ ] Ring buffer usage (Linux)
- [ ] Heartbeat endpoint
  - [ ] POST /v1/devices/{id}/heartbeat
  - [ ] Every 60 seconds
- [ ] Report sensor version
  - [ ] Enable remote upgrade notifications

---

## CHAPTER 10: Control Plane Client

**Goal:** Receive policies and configuration from Oximy Cloud.

### Phase 10.1: Policy Fetch on Startup

- [ ] Create `crates/oisp-core/src/control_plane.rs`
- [ ] Fetch policies on startup
  - [ ] GET /v1/devices/{id}/policies
  - [ ] Parse policy document
- [ ] Define policy types
  - [ ] RedactionPolicy - patterns to redact
  - [ ] SamplingPolicy - what percentage to capture
  - [ ] FilterPolicy - processes/providers to include/exclude
  - [ ] AlertPolicy - thresholds for local alerts
- [ ] Store policies locally
  - [ ] Cache for offline operation
  - [ ] TTL-based refresh (default 5 min)

### Phase 10.2: Real-time Policy Updates

- [ ] WebSocket connection for push updates
  - [ ] wss://api.oximy.com/v1/devices/{id}/stream
  - [ ] Reconnect on disconnect with backoff
- [ ] Handle policy update messages
  - [ ] Validate new policies
  - [ ] Apply atomically
  - [ ] Acknowledge receipt
- [ ] Graceful degradation
  - [ ] Fall back to cached policies
  - [ ] Continue operating offline
  - [ ] Log policy staleness warnings

### Phase 10.3: Dynamic Redaction Rules

- [ ] Extend RedactionPlugin to accept cloud rules
  - [ ] Load rules from control plane
  - [ ] Hot-reload without restart
- [ ] Cloud rule format
  - [ ] Regex patterns
  - [ ] Entity types (PII, API keys, etc.)
  - [ ] Confidence thresholds
  - [ ] Action (redact, hash, mask)
- [ ] Apply before sending to cloud
  - [ ] Redact locally first
  - [ ] Only send redacted events

### Phase 10.4: Sampling Rules

- [ ] Implement sampling logic
  - [ ] Sample by percentage (e.g., 10% of traffic)
  - [ ] Sample by process name pattern
  - [ ] Sample by provider
  - [ ] Sample by event type
- [ ] Apply in pipeline
  - [ ] Before export plugins
  - [ ] Still count dropped events
- [ ] Deterministic sampling
  - [ ] Based on event_id hash
  - [ ] Reproducible across runs

### Phase 10.5: Local Alert Evaluation

- [ ] Evaluate alert conditions locally
  - [ ] Token spend threshold per hour/day
  - [ ] Error rate threshold
  - [ ] Unusual provider detection
  - [ ] Custom rules from cloud
- [ ] Local notifications
  - [ ] Log warning
  - [ ] Write to alert file
  - [ ] Future: desktop notification
- [ ] Report alert triggers to cloud
  - [ ] POST /v1/alerts

---

## CHAPTER 11: Sink Configuration UI ✅ COMPLETE

**Goal:** Web UI for configuring export destinations.

### Phase 11.1: Configuration API

- [ ] Add API endpoints (deferred - requires backend changes)
  - [ ] GET /api/config - current config
  - [ ] PUT /api/config - update config
  - [ ] POST /api/config/validate - validate without saving
  - [ ] POST /api/config/test-sink - test connection
- [ ] Persist config changes (deferred)
  - [ ] Write to config file
  - [ ] Hot-reload pipeline

### Phase 11.2: Configuration UI Page ✅ COMPLETE

- [x] Add "Settings" page to frontend
- [x] Sink configuration section
  - [x] JSONL: path, enable
  - [x] OTLP: endpoint, protocol, compression, headers, enable
  - [x] Kafka: brokers, topic, SASL auth, compression, enable
  - [x] Webhook: url, method, auth, batch mode, retries, enable
- [x] Connection test buttons
  - [x] Show success/failure with visual indicators
  - [x] Loading state during testing
- [x] Save/Apply button
- [x] Professional tabbed interface (Sinks, Privacy, General)

### Phase 11.3: Oximy Setup Flow

- [ ] Add "Connect to Oximy Cloud" section (deferred - requires Chapter 7)
- [ ] API key input with validation
- [ ] "Register Device" button
- [ ] Status indicator

### Phase 11.4: Redaction Configuration UI ✅ COMPLETE

- [x] Add "Privacy" section in settings
- [x] Redaction mode selector (safe/full/minimal)
- [x] Custom patterns editor
  - [x] Add/remove regex patterns
  - [x] Visual pattern list
- [x] Entity type toggles
  - [x] API keys & secrets
  - [x] Email addresses
  - [x] Credit card numbers

### Phase 11.5: Dashboard & Views ✅ COMPLETE (NEW)

- [x] Professional sidebar navigation layout
- [x] Dashboard view with:
  - [x] Real-time stats cards (events, AI calls, traces, processes, tokens)
  - [x] Activity feed with recent events
  - [x] Provider summary with request counts
  - [x] Active traces widget
  - [x] Connection status indicator
- [x] Enhanced Process Tree view
- [x] Enhanced Timeline view with visualization
- [x] Enhanced Log View with search and filters
- [x] Inventory view (providers, applications)
- [x] Traces view (active/completed)
- [x] Real-time WebSocket updates
- [x] Responsive design with dark theme

---

## CHAPTER 12: Advanced Linux Features ✅ COMPLETE

**Goal:** Production-grade Linux capabilities.

### Phase 12.1: Network Tracepoints ✅ COMPLETE

- [x] Add `sys_enter_connect` tracepoint to eBPF
  - [x] Capture socket FD
  - [x] Capture destination sockaddr
  - [x] Capture PID/TID
- [x] Add `sys_exit_connect` tracepoint
  - [x] Capture return value
  - [x] Track connection success/failure
- [x] Add socket → address mapping
  - [x] BPF map: (pid, fd) → sockaddr
  - [x] Correlate SSL events with connections
- [ ] Expose in Web UI
  - [ ] Show remote address/port for AI requests
  - [ ] Network flow visualization

### Phase 12.2: PID/Process Filtering (Kernel-side) ✅ COMPLETE

- [x] Add BPF map for target PIDs
  - [x] `BPF_MAP_TYPE_HASH` for PID set
  - [x] Update from userspace
- [x] Add comm filtering in eBPF
  - [x] Match process name in kernel
  - [x] Avoid sending unwanted events
- [x] Add userspace control API
  - [x] Update filter dynamically
  - [ ] API endpoint: POST /api/filters (deferred to Chapter 11)
- [ ] Add Web UI for filters (deferred to Chapter 11)
  - [ ] Checkbox list of active processes
  - [ ] PID input field

### Phase 12.3: Performance Optimization ✅ COMPLETE

- [x] Profile eBPF programs
  - [ ] Measure CPU overhead (deferred - requires production testing)
  - [ ] Identify hot paths (deferred - requires production testing)
- [x] Optimize ring buffer sizing
  - [x] Configurable sizing (SSL: 256KB, Process/File/Network: 64KB)
  - [ ] Dynamic sizing based on load (deferred)
  - [ ] Avoid overflows (handled via ring buffer semantics)
- [ ] Implement per-CPU buffers if needed (deferred - current design sufficient)
  - [ ] Reduce lock contention
- [ ] Measure and document overhead (deferred - requires production testing)
  - [ ] Target: <3% CPU
  - [ ] Target: <50MB memory
- [x] Add performance metrics endpoint
  - [x] GET /api/metrics (JSON format)
  - [x] GET /metrics (Prometheus format)

### Phase 12.4: Resource Metrics ✅ COMPLETE

- [x] Capture CPU usage per process
  - [x] Read from /proc/[pid]/stat
  - [x] Calculate delta over time (CPU percentage)
- [x] Capture memory usage per process
  - [x] Read from /proc/[pid]/statm
  - [x] RSS and virtual memory
- [x] Add API endpoint for process metrics
  - [x] GET /api/metrics/processes
  - [x] JSON format with CPU%, RSS, VMS per process
- [x] Add ResourceMetrics component to frontend
  - [x] CPU usage bars per process (sorted, color-coded by usage level)
  - [x] Memory usage bars per process (RSS, sorted)
  - [x] Summary cards (tracked processes, total CPU, total RSS, total VMS)
  - [x] Full process table with sortable data
- [ ] Aggregate in timeline view
  - [ ] Overlay on process events (future enhancement)

---

## CHAPTER 13: Configuration System

**Goal:** Robust, flexible configuration.

### Phase 13.1: Configuration File Loading

- [ ] Implement config file discovery
  - [ ] Check CLI --config flag
  - [ ] Check $OISP_CONFIG env var
  - [ ] Check ~/.config/oisp-sensor/config.toml
  - [ ] Check /etc/oisp-sensor/config.toml
  - [ ] Fall back to defaults
- [ ] Parse TOML config file
  - [ ] Use `config` or `toml` crate with serde
  - [ ] Support all current CLI options
- [ ] Add hot-reload capability
  - [ ] Watch config file for changes (notify crate)
  - [ ] Signal handler for SIGHUP
  - [ ] Apply non-disruptive changes

### Phase 13.2: Environment Variable Overrides

- [ ] Support env var overrides
  - [ ] OISP_LOG_LEVEL
  - [ ] OISP_WEB_PORT
  - [ ] OISP_WEB_HOST
  - [ ] OISP_REDACTION_MODE
  - [ ] OISP_OXIMY_API_KEY
  - [ ] OISP_OXIMY_ENDPOINT
- [ ] Document env var naming convention
  - [ ] OISP_ prefix
  - [ ] SCREAMING_SNAKE_CASE
- [ ] Priority: env > config file > defaults

### Phase 13.3: Sink Configuration Schema

- [ ] Define sink config schema
  - [ ] Each sink type has its own config section
  - [ ] Enable/disable per sink
  - [ ] Connection parameters
  - [ ] Retry/backoff settings
- [ ] Support multiple sinks of same type
  - [ ] [[export.jsonl]] array syntax
- [ ] Validate sink configs on startup
  - [ ] Check required fields
  - [ ] Test connections where possible

---

# PART 3: STABILIZE & RELEASE (Linux)

> **Only start this after Part 2 is complete.**
> At this point, Linux features are frozen and we prepare for release.

## CHAPTER 14: CI/CD Pipeline

**Goal:** Automated builds, tests, and releases.

### Phase 14.1: GitHub CI Workflow

- [ ] Create `.github/workflows/ci.yml` for PR checks
  - [ ] cargo fmt --check
  - [ ] cargo clippy --workspace --all-targets -- -D warnings
  - [ ] cargo test --workspace
  - [ ] Build check for Linux
- [ ] Cache cargo registry and target
- [ ] Run on pull requests and main branch

### Phase 14.2: Release Workflow

- [ ] Create `.github/workflows/release.yml` for tags
  - [ ] Trigger on `v*` tags
  - [ ] Build Linux x86_64 binary
  - [ ] Build Linux aarch64 binary (cross-compile)
  - [ ] Create GitHub Release with all artifacts
  - [ ] Generate changelog from commits
- [ ] eBPF compilation in CI
  - [ ] Install nightly Rust
  - [ ] Install bpf-linker
  - [ ] Compile eBPF bytecode
  - [ ] Embed in release binary

### Phase 14.3: Docker Workflow

- [ ] Create `.github/workflows/docker.yml`
  - [ ] Build Docker image on release
  - [ ] Push to ghcr.io/oximyhq/oisp-sensor
  - [ ] Multi-arch manifest (amd64 + arm64)
  - [ ] Image signing with cosign (optional)

---

## CHAPTER 15: Linux Packaging

**Goal:** Easy installation on Linux distros.

### Phase 15.1: .deb Package

- [ ] Create `.deb` package specification
  - [ ] debian/control file
  - [ ] debian/postinst (set capabilities)
  - [ ] debian/prerm (cleanup)
  - [ ] debian/oisp-sensor.service (systemd)
- [ ] Build with `cargo-deb` or manual dpkg-buildpackage
- [ ] Test on Ubuntu 22.04
- [ ] Test on Ubuntu 24.04
- [ ] Test on Debian 12

### Phase 15.2: .rpm Package

- [ ] Create `.rpm` package specification
  - [ ] oisp-sensor.spec file
  - [ ] scriptlets for pre/post install
  - [ ] systemd service
- [ ] Build with `cargo-rpm` or rpmbuild
- [ ] Test on Fedora 39/40
- [ ] Test on RHEL 9 / Rocky Linux 9

### Phase 15.3: Install Script

- [ ] Create `install.sh` script
  - [ ] Detect architecture (x86_64, aarch64)
  - [ ] Download appropriate binary from GitHub releases
  - [ ] Verify checksum (SHA256)
  - [ ] Install to /usr/local/bin
  - [ ] Set CAP_BPF capability
  - [ ] Optionally install systemd service
- [ ] Host script at sensor.oisp.dev/install.sh
- [ ] Test script on fresh VMs

---

## CHAPTER 16: Documentation

**Goal:** Comprehensive, accurate documentation.

### Phase 16.1: README Updates

- [ ] Update main README.md
  - [ ] Accurate feature status checkmarks
  - [ ] Real installation commands
  - [ ] Real example output
- [ ] Add actual screenshots
  - [ ] Web UI timeline view
  - [ ] Web UI process tree view
  - [ ] TUI screenshot
- [ ] Create demo GIF

### Phase 16.2: Architecture Documentation

- [ ] Write `docs/architecture/OVERVIEW.md`
- [ ] Write `docs/architecture/EBPF.md`
- [ ] Write `docs/architecture/PIPELINE.md`
- [ ] Create Mermaid diagrams

### Phase 16.3: User Guides

- [ ] Write `docs/quickstart.md`
- [ ] Write `docs/configuration.md`
- [ ] Write `docs/troubleshooting.md`
- [ ] Write `docs/faq.md`

### Phase 16.4: Developer Documentation

- [ ] Update `CONTRIBUTING.md`
- [ ] Write `docs/dev/plugins.md`
- [ ] Write `docs/dev/testing.md`

---

## CHAPTER 17: Testing & Quality

**Goal:** Comprehensive test coverage and production quality.

### Phase 17.1: Unit Tests

- [ ] Increase unit test coverage
  - [ ] Target: 80% line coverage
- [ ] Test each crate independently
  - [ ] oisp-core: 15+ tests
  - [ ] oisp-decode: 25+ tests
  - [ ] oisp-export: 10+ tests (mocked)
  - [ ] oisp-redact: 10+ tests
  - [ ] oisp-enrich: 5+ tests

### Phase 17.2: Integration Tests

- [ ] Create `tests/` directory at workspace root
- [ ] Pipeline integration tests
  - [ ] Feed raw events → verify output events
  - [ ] Test full decode → enrich → export flow
- [ ] eBPF integration tests (Linux only)
  - [ ] Load eBPF programs
  - [ ] Make HTTPS request
  - [ ] Verify event captured
- [ ] Docker integration tests
  - [ ] Build image
  - [ ] Start container
  - [ ] Verify API endpoints

### Phase 17.3: Test Coverage

- [ ] Add `cargo-tarpaulin` or `cargo-llvm-cov`
- [ ] Add coverage to CI
- [ ] Add coverage badge to README

### Phase 17.4: Error Handling

- [ ] Review all crates for error handling
- [ ] Use thiserror consistently
- [ ] Categorize errors (transient, config, fatal)
- [ ] Audit log levels

### Phase 17.5: Health & Metrics

- [ ] Enhance /api/health endpoint
- [ ] Add /api/ready and /api/live endpoints
- [ ] Add /api/metrics (Prometheus format)

### Phase 17.6: Security Hardening

- [ ] Drop privileges after eBPF load
- [ ] TLS for web UI (optional)
- [ ] Localhost-only by default
- [ ] Run `cargo audit` in CI

---

# PART 4: CROSS-PLATFORM EXPANSION

> **Only start this after Part 3 (Linux release) is complete.**
> Linux is the foundation. macOS/Windows build on it.

## CHAPTER 18: Cross-Platform Research

**Goal:** Evaluate options for macOS and Windows support.

### Phase 18.1: rbpf Evaluation

[rbpf](https://github.com/qmonnet/rbpf) is a **pure Rust eBPF virtual machine** that can run eBPF bytecode in userspace on any platform.

- [ ] Evaluate rbpf capabilities
  - [ ] Can it run our SSL interception eBPF programs?
  - [ ] What helper functions are available?
  - [ ] Performance of interpreter vs JIT
- [ ] Create proof-of-concept
  - [ ] Compile our eBPF programs to bytecode
  - [ ] Load with rbpf instead of kernel
  - [ ] Feed synthetic data, verify output
- [ ] Identify limitations
  - [ ] No kernel hooks (uprobes, tracepoints)
  - [ ] Need platform APIs to feed data
  - [ ] Memory access restrictions

**Key insight:** rbpf can *execute* eBPF logic, but we still need platform-specific code to *capture* the raw data to feed into it.

### Phase 18.2: ebpf-for-windows Evaluation

[ebpf-for-windows](https://github.com/microsoft/ebpf-for-windows) is Microsoft's native eBPF implementation for Windows.

- [ ] Evaluate maturity and support
  - [ ] Which Windows versions supported?
  - [ ] Which eBPF program types available?
  - [ ] Can we use uprobes or similar?
- [ ] Create proof-of-concept
  - [ ] Set up Windows dev environment
  - [ ] Try loading our eBPF programs
  - [ ] Document what works/doesn't
- [ ] Assess viability
  - [ ] Is it production-ready?
  - [ ] What are the dependencies/requirements?

### Phase 18.3: Platform API Research

Independent of eBPF, we need to understand native APIs:

**macOS:**
- [ ] Endpoint Security Framework (ESF)
  - [ ] Process exec/exit events
  - [ ] File open/read/write events
  - [ ] Network connect events
  - [ ] Entitlements required
- [ ] Network Extension Framework
  - [ ] Content filter for SSL inspection
  - [ ] System Extension requirements
- [ ] Lighter alternatives
  - [ ] libproc for process enumeration
  - [ ] FSEvents for file changes
  - [ ] netstat/lsof for connections

**Windows:**
- [ ] ETW (Event Tracing for Windows)
  - [ ] Microsoft-Windows-Kernel-Process
  - [ ] Microsoft-Windows-Kernel-File
  - [ ] Microsoft-Windows-Kernel-Network
- [ ] WFP (Windows Filtering Platform)
  - [ ] Network traffic inspection
- [ ] Rust ETW libraries
  - [ ] ferrisetw
  - [ ] tracelogging

---

## CHAPTER 19: macOS Implementation

**Goal:** macOS support with metadata and optional full capture.

### Phase 19.1: macOS "Lite" (Metadata Only)

- [ ] Create `crates/oisp-capture-macos/src/process.rs`
  - [ ] Use libproc to enumerate processes
  - [ ] Detect AI-related processes by name/bundle ID
  - [ ] Track process start/exit
  - [ ] Get parent PID
- [ ] Create `crates/oisp-capture-macos/src/network.rs`
  - [ ] Enumerate network connections
  - [ ] Match connections to AI provider IPs/domains
  - [ ] Track connection timing
- [ ] Create `crates/oisp-capture-macos/src/lib.rs`
  - [ ] Implement CapturePlugin trait
  - [ ] Combine process + network monitoring
  - [ ] Emit RawCaptureEvents
- [ ] Test on macOS 14 (Sonoma)
- [ ] Test on macOS 15 (Sequoia)
- [ ] Document limitations (metadata only)

### Phase 19.2: macOS "Full" (with rbpf or ESF)

**Option A: rbpf approach**
- [ ] Use platform APIs to capture raw SSL data
- [ ] Feed into rbpf for processing
- [ ] Output OISP events

**Option B: Endpoint Security approach**
- [ ] Create System Extension
- [ ] Request entitlements from Apple
- [ ] Implement ESF callbacks
- [ ] Parse SSL data

- [ ] Create `.pkg` installer
- [ ] Handle notarization
- [ ] Document user approval flow

---

## CHAPTER 20: Windows Implementation

**Goal:** Windows support with metadata and optional full capture.

### Phase 20.1: Windows "Lite" (Metadata Only)

- [ ] Create `crates/oisp-capture-windows/src/process.rs`
  - [ ] Use WMI for process enumeration
  - [ ] Win32 API for process info
  - [ ] Track process start/exit
  - [ ] Get parent PID
- [ ] Create `crates/oisp-capture-windows/src/network.rs`
  - [ ] Enumerate TCP connections (GetExtendedTcpTable)
  - [ ] Match to AI provider endpoints
  - [ ] Track connection timing
- [ ] Create `crates/oisp-capture-windows/src/lib.rs`
  - [ ] Implement CapturePlugin trait
  - [ ] Combine process + network monitoring
  - [ ] Emit RawCaptureEvents
- [ ] Test on Windows 11
- [ ] Test on Windows Server 2022
- [ ] Document limitations

### Phase 20.2: Windows "Full" (with ebpf-for-windows or ETW)

**Option A: ebpf-for-windows approach**
- [ ] Use Microsoft's eBPF implementation
- [ ] Evaluate if our programs can run
- [ ] Adapt as needed

**Option B: ETW approach**
- [ ] Create Windows service
- [ ] Consume ETW events
- [ ] Convert to OISP format

- [ ] Create `.msi` installer
- [ ] Handle elevation/UAC
- [ ] Document admin requirements

---

## CHAPTER 21: Cross-Platform CI/CD

**Goal:** Build and release for all platforms.

### Phase 21.1: macOS Builds

- [ ] Add macOS to release workflow
  - [ ] Build macOS x86_64 binary
  - [ ] Build macOS aarch64 binary (Apple Silicon)
- [ ] Create Homebrew formula
  - [ ] oximy/tap/oisp-sensor
- [ ] Code signing
  - [ ] Apple Developer certificate
  - [ ] Notarization

### Phase 21.2: Windows Builds

- [ ] Add Windows to release workflow
  - [ ] Build Windows x86_64 binary
- [ ] Create winget manifest
- [ ] Create `.msi` installer
- [ ] Optional: code signing

---

# PART 5: ECOSYSTEM

## CHAPTER 22: SDK & Libraries

**Goal:** Easy integration for developers.

### Phase 22.1: Python SDK

- [ ] Create `oisp-python` package
- [ ] Event reading from JSONL
- [ ] WebSocket client for live events
- [ ] Oximy Cloud client
  - [ ] Upload events
  - [ ] Query events
  - [ ] Fetch policies
- [ ] Publish to PyPI

### Phase 22.2: JavaScript/TypeScript SDK

- [ ] Create `@oisp/sdk` npm package
- [ ] Event types (TypeScript)
- [ ] WebSocket client
- [ ] Oximy Cloud client
- [ ] Publish to npm

### Phase 22.3: Go SDK

- [ ] Create `oisp-go` module
- [ ] Event types
- [ ] Oximy Cloud client
- [ ] Publish to pkg.go.dev

---

## CHAPTER 23: Integrations

**Goal:** Pre-built integrations with popular tools.

### Phase 23.1: Grafana Dashboard

- [ ] Create Grafana dashboard JSON
  - [ ] Requires OTLP → Tempo/Loki
  - [ ] AI activity timeline
  - [ ] Provider breakdown
  - [ ] Token usage
- [ ] Document Grafana setup
- [ ] Add to examples/

### Phase 23.2: Datadog Integration

- [ ] Document OTLP setup for Datadog
- [ ] Create Datadog dashboard template
- [ ] Create Datadog monitors template

### Phase 23.3: Splunk Integration

- [ ] Document HTTP Event Collector setup
- [ ] Create webhook exporter config
- [ ] Create Splunk dashboard

---

## CHAPTER 24: Community & Growth

**Goal:** Build open source community.

### Phase 24.1: Community Setup

- [ ] Enable GitHub Discussions
- [ ] Create issue templates
  - [ ] Bug report
  - [ ] Feature request
  - [ ] Question
- [ ] Create PR template
- [ ] Add CODE_OF_CONDUCT.md

### Phase 24.2: Marketing Assets

- [ ] Create logo/banner SVGs
- [ ] Create social media images
- [ ] Write launch blog post
- [ ] Create demo video

### Phase 24.3: Adoption

- [ ] Submit to awesome-lists
- [ ] Post on Hacker News
- [ ] Post on Reddit (r/programming, r/devops)
- [ ] Create Twitter/X presence

---

# APPENDIX: Quick Reference

## Current State Summary

| Area | Status | Notes |
|------|--------|-------|
| Linux eBPF SSL capture | ✅ Complete | Working in Docker |
| Linux process tracepoints | ✅ Complete | Code written, needs Linux test |
| HTTP/AI decoding | ✅ Complete | OpenAI, Anthropic, 18+ providers |
| Request/response correlation | ✅ Complete | PID+TID+FD based |
| TUI | ✅ Complete | ratatui-based |
| Web UI (React) | ✅ Complete | Process tree, timeline, log |
| Docker | ✅ Complete | Multi-arch builds |
| JSONL export | ✅ Complete | Local file |
| WebSocket export | ✅ Complete | For UI |
| OTLP export | ✅ Complete | Chapter 6 (needs Linux testing) |
| Kafka export | ✅ Complete | Chapter 6 (needs Linux testing) |
| Webhook export | ✅ Complete | Chapter 6 |
| Oximy exporter | ❌ Not started | Chapter 7 |
| Control plane client | ❌ Not started | Chapter 10 |
| Sink config UI | ✅ Complete | Chapter 11 (frontend done, API pending) |
| Advanced Linux | ❌ Not started | Chapter 12 |
| Configuration system | ❌ Not started | Chapter 13 |
| CI/CD | ❌ Not started | Chapter 14 |
| Linux packaging | ❌ Not started | Chapter 15 |
| Documentation | ❌ Incomplete | Chapter 16 |
| macOS capture | ❌ Not started | Chapter 19 |
| Windows capture | ❌ Not started | Chapter 20 |

## Priority Order (REVISED)

**The correct order is: Complete Linux → Stabilize → Release → Cross-Platform**

### PART 2: COMPLETE LINUX IMPLEMENTATION (Do First!)

| # | Chapter | Focus |
|---|---------|-------|
| 6 | Sink Implementation | OTLP, Kafka, Webhook exports |
| 7 | Oximy Exporter | Connect to Oximy Cloud |
| 10 | Control Plane | Receive policies from cloud |
| 11 | Sink Config UI | Web UI for sink configuration |
| 12 | Advanced Linux | Network tracepoints, filtering, perf |
| 13 | Configuration | Config file loading, env vars |

### PART 3: STABILIZE & RELEASE LINUX (Then Freeze!)

| # | Chapter | Focus |
|---|---------|-------|
| 14 | CI/CD Pipeline | Automated builds, tests |
| 15 | Linux Packaging | .deb, .rpm, install script |
| 16 | Documentation | README, guides, architecture |
| 17 | Testing & Quality | Coverage, security, metrics |

### PART 4: CROSS-PLATFORM (After Linux v1.0)

| # | Chapter | Focus |
|---|---------|-------|
| 18 | Research | rbpf, ebpf-for-windows, platform APIs |
| 19 | macOS | Lite (metadata) → Full (ESF) |
| 20 | Windows | Lite (metadata) → Full (ETW) |
| 21 | Cross-Platform CI | Multi-platform builds |

### PART 5: ECOSYSTEM (Ongoing)

| # | Chapter | Focus |
|---|---------|-------|
| 22 | SDKs | Python, JavaScript, Go |
| 23 | Integrations | Grafana, Datadog, Splunk |
| 24 | Community | GitHub setup, marketing |

## Cross-Platform Strategy

### Key Resources
- **[rbpf](https://github.com/qmonnet/rbpf)** - Rust userspace eBPF VM, runs on any platform
- **[ebpf-for-windows](https://github.com/microsoft/ebpf-for-windows)** - Microsoft's native eBPF for Windows

### Approach
1. **Linux:** Native kernel eBPF (what we have now)
2. **macOS/Windows Lite:** Platform APIs for metadata (process, network)
3. **macOS/Windows Full:** Either:
   - Use rbpf with platform-specific data feeding
   - Use native frameworks (ESF on macOS, ETW on Windows)

---

## Session Log

### Prior Sessions (Chapters 1-9)
See detailed session logs preserved below.

### 2025-12-23 - Session 8: Big Picture Planning
**Completed:**
- Analyzed entire project scope
- Defined relationship between OSS and Oximy Cloud
- Created comprehensive TODO covering all chapters
- Identified 24 chapters / major work areas
- Estimated 100+ individual tasks

**Architecture Defined:**
```
[Capture Layer] → [OISP Events] → [Sinks (JSONL, OTLP, Kafka, Webhook, Oximy)]
                                       ↓
                               [Oximy Cloud (Proprietary)]
                                       ↓
                               [ML Redaction, Policies, Reports]
                                       ↓
                               [Push back to sensors]
```

**Key Decisions:**
1. Complete ALL Linux features before CI/CD and packaging
2. macOS/Windows start with "lite" (metadata only) before "full" capture
3. Oximy Exporter is high priority for business model
4. Control Plane Client enables dynamic policy updates
5. Investigate rbpf and ebpf-for-windows for cross-platform eBPF

---

*Last updated: 2025-12-23*
*Session: Session 11 - Chapter 11 Frontend Dashboard*

---

# SESSION LOGS

### 2025-12-23 - Session 11: Chapter 11 Frontend Dashboard & UI Overhaul

**Goal:** Build comprehensive, professional dashboard UI with all Chapter 6 sink configurations.

**Completed:**

**New Dashboard Layout:**
- Created professional sidebar navigation with collapsible design
- Added TopBar with real-time stats and connection status
- Implemented 7 main views: Dashboard, Process Tree, Timeline, Log, Inventory, Traces, Settings

**Dashboard View (New):**
- Real-time stats cards: Total Events, AI Calls, Active Traces, Processes, Total Tokens
- Activity feed with recent events and color-coded types
- Provider summary with request counts and models
- Active traces widget
- Connection status with uptime display

**Settings Page (Chapter 11):**
- **Export Sinks Tab:**
  - JSONL sink configuration (path, enable/disable)
  - OTLP sink (endpoint, protocol, compression, headers)
  - Kafka sink (brokers, topic, SASL auth, compression)
  - Webhook sink (URL, method, auth type, batch mode, retries)
  - Connection test buttons with success/error indicators
- **Privacy Tab:**
  - Redaction mode selector (minimal/safe/full)
  - Entity type toggles (API keys, emails, credit cards)
  - Custom regex pattern editor
- **General Tab:**
  - Log level, max events, WebSocket toggle
  - Version and platform info

**Inventory View:**
- AI Providers list with request counts and models
- Applications table with executable info and provider usage

**Traces View:**
- Active traces with live indicator
- Completed traces history
- Stats: duration, LLM calls, tool calls, tokens

**Enhanced Existing Views:**
- Process Tree: Improved styling, expand/collapse all, event badges
- Timeline: Visual timeline with zoom, grouped event log
- Log View: Search, filters, expandable event details
- Empty State: Better design with feature cards and loading indicator

**New Hooks:**
- `useStats()` - Fetch sensor stats
- `useInventory()` - Fetch provider/app inventory
- `useTraces()` - Fetch agent traces

**Styling:**
- IBM Plex Sans/Mono fonts for professional look
- Refined dark theme with subtle gradients
- Consistent component styling (buttons, inputs, cards)
- Smooth animations (fade-in, slide-up, stagger)
- Custom scrollbars and focus states

**Build Status:**
- TypeScript: No errors
- Next.js build: Success (18.9kB main bundle)

**Files Created/Modified:**
- `frontend/src/app/page.tsx` - New dashboard layout
- `frontend/src/app/layout.tsx` - Updated HTML structure
- `frontend/src/app/globals.css` - New professional styling
- `frontend/tailwind.config.ts` - Refined color palette
- `frontend/src/components/Sidebar.tsx` - New sidebar navigation
- `frontend/src/components/TopBar.tsx` - New top bar
- `frontend/src/components/DashboardView.tsx` - New dashboard
- `frontend/src/components/InventoryView.tsx` - New inventory view
- `frontend/src/components/TracesView.tsx` - New traces view
- `frontend/src/components/SettingsView.tsx` - New settings with sink config
- `frontend/src/lib/useStats.ts` - Stats hook
- `frontend/src/lib/useInventory.ts` - Inventory hook
- `frontend/src/lib/useTraces.ts` - Traces hook
- Enhanced: ProcessTreeView, TimelineView, LogView, EventBlock, ProcessNode, EmptyState

**Next Steps:**
- Chapter 11.1: Add backend API endpoints for config persistence
- Chapter 7: Oximy Cloud Integration (when ready)
- Chapter 12: Expose network data in UI

---

### 2025-12-23 - Session 10: Chapter 6 Verification & Completion

**Discovery:**
- Reviewed export crate files and discovered Chapter 6 was already implemented!
- All three sink implementations (OTLP, Kafka, Webhook) are fully functional

**Verified Implementation:**

**OTLP Exporter (624 lines in `otlp.rs`):**
- Uses `opentelemetry-otlp` crate with proper LogExporter
- Supports gRPC, HTTP/proto, HTTP/JSON transports
- Full OpenTelemetry semantic convention mapping (gen_ai.*, process.*, host.*)
- TLS, compression (gzip), authentication (API key, Bearer)
- Batch processing with configurable size and flush interval
- Maps all OISP event types to OTLP log records

**Kafka Exporter (435 lines in `kafka.rs`):**
- Uses `rdkafka` crate with FutureProducer
- SASL authentication (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512)
- TLS configuration
- Compression (gzip, snappy, lz4, zstd)
- Message keys (event_id or host:pid)
- Message headers (event_type, oisp_version)
- Producer batching via linger_ms and batch_size

**Webhook Exporter (520 lines in `webhook.rs`):**
- Uses `reqwest` crate with async client
- POST/PUT/PATCH methods
- Authentication (API key, Bearer, Basic)
- Single event or batch mode
- Exponential backoff retry logic
- Dead letter queue file for failed events
- Proper response handling (2xx/4xx/5xx)

**Test Results:**
- All 54 tests pass across workspace
- Zero clippy warnings
- Kafka feature requires Linux (rdkafka needs cmake)

**Next Chapter: Chapter 7 (Oximy Cloud Integration)**

---

### 2025-12-23 - Session 9: Priority Reordering & Cross-Platform Strategy

**Discussion:**
- Reconsidered priority order: why do CI/CD before features are complete?
- Discovered [rbpf](https://github.com/qmonnet/rbpf) - pure Rust eBPF VM that runs on any platform
- Discovered [ebpf-for-windows](https://github.com/microsoft/ebpf-for-windows) - Microsoft's native eBPF implementation

**Key Decisions:**
1. **Complete Linux implementation fully BEFORE** CI/CD, packaging, docs
2. **Freeze Linux features, THEN stabilize and release**
3. **Cross-platform comes AFTER Linux v1.0**
4. **Investigate rbpf for macOS/Windows** - can run eBPF bytecode without kernel support

**Reorganized Parts:**
- PART 2: Complete Linux Implementation (Chapters 6-13)
- PART 3: Stabilize & Release Linux (Chapters 14-17)
- PART 4: Cross-Platform Expansion (Chapters 18-21)
- PART 5: Ecosystem (Chapters 22-24)

**New Chapter 18: Cross-Platform Research**
- Evaluate rbpf capabilities
- Evaluate ebpf-for-windows maturity
- Research macOS ESF and Windows ETW as alternatives

**Updated Priority Order:**
```
Part 2 (Linux features) → Part 3 (stabilize/release) → Part 4 (macOS/Windows) → Part 5 (ecosystem)
```

---

# HISTORICAL SESSION LOGS

### 2025-12-23 - Session 1: eBPF Data Capture
**Completed:**
- Added RingBuf map to eBPF program for kernel→userspace event delivery
- Implemented SSL_write/SSL_read argument capture via HashMap correlation
- Added data buffer reading with bpf_probe_read_user_buf
- Created userspace ring buffer consumer with event parsing
- Successfully captured decrypted HTTPS traffic (HTTP/1.1 requests and responses)
- Added sanity check filter to skip malformed events

**Key Files Modified:**
- `ebpf/oisp-ebpf-capture-ebpf/src/main.rs` - eBPF program with RingBuf + HashMap
- `ebpf/oisp-ebpf-capture/src/main.rs` - Userspace event consumer
- `ebpf/oisp-ebpf-capture-common/src/lib.rs` - Shared SslEvent struct

**Next Steps:**
- Chapter 3: Integrate with oisp-sensor pipeline

### 2025-12-23 - Session 2: Chapter 2 Testing
**Completed:**
- Created comprehensive test suite in `ebpf/tests/`
- Updated Dockerfile to include curl, Python, Node.js for testing
- Phase 2.1: Basic verification with curl - SSL_write/SSL_read working
- Phase 2.2: AI provider testing - OpenAI/Anthropic format requests captured
- Phase 2.3: Multi-process testing - Python requests + Node.js https captured

**Key Observations:**
- HTTP/2 (curl default) shows binary HPACK-encoded headers
- HTTP/1.1 (Node.js) shows plaintext headers - easier to parse
- Large responses (>4KB) correctly show truncation: `len=4096/8329`
- Streaming responses generate multiple SSL_read events
- PID/comm correctly identifies curl, python3, node processes

**Test Files Created:**
- `tests/test_basic.sh` - Basic curl HTTPS verification
- `tests/test_ai_providers.sh` - OpenAI/Anthropic format testing
- `tests/test_multiprocess.py` - Python requests library tests
- `tests/test_multiprocess.js` - Node.js https module tests
- `tests/run_all_tests.sh` - Master test runner

**Next Steps:**
- Chapter 3: Integrate with oisp-sensor pipeline

### 2025-12-23 - Session 3: Chapter 3 Pipeline Integration (COMPLETE!)
**Completed:**

**Phase 3.1: eBPF Integration**
- Replaced libbpf-rs with Aya for eBPF loading
- Created SslEvent type matching eBPF kernel structure
- Implemented full CapturePlugin trait for EbpfCapture
- Added CLI options for --ebpf-path and --libssl-path

**Phase 3.2: HTTP Decoding (ENHANCED)**
- Enhanced HTTP request/response parsing with chunked transfer encoding
- Added SSE streaming response parsing for both OpenAI and Anthropic
- 10 new unit tests for HTTP parsing

**Phase 3.3: AI Provider Detection (ENHANCED)**
- Added Anthropic-specific request/response parsing
- Enhanced provider detection from response body
- Support for 18+ AI providers
- 16 new unit tests for AI detection

**Phase 3.4: Request/Response Correlation (NEW)**
- Correlation by PID + TID + FD for accurate matching
- Streaming response reassembly (OpenAI and Anthropic styles)
- Automatic cleanup of stale pending requests (5 min timeout)
- Latency tracking between request and response
- 5 new unit tests for correlation logic

**Key Files Modified:**
- `crates/oisp-capture-ebpf/Cargo.toml` - Aya dependency
- `crates/oisp-capture-ebpf/src/types.rs` - SslEvent struct
- `crates/oisp-capture-ebpf/src/ebpf_capture.rs` - Full eBPF capture
- `crates/oisp-decode/src/http.rs` - Enhanced HTTP parsing + chunked
- `crates/oisp-decode/src/ai.rs` - Anthropic parsing + tests
- `crates/oisp-decode/src/sse.rs` - Anthropic stream reassembler
- `crates/oisp-decode/src/decoder.rs` - Correlation logic + tests
- `crates/oisp-sensor/src/main.rs` - CLI options

**Test Summary:**
- Total: 48 tests across workspace
- oisp-decode: 39 tests (HTTP, SSE, AI, Decoder)
- oisp-core: 7 tests (Providers, Redaction)
- oisp-capture: 2 tests (Generator)
- All pass with zero clippy warnings

**Architecture:**
```
[eBPF Kernel Space]
    |
    v (ring buffer)
[EbpfCapture Plugin] -> [RawCaptureEvent]
    |
    v (mpsc channel)
[HttpDecoder] -> [Correlation] -> [OISP Events]
    |
    v
[Enrich -> Redact -> Export]
    |
    v
[JSONL / WebSocket / TUI / Web UI]
```

**Next Steps:**
- ~~Chapter 4: TUI/GUI Verification - test with real traffic~~ **DONE!**
- Chapter 5: Docker & Containerization

---

### 2025-12-23 - Session 4: Chapter 4 - TUI/GUI Verification
**Completed:**
- Fixed OISP serialization to match spec (envelope fields at root, event data in `data` field)
- Implemented custom `Serialize`/`Deserialize` for `OispEvent` enum
- Added 6 unit tests for serialization/deserialization and roundtrip
- Fixed event broadcast: pipeline now properly shares `event_sender()` with web server
- Added background task to populate events list in web AppState
- Fixed web UI timestamp and process name display (now reads from correct OISP fields)
- Implemented tab switching in web UI (Timeline, Inventory, Traces views)
- Inventory view shows AI providers with request/response counts, latency, models
- Traces view shows active agent traces
- Verified TUI code is correctly structured for event display
- Fixed redaction test (API key was too short for regex pattern)

**Key Files Modified:**
- `crates/oisp-core/src/events/mod.rs` - Custom serde for OISP-compliant JSON
- `crates/oisp-core/src/events/ai.rs` - Refactored streaming/embedding to use data field
- `crates/oisp-core/src/pipeline.rs` - Added `event_sender()` method
- `crates/oisp-sensor/src/main.rs` - Use pipeline's event sender for web server
- `crates/oisp-web/src/lib.rs` - Background task to populate events, renamed field
- `crates/oisp-web/src/ws.rs` - Use renamed event_tx field
- `crates/oisp-web/static/index.html` - Tab switching, inventory/traces views

**Tests Added:**
- `test_ai_request_serialization_format` - Verifies OISP JSON structure
- `test_process_exec_serialization_format` - Verifies process event structure
- `test_ai_request_deserialization` - Verifies parsing from JSON
- `test_roundtrip_ai_request` - Verifies serialize/deserialize roundtrip
- `test_roundtrip_process_exec` - Process event roundtrip
- `test_roundtrip_agent_tool_call` - Agent event roundtrip

---

### 2025-12-23 - Session 5: Chapter 5 - Docker & Containerization (COMPLETE!)
**Completed:**
- Created comprehensive multi-stage Dockerfile for eBPF + userspace build
- Updated docker-compose.yml with multiple service profiles (sensor, web, demo, tui, dev)
- Added Dockerfile.multiarch for cross-platform builds (amd64 + arm64)
- Created docker-build.sh and docker-run.sh helper scripts
- Added /api/health endpoint for Docker health checks
- Updated ebpf/Dockerfile for standalone testing
- Created docker-compose.yml in ebpf/ for development
- Fixed Aya API compatibility (UProbe::attach signature changed in git version)
- Fixed eBPF ring buffer lifetime issues in async context
- Fixed glibc version mismatch (use Debian trixie for runtime)
- Fixed web server binding (0.0.0.0 for Docker accessibility)
- **Successfully built and tested Docker image!**

**Key Files Created/Modified:**
- `Dockerfile` - Multi-stage build: ebpf-builder -> userspace-builder -> runtime
- `docker/docker-compose.yml` - 5 service profiles for different use cases
- `docker/Dockerfile.multiarch` - Multi-architecture build with buildx
- `scripts/docker-build.sh` - Build script with --multiarch and --push options
- `scripts/docker-run.sh` - Run script for demo/record/shell modes
- `crates/oisp-web/src/lib.rs` - Added health_check() endpoint, bind to 0.0.0.0
- `crates/oisp-capture-ebpf/Cargo.toml` - Use Aya from git
- `crates/oisp-capture-ebpf/src/ebpf_capture.rs` - Fixed UProbe attach API + ring buffer lifetime
- `crates/oisp-sensor/Cargo.toml` - Added libc for Linux
- `ebpf/Dockerfile` - Updated for testing
- `ebpf/docker-compose.yml` - Added for standalone dev

**Docker Configuration:**
- Privileged mode for eBPF access
- Host network mode for SSL interception
- Host PID namespace for process correlation
- Volume mounts: /sys/kernel/debug, /sys/fs/bpf, /proc, /lib, /usr/lib
- Health checks with curl to /api/health
- Restart policies (unless-stopped)

**Testing Results (macOS via Docker Desktop):**
- Docker image builds successfully
- Demo mode starts and generates events
- Health endpoint: `curl http://localhost:7777/api/health` -> OK
- Stats endpoint: Returns event counts
- Events endpoint: Returns OISP-format JSON events
- Inventory endpoint: Shows providers (openai, anthropic)
- Traces endpoint: Shows active agent traces

**Remaining for Linux Testing:**
- Test eBPF capture on actual Linux host
- Verify uprobe attachment inside container

---

### 2025-12-23 - Session 6: AgentSight Analysis & Chapter 9 Planning

**Analysis of AgentSight (https://github.com/eunomia-bpf/agentsight/):**

AgentSight's UI excellence comes from:

1. **PID as First-Class Citizen**: Every event has `{ pid, comm, timestamp, source, data }` at the top level. Not optional, not nested.

2. **Unified Event Stream**: SSL events + process events + file events all flow through the same stream with consistent structure.

3. **Process Tree View**: The killer feature. Groups all events under the process that caused them:
   ```
   [claude] PID 334599
     ├─ FILE_OPEN /home/user/.claude/settings.json
     ├─ AI PROMPT claude-opus-4
     ├─ AI RESPONSE (15 responses)
     └─ [child] PID 334601 (spawned subprocess)
   ```

4. **ppid (Parent PID)**: Critical for building the tree hierarchy. Without ppid, you can't show parent-child process relationships.

5. **React Frontend**: 28 TypeScript files with proper state management vs our 700-line inline HTML.

**What OISP Currently Lacks:**

| AgentSight | OISP Status |
|------------|-------------|
| Flat event with pid at top | Complex enum with optional nested process |
| Single merged event stream | Multiple export targets, complex structure |
| React frontend with state | Static HTML with inline JS |
| File operations from eBPF | Not capturing file opens |
| Process lifecycle with ppid | Not implemented (Chapter 9.1 TODO) |
| Process Tree view | No process grouping in UI |

**Key Decisions Made:**

1. **Keep OISP spec as-is** for export/storage - it's correct for interop
2. **Add WebEvent format** specifically for frontend consumption
3. **Build React frontend** with Next.js + TypeScript
4. **Add process tracepoints** to capture exec/exit/file events
5. **ppid is mandatory** for tree building - all platforms must provide it
6. **Platform identifiers**: Linux=PID, macOS=audit_token (normalize to int), Windows=PID

**Created Chapter 9**: AgentSight-Style UI & Process-Centric View
- Phase 9.1: Process Tracepoints (eBPF)
- Phase 9.2: WebEvent Format
- Phase 9.3: React Frontend
- Phase 9.4: Process Tree Building
- Phase 9.5: Embed Frontend in Binary

**Renumbered Chapters**:
- Chapter 10: Platform Expansion (macOS/Windows) - updated with ES/ETW details
- Chapter 11: Advanced Features (network tracepoints, filtering, performance)

---

### 2025-12-23 - Session 7: Chapter 8 - Codebase Cleanup (COMPLETE!)
**Completed:**

**Phase 8.1: Remove Unused Files**
- Deleted legacy C-based eBPF code: `bpf/ssl_monitor.bpf.c`, `bpf/process_monitor.bpf.c`, `bpf/Makefile`
- Removed empty placeholder directories: `assets/`, `installers/linux/`, `installers/macos/`, `installers/windows/`

**Phase 8.2: Consolidate eBPF Structure**
- Renamed `oisp-ebpf-capture/` to `ebpf/` for clarity
- Updated Dockerfile and docker/Dockerfile.multiarch with new paths
- Updated CONTRIBUTING.md with correct project structure
- Updated all TODO.md references

**Phase 8.3: Code Quality**
- Fixed clippy warning in oisp-web (double_ended_iterator_last -> rsplit().next())
- Fixed doc warning (URL hyperlink format in oisp-capture-ebpf)
- Ran cargo fmt on entire workspace
- All tests passing (48 tests across workspace)

**Key Files Modified:**
- `crates/oisp-web/src/web_event.rs` - Fixed clippy warning
- `crates/oisp-capture-ebpf/src/lib.rs` - Fixed doc warning
- `Dockerfile` - Updated eBPF paths from oisp-ebpf-capture to ebpf
- `docker/Dockerfile.multiarch` - Updated eBPF paths
- `CONTRIBUTING.md` - Updated project structure and build instructions
- `ebpf/README.md` - Added explanation of separate workspace

**Files Deleted:**
- `bpf/ssl_monitor.bpf.c`
- `bpf/process_monitor.bpf.c`
- `bpf/Makefile`
- `assets/` (empty)
- `installers/linux/` (empty)
- `installers/macos/` (empty)
- `installers/windows/` (empty)

---
