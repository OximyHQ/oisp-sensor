# OISP Sensor Docker Image
#
# Multi-stage build for eBPF-enabled OISP sensor:
# 1. Stage 1: Build eBPF bytecode (requires nightly Rust + bpf-linker)
# 2. Stage 2: Build userspace binary (stable Rust)
# 3. Stage 3: Runtime image (minimal Debian)
#
# Build:
#   docker build -t oisp-sensor .
#
# Run (requires Linux host with kernel 5.8+):
#   docker run --privileged --pid=host --network=host \
#     -v /sys/kernel/debug:/sys/kernel/debug:ro \
#     -v /sys/fs/bpf:/sys/fs/bpf \
#     oisp-sensor record
#
# For demo mode (works anywhere):
#   docker run -p 7777:7777 oisp-sensor demo

ARG RUST_VERSION=1.83
ARG DEBIAN_VERSION=bookworm

# =============================================================================
# Stage 1: eBPF Builder - Build the eBPF bytecode
# =============================================================================
# Use latest Rust for eBPF builder since bpf-linker requires newer rustc (1.86+)
FROM rust:latest AS ebpf-builder

# Install eBPF build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    clang \
    llvm \
    libelf-dev \
    linux-headers-generic \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install nightly toolchain (required for eBPF compilation)
RUN rustup toolchain install nightly --component rust-src

# Install bpf-linker (required for linking eBPF programs)
# Use --locked to ensure reproducible builds
RUN cargo install bpf-linker --locked

WORKDIR /build/ebpf

# Copy eBPF workspace files
COPY ebpf/Cargo.toml ebpf/Cargo.lock ./
COPY ebpf/rustfmt.toml ./
COPY ebpf/oisp-ebpf-capture ./oisp-ebpf-capture
COPY ebpf/oisp-ebpf-capture-common ./oisp-ebpf-capture-common
COPY ebpf/oisp-ebpf-capture-ebpf ./oisp-ebpf-capture-ebpf

# Build eBPF programs (this produces the .o bytecode files)
# The build.rs in oisp-ebpf-capture handles eBPF compilation via aya-build
RUN cargo build --release --package oisp-ebpf-capture

# The eBPF bytecode is embedded in the binary, but we also copy it for reference
# The compiled eBPF object is at: target/bpfel-unknown-none/release/oisp-ebpf-capture-ebpf
RUN mkdir -p /build/ebpf-bytecode && \
    find target -name "*.o" -path "*/bpfel-*/*" -exec cp {} /build/ebpf-bytecode/ \; || true

# =============================================================================
# Stage 2: Userspace Builder - Build the main sensor binary
# =============================================================================
# Use latest Rust since Aya from git requires edition 2024 (Rust 1.86+)
FROM rust:latest AS userspace-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    llvm \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the sensor binary in release mode
RUN cargo build --release --package oisp-sensor

# =============================================================================
# Stage 3: Runtime - Minimal production image
# =============================================================================
# Use trixie (Debian 13) to match the glibc version from rust:latest
FROM debian:trixie-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    # For SSL interception (libssl.so)
    openssl \
    # For debugging (optional, can be removed for smaller image)
    procps \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (sensor can be run with --user if not needing privileged mode)
RUN useradd -m -s /bin/bash -u 1000 oisp

# Copy binaries from builder stages
COPY --from=ebpf-builder /build/oisp-ebpf-capture/target/release/oisp-ebpf-capture /usr/local/bin/
COPY --from=userspace-builder /build/target/release/oisp-sensor /usr/local/bin/

# Copy eBPF bytecode (optional, for debugging or alternative loading)
COPY --from=ebpf-builder /build/ebpf-bytecode /usr/share/oisp/ebpf/

# Create directories for data and logs
RUN mkdir -p /var/lib/oisp /var/log/oisp /etc/oisp && \
    chown -R oisp:oisp /var/lib/oisp /var/log/oisp

# Copy example config (if exists)
COPY config.example.toml /etc/oisp/config.example.toml

# Volume for persistent data
VOLUME /var/lib/oisp

# Expose web UI port
EXPOSE 7777

# Labels for container metadata
LABEL org.opencontainers.image.title="OISP Sensor" \
      org.opencontainers.image.description="Open Inference Security Protocol sensor for AI observability" \
      org.opencontainers.image.vendor="Oximy" \
      org.opencontainers.image.source="https://github.com/oximyHQ/oisp-sensor" \
      org.opencontainers.image.licenses="Apache-2.0"

# Health check - use demo mode check since record mode requires privileges
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7777/api/health 2>/dev/null || exit 1

# Default entrypoint
ENTRYPOINT ["oisp-sensor"]

# Default command: demo mode (works without privileges)
# For eBPF capture, run with: docker run --privileged ... oisp-sensor record
CMD ["demo", "--output", "/var/lib/oisp/events.jsonl", "--port", "7777"]
