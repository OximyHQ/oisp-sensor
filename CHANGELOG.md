# Changelog

All notable changes to OISP Sensor are documented in this file.

## [Unreleased]

### Added
- **Troubleshooting guide** - Comprehensive guide at `docs-site/src/content/docs/guides/troubleshooting.md`
- **Enhanced ssl-info command** - Now shows alternative TLS libraries (GnuTLS, NSS, BoringSSL) and unsupported implementations with clear guidance
- **Improved check command** - Added note about unsupported TLS libraries with reference to `ssl-info` command
- **multi-process/python-celery example** - Cookbook example showing OISP capturing AI calls from multiple Celery workers

### Changed
- **install.sh systemd service** - Synced with comprehensive version from `packaging/systemd/`:
  - Added `ExecStartPre` for directory creation
  - Added `ReadWritePaths` for security hardening
  - Added `LimitMEMLOCK=infinity` and `LimitNOFILE=65536` resource limits
  - Added `ExecReload` for SIGHUP config reload
  - Added `RUST_BACKTRACE=1` environment variable

### Documentation
- Updated `LINUX_TODO.md` with accurate status for all items
- Updated `ROADMAP_LINUX_COMPLETE.md` with current progress
- Added HTTP/2 feasibility assessment (estimated 6-10 weeks effort)

## Previous Work (Pre-Changelog)

### Implemented Features
- `oisp-sensor check` - System compatibility validation
- `oisp-sensor diagnose --pid <PID>` - Process-specific SSL diagnostics
- `oisp-sensor ssl-info` - System SSL library information
- `oisp-sensor daemon start/stop/status/logs` - Background service management
- Spec bundle integration for provider detection
- Systemd unit file at `packaging/systemd/oisp-sensor.service`

### Cookbook Examples (Verified in Nightly CI)
- python/01-openai-simple
- python/02-litellm
- python/03-langchain-agent
- python/04-fastapi-service
- node/01-openai-simple
- self-hosted/n8n
- kubernetes/daemonset

### Known Limitations
- HTTP/2 not supported (requires HPACK + multiplexing implementation)
- gRPC traffic not captured (uses HTTP/2)
- Unsupported TLS libraries: Go crypto/tls, rustls, BoringSSL, GnuTLS, NSS
- NVM Node.js uses statically linked OpenSSL (needs binary_paths config)
