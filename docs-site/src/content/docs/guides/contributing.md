---
title: Contributing
description: Contribute to OISP Sensor development
---


Thank you for your interest in contributing to OISP Sensor! This guide will help you get started.

## Ways to Contribute

- **Report bugs** - Found an issue? Let us know
- **Request features** - Have an idea? Share it
- **Improve documentation** - Fix typos, clarify sections, add examples
- **Submit code** - Bug fixes, new features, performance improvements
- **Write cookbooks** - Share example integrations
- **Test on new platforms** - Help expand platform support

---

## Getting Started

### Prerequisites

- **Rust** - 1.75 or later
- **Linux** - Ubuntu 22.04+ or similar (for development)
- **Docker** - For testing cookbooks
- **Git** - Version control

### Fork and Clone

```bash
# Fork the repo on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/oisp-sensor.git
cd oisp-sensor

# Add upstream remote
git remote add upstream https://github.com/oximyHQ/oisp-sensor.git
```

### Build from Source

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    clang \
    llvm \
    libelf-dev \
    linux-headers-$(uname -r)

# Build
cargo build --release

# Run
sudo ./target/release/oisp-sensor check
```

---

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

**Branch naming:**
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `test/` - Test improvements
- `refactor/` - Code refactoring

### 2. Make Changes

Follow the [code style](#code-style) and [testing](#testing) guidelines.

### 3. Commit

```bash
git add .
git commit -m "feat: add support for new provider"
```

**Commit message format:**

```
<type>: <short description>

[optional body]

[optional footer]
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `test` - Test additions/changes
- `refactor` - Code refactoring
- `perf` - Performance improvements
- `chore` - Build/tooling changes

**Examples:**

```
feat: add Anthropic Claude provider detection

Add support for detecting Anthropic Claude API calls
and parsing response events.

Closes #123
```

```
fix: handle chunked encoding in SSL capture

Fixes issue where chunked responses were not properly
decoded, causing missing ai.response events.

Fixes #456
```

### 4. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

---

## Code Style

### Rust

Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

**Format code:**

```bash
cargo fmt
```

**Lint with Clippy:**

```bash
cargo clippy -- -D warnings
```

**Key conventions:**
- Use meaningful variable names
- Add doc comments for public APIs
- Prefer `Result<T, E>` over panicking
- Use `tracing` for logging, not `println!`

**Example:**

```rust
/// Captures SSL traffic from the specified process.
///
/// # Arguments
///
/// * `pid` - Process ID to monitor
/// * `config` - Capture configuration
///
/// # Errors
///
/// Returns error if eBPF program fails to attach.
pub fn capture_ssl(pid: u32, config: &CaptureConfig) -> Result<()> {
    tracing::info!("Attaching to PID {}", pid);
    // Implementation
    Ok(())
}
```

### Documentation

- Use clear, concise language
- Provide examples
- Use code blocks with syntax highlighting
- Link to related docs

---

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Run integration tests
cargo test --test '*'
```

### Cookbook Validation

```bash
cd oisp-cookbook

# Test a specific cookbook
./test-all.sh python/01-openai-simple

# Test all cookbooks
./test-all.sh
```

### Manual Testing

```bash
# Build and run
cargo build --release
sudo ./target/release/oisp-sensor record --output /tmp/test.jsonl

# In another terminal, trigger AI activity
python3 -c "
import openai
client = openai.OpenAI()
response = client.chat.completions.create(
    model='gpt-4o-mini',
    messages=[{'role': 'user', 'content': 'Test'}]
)
print(response.choices[0].message.content)
"

# Stop sensor (Ctrl+C) and check events
cat /tmp/test.jsonl | jq .
```

---

## Project Structure

```
oisp-sensor/
├── crates/
│   ├── oisp-capture-ebpf/    # eBPF capture (Linux)
│   ├── oisp-capture-macos/   # macOS capture (preview)
│   ├── oisp-capture-windows/ # Windows capture (preview)
│   ├── oisp-decode/          # HTTP/JSON decoding
│   ├── oisp-export/          # Export formats (JSONL, OTLP, Kafka)
│   ├── oisp-redaction/       # PII/sensitive data redaction
│   ├── oisp-cli/             # CLI interface
│   └── oisp-web/             # Web UI
├── docs-site/                # Astro documentation site
├── oisp-cookbook/            # Example integrations (submodule)
├── Cargo.toml                # Workspace manifest
└── README.md
```

---

## Adding a New Provider

To add support for a new AI provider (e.g., Cohere, Mistral):

### 1. Add Provider Definition

**`crates/oisp-decode/src/providers/mod.rs`:**

```rust
pub enum Provider {
    OpenAI,
    Anthropic,
    Cohere,  // Add new provider
}
```

### 2. Add Detection Logic

**`crates/oisp-decode/src/providers/cohere.rs`:**

```rust
use crate::{Provider, Request, Response};

pub fn detect_request(host: &str, path: &str, body: &str) -> Option<Request> {
    if !host.contains("cohere.ai") {
        return None;
    }

    // Parse Cohere request format
    let data = serde_json::from_str(body).ok()?;

    Some(Request {
        provider: Provider::Cohere,
        model: data["model"].as_str()?.to_string(),
        messages: parse_messages(&data),
        // ...
    })
}

pub fn detect_response(body: &str) -> Option<Response> {
    // Parse Cohere response format
    // ...
}
```

### 3. Register Provider

**`crates/oisp-decode/src/lib.rs`:**

```rust
mod providers;
use providers::{openai, anthropic, cohere};

pub fn decode_request(host: &str, path: &str, body: &str) -> Option<Request> {
    openai::detect_request(host, path, body)
        .or_else(|| anthropic::detect_request(host, path, body))
        .or_else(|| cohere::detect_request(host, path, body))  // Add
}
```

### 4. Add Tests

**`crates/oisp-decode/src/providers/cohere.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cohere_request() {
        let body = r#"{"model":"command","message":"Hello"}"#;
        let req = detect_request("api.cohere.ai", "/v1/chat", body);
        assert!(req.is_some());
        assert_eq!(req.unwrap().provider, Provider::Cohere);
    }
}
```

### 5. Add Cookbook Example

Create `oisp-cookbook/python/05-cohere-simple/`:

```
05-cohere-simple/
├── README.md
├── app.py
├── docker-compose.yml
├── validate.sh
└── expected-events.jsonl
```

### 6. Update Documentation

Add to [docs-site/src/content/docs/reference/providers.md](docs-site/src/content/docs/reference/providers.md).

---

## Adding Platform Support

### macOS

**Current status:** Metadata capture only (preview)

**Roadmap:** Full SSL capture via System Extension

**How to contribute:**
1. **System Extension development** - Implement in `crates/oisp-capture-macos/`
2. **Testing** - Test on various macOS versions (12+)
3. **Code signing** - Help with notarization process

### Windows

**Current status:** Metadata capture only (preview)

**Roadmap:** Full capture via ETW (Event Tracing for Windows)

**How to contribute:**
1. **ETW integration** - Implement in `crates/oisp-capture-windows/`
2. **Testing** - Test on Windows 10/11
3. **Installer** - MSI packaging

---

## Documentation Contributions

### Astro Docs Site

Located in `docs-site/`:

```bash
cd docs-site
npm install
npm run dev  # http://localhost:4321
```

**File structure:**

```
docs-site/
├── src/
│   └── content/
│       └── docs/
│           ├── getting-started/
│           ├── platforms/
│           ├── cookbooks/
│           ├── architecture/
│           ├── configuration/
│           ├── reference/
│           └── guides/
└── astro.config.mjs  # Sidebar navigation
```

**Adding a new page:**

1. Create markdown file in appropriate section
2. Add frontmatter:
   ```markdown
   ---
   title: Page Title
   description: Brief description
   ---
   ```
3. Update sidebar in `astro.config.mjs`
4. Preview with `npm run dev`
5. Build with `npm run build`

### Cookbook Documentation

Each cookbook should have:

1. **README.md** - Overview, prerequisites, running instructions
2. **Code** - Working example with comments
3. **docker-compose.yml** - Easy reproducibility
4. **validate.sh** - Automated validation
5. **expected-events.jsonl** - Expected output

---

## Pull Request Guidelines

### Before Submitting

- [ ] Code builds (`cargo build --release`)
- [ ] Tests pass (`cargo test`)
- [ ] Linting passes (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation updated (if needed)
- [ ] Cookbook tested (if applicable)

### PR Description Template

```markdown
## Description

Brief description of changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing

How was this tested?

## Checklist

- [ ] Code builds
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Changelog updated (if needed)

## Related Issues

Closes #123
```

### Review Process

1. **Automated checks** - CI runs tests, linting, builds
2. **Code review** - Maintainer reviews code
3. **Discussion** - Address feedback
4. **Approval** - Maintainer approves
5. **Merge** - Squash and merge to main

---

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- record --output /tmp/test.jsonl
```

**Module-specific logging:**

```bash
# eBPF capture only
RUST_LOG=oisp_capture_ebpf=debug cargo run

# Decode module only
RUST_LOG=oisp_decode=debug cargo run

# Multiple modules
RUST_LOG=oisp_capture_ebpf=debug,oisp_decode=info cargo run
```

### Debug eBPF Programs

```bash
# List loaded BPF programs
sudo bpftool prog list

# Dump BPF maps
sudo bpftool map dump id <map_id>

# View eBPF logs
sudo cat /sys/kernel/debug/tracing/trace_pipe
```

### Profile Performance

```bash
# Build with profiling
cargo build --release --features profiling

# Run with perf
sudo perf record -F 99 -g ./target/release/oisp-sensor record
sudo perf report
```

---

## Release Process

Maintainers only:

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md**
3. **Create tag:**
   ```bash
   git tag -a v0.3.0 -m "Release v0.3.0"
   git push origin v0.3.0
   ```
4. **GitHub Actions** builds binaries, packages, Docker images
5. **Draft release** on GitHub with changelog

---

## Community

### Communication Channels

- **GitHub Issues** - Bug reports, feature requests
- **GitHub Discussions** - General questions, ideas
- **Pull Requests** - Code contributions

### Code of Conduct

Be respectful, inclusive, and constructive. We follow the [Contributor Covenant](https://www.contributor-covenant.org/).

---

## Getting Help

- **Documentation** - https://sensor.oisp.dev
- **Discussions** - https://github.com/oximyHQ/oisp-sensor/discussions
- **Issues** - https://github.com/oximyHQ/oisp-sensor/issues

---

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.

---

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- GitHub contributors page
- Release notes (for significant contributions)

---

Thank you for contributing to OISP Sensor! Your contributions help make AI observability better for everyone.
