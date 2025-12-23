# OISP Sensor Docker Image
#
# Build:
#   docker build -t oisp-sensor .
#
# Run:
#   docker run --privileged -v /sys/kernel/debug:/sys/kernel/debug oisp-sensor record

# Builder stage
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    llvm \
    libbpf-dev \
    linux-headers-generic \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libbpf1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (sensor can be run with --user if not needing root)
RUN useradd -m -s /bin/bash oisp

# Copy binary
COPY --from=builder /build/target/release/oisp-sensor /usr/local/bin/

# Create directories
RUN mkdir -p /var/lib/oisp /var/log/oisp && \
    chown -R oisp:oisp /var/lib/oisp /var/log/oisp

# Default output location
VOLUME /var/lib/oisp

# Expose web UI port
EXPOSE 7777

# Health check
HEALTHCHECK --interval=30s --timeout=3s \
    CMD oisp-sensor status || exit 1

# Default command
ENTRYPOINT ["oisp-sensor"]
CMD ["record", "--output", "/var/lib/oisp/events.jsonl"]

