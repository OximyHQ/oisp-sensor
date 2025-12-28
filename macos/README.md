# OISP for macOS

OISP (Observability for Intelligent Systems Platform) captures AI API traffic from any application on your Mac, providing real-time visibility into LLM usage, costs, and performance.

## Architecture

OISP on macOS uses a Network Extension (System Extension) to transparently intercept HTTPS traffic to AI API endpoints. The architecture consists of:

```
┌─────────────────────────────────────────────────────────────┐
│                     OISP Menu Bar App                        │
│  (SwiftUI app for status, settings, and extension control)  │
└─────────────────────────────────────────────────────────────┘
                              │
                     Unix Domain Socket
                              │
┌─────────────────────────────────────────────────────────────┐
│                    oisp-sensor (Rust)                        │
│  Receives events, decodes HTTP, emits to dashboard/exports  │
└─────────────────────────────────────────────────────────────┘
                              ▲
                     Unix Domain Socket
                              │
┌─────────────────────────────────────────────────────────────┐
│              OISP Network Extension                          │
│  NETransparentProxyProvider + TLS MITM + Event Emission     │
└─────────────────────────────────────────────────────────────┘
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
| Local (Ollama) | localhost, 127.0.0.1 |

## Requirements

- macOS 13.0 (Ventura) or later
- Apple Silicon (M1/M2/M3) or Intel Mac
- Admin access (for extension approval)

## Installation

See [INSTALLATION.md](INSTALLATION.md) for detailed installation instructions.

### Quick Start

1. Download OISP.dmg from the releases page
2. Open the DMG and drag OISP to Applications
3. Open OISP from Applications
4. Follow the setup wizard:
   - Approve the Network Extension in System Settings
   - Trust the OISP CA certificate

## Building from Source

### Prerequisites

- Xcode 15 or later
- xcodegen (`brew install xcodegen`)
- Rust toolchain (`rustup`)
- Apple Developer ID (for distribution)

### Build Steps

```bash
# Navigate to macOS directory
cd oisp-sensor/macos

# Generate Xcode project
xcodegen generate

# Build with Swift Package Manager (for library targets)
swift build

# Build with Xcode (for full app with extension)
xcodebuild -project OISP.xcodeproj -scheme OISP -configuration Debug build

# Or use the release build script
./Scripts/build-release.sh --team YOUR_TEAM_ID
```

### Running Locally (Development)

For development without a Developer ID:

```bash
# Build in debug mode
swift build

# Run tests
swift test

# The full app requires signing, so open in Xcode:
open OISP.xcodeproj
```

## How It Works

### TLS Interception

OISP uses a man-in-the-middle (MITM) approach to decrypt HTTPS traffic:

1. **Certificate Authority**: OISP generates a local CA certificate on first run
2. **Certificate Trust**: The CA must be added to the system keychain and trusted
3. **Per-host Certificates**: For each AI endpoint, OISP dynamically generates certificates signed by the CA
4. **Transparent Proxy**: The Network Extension intercepts connections and performs TLS termination

### Security Considerations

- The CA private key never leaves your Mac
- Traffic is only intercepted for known AI endpoints (whitelist approach)
- All processing happens locally - no data is sent to external servers
- The extension uses minimal privileges required for operation

## Configuration

Configuration is stored in `~/Library/Application Support/OISP/`:

```
~/.config/oisp/
├── config.toml          # General configuration
├── ca.key               # CA private key (protected)
├── ca.pem               # CA certificate
└── custom-endpoints.txt # Additional endpoints to intercept
```

### Adding Custom Endpoints

Edit `custom-endpoints.txt` (one per line):

```
api.my-custom-llm.com
internal-llm.company.internal
```

## Troubleshooting

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues and solutions.

### Quick Fixes

**Extension not loading:**
```bash
systemextensionsctl list
# Look for com.oisp.networkextension
```

**Reset extension:**
```bash
# Unload extension
sudo systemextensionsctl uninstall com.oisp.networkextension

# Re-open OISP to reinstall
```

**Check logs:**
```bash
log stream --predicate 'subsystem == "com.oisp"' --level debug
```

## Contributing

### The Apple Developer Requirement

macOS System Extensions (like the OISP Network Extension) **require code signing** with an Apple Developer ID ($99/year). This creates a challenge for open source contributions.

### What Contributors Can Do

**Without Apple Developer ID:**
- ✅ Modify and test Rust code (`cargo build && cargo test`)
- ✅ Modify Swift code and verify compilation (`xcodebuild -target OISPCore`)
- ✅ Write unit tests for OISPCore framework
- ✅ Update documentation
- ✅ Submit PRs for review

**Requires Apple Developer ID:**
- ❌ Test the full Network Extension end-to-end
- ❌ Build signed DMG releases
- ❌ Run notarization

### Development Workflow

```bash
# 1. Build and test Rust code (works for everyone)
cargo build --release
cargo test

# 2. Build Swift framework (works for everyone)
cd macos
xcodegen generate
xcodebuild -target OISPCore -configuration Debug build \
    CODE_SIGN_IDENTITY="" CODE_SIGNING_REQUIRED=NO

# 3. Full app testing (requires Developer ID)
# Only maintainers can test the Network Extension
```

### CI/CD

Our CI builds:
- Rust crates on Linux, macOS, Windows
- Swift OISPCore framework (unsigned) on macOS

Signed releases are built manually by maintainers and published to GitHub Releases.

## Privacy

OISP is designed with privacy in mind:

- **Local Only**: All processing happens on your Mac
- **No Telemetry**: OISP does not phone home
- **Selective Interception**: Only AI API traffic is captured
- **User Control**: Pause/resume capture anytime

## License

See the main OISP repository for license information.
