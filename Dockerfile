# OISP Sensor Docker Image
#
# Multi-stage build for eBPF-enabled OISP sensor:
# 1. Stage 1: Build sslsniff (libbpf-based C binary)
# 2. Stage 2: Build React frontend (Node.js)
# 3. Stage 3: Build sensor binary with embedded sslsniff (Rust)
# 4. Stage 4: Runtime image (minimal Debian)
#
# The result is a SINGLE binary with sslsniff embedded.
#
# Prerequisites:
#   Clone with submodules: git clone --recurse-submodules <repo>
#   Or init after clone: git submodule update --init --recursive
#
# Submodules (pinned to specific commits):
#   - bpftool: https://github.com/libbpf/bpftool @ 5b402ff
#   - libbpf:  https://github.com/libbpf/libbpf  @ 7a6e6b4
#
# Build:
#   docker build -t oisp-sensor .
#
# Run (requires Linux host with kernel 5.8+):
#   docker run --privileged --pid=host --network=host \
#     -v /sys/kernel/debug:/sys/kernel/debug:rw \
#     -v /sys/fs/bpf:/sys/fs/bpf \
#     oisp-sensor record
#
# For demo mode (works anywhere):
#   docker run -p 7777:7777 oisp-sensor demo

# =============================================================================
# Stage 1: Build sslsniff (libbpf-based C binary)
# =============================================================================
FROM debian:bookworm-slim AS libbpf-builder

# Install build dependencies for libbpf and sslsniff
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    clang \
    llvm \
    libelf-dev \
    zlib1g-dev \
    make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy bpftool submodule (includes libbpf as nested submodule)
# - bpftool: https://github.com/libbpf/bpftool @ 5b402ff
# - libbpf (nested): https://github.com/libbpf/libbpf @ 7a6e6b4
COPY bpftool ./bpftool

# Copy our sslsniff source and vmlinux headers
COPY bpf ./bpf
COPY vmlinux ./vmlinux

# Build bpftool first (this also builds libbpf)
RUN cd bpftool/src && make -j$(nproc)

# Build sslsniff with paths pointing to bpftool's nested libbpf
WORKDIR /build/bpf
RUN LIBBPF_SRC=/build/bpftool/libbpf/src \
    BPFTOOL_SRC=/build/bpftool/src \
    VMLINUX_DIR=/build/vmlinux \
    make clean && \
    LIBBPF_SRC=/build/bpftool/libbpf/src \
    BPFTOOL_SRC=/build/bpftool/src \
    VMLINUX_DIR=/build/vmlinux \
    make sslsniff && \
    cp sslsniff /usr/local/bin/ && \
    echo "sslsniff built: $(ls -la /usr/local/bin/sslsniff)"

# =============================================================================
# Stage 2: Frontend Builder - Build the React frontend
# =============================================================================
FROM node:20-slim AS frontend-builder

WORKDIR /build/frontend

# Copy package files and install dependencies
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

# Copy frontend source and build
COPY frontend/ ./
RUN npm run build

# =============================================================================
# Stage 3: Rust Builder - Build sensor with embedded sslsniff
# =============================================================================
FROM rust:latest AS userspace-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Copy sslsniff from libbpf-builder (will be embedded via build.rs)
COPY --from=libbpf-builder /usr/local/bin/sslsniff /usr/local/bin/sslsniff

WORKDIR /build

# Copy Cargo workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Copy the built frontend assets from frontend-builder
COPY --from=frontend-builder /build/frontend/out ./frontend/out

# Build the sensor binary with cache mounts and lld linker
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    RUSTFLAGS="-C link-arg=-fuse-ld=lld" \
    cargo build --release --package oisp-sensor && \
    cp target/release/oisp-sensor /usr/local/bin/oisp-sensor && \
    echo "Sensor built: $(ls -la /usr/local/bin/oisp-sensor)"

# =============================================================================
# Stage 4: Runtime - Minimal production image
# =============================================================================
# Use trixie (Debian 13) to match the glibc version from rust:latest
FROM debian:trixie-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libelf1 \
    zlib1g \
    # For SSL interception
    openssl \
    # For debugging (optional)
    procps \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash -u 1000 oisp

# Copy the SINGLE binary (sslsniff is embedded!)
COPY --from=userspace-builder /usr/local/bin/oisp-sensor /usr/local/bin/

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

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7777/api/health 2>/dev/null || exit 1

# Default entrypoint
ENTRYPOINT ["oisp-sensor"]

# Default command: demo mode (works without privileges)
# For eBPF capture, run with: docker run --privileged ... oisp-sensor record
CMD ["demo", "--output", "/var/lib/oisp/events.jsonl", "--port", "7777"]
