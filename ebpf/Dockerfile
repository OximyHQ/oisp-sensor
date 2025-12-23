# oisp-ebpf-capture Docker Image
#
# Standalone eBPF capture tool for development and testing.
#
# Build:
#   docker build -t oisp-ebpf-capture .
#
# Run (requires privileged mode and Linux host):
#   docker run --privileged --pid=host --network=host \
#     -v /sys/kernel/debug:/sys/kernel/debug:ro \
#     -v /sys/fs/bpf:/sys/fs/bpf \
#     oisp-ebpf-capture

ARG RUST_VERSION=nightly
ARG DEBIAN_VERSION=bookworm

# =============================================================================
# Builder Stage
# =============================================================================
FROM rust:latest AS builder

# Install eBPF build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    clang \
    llvm \
    libelf-dev \
    linux-headers-generic \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install nightly toolchain (required for eBPF programs)
RUN rustup toolchain install nightly --component rust-src

# Install bpf-linker with --locked for reproducible builds
RUN cargo install bpf-linker --locked

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY rustfmt.toml ./
COPY oisp-ebpf-capture ./oisp-ebpf-capture
COPY oisp-ebpf-capture-common ./oisp-ebpf-capture-common
COPY oisp-ebpf-capture-ebpf ./oisp-ebpf-capture-ebpf

# Build the eBPF programs and userspace loader
RUN cargo build --release

# Copy test scripts (optional)
COPY tests ./tests
RUN chmod +x tests/*.sh tests/*.py tests/*.js 2>/dev/null || true

# =============================================================================
# Runtime Stage
# =============================================================================
FROM debian:${DEBIAN_VERSION}-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssl \
    # Testing tools
    curl \
    python3 \
    python3-pip \
    nodejs \
    npm \
    jq \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Install Python requests for testing
RUN pip3 install --break-system-packages requests

# Create non-root user
RUN useradd -m -s /bin/bash -u 1000 oisp

# Copy compiled binary
COPY --from=builder /app/target/release/oisp-ebpf-capture /usr/local/bin/

# Copy test scripts
COPY --from=builder /app/tests /opt/oisp/tests

WORKDIR /opt/oisp

# Default: run the capture tool
ENTRYPOINT ["oisp-ebpf-capture"]
CMD ["--help"]
