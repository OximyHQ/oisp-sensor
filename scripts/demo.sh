#!/bin/bash
# Demo script for OISP Sensor
# 
# This runs the sensor in demo mode, generating fake AI events
# to test the pipeline and UI without needing eBPF or root.
#
# Usage: ./scripts/demo.sh [OPTIONS]
#
# Options are passed directly to `oisp-sensor demo`

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check if binary exists
BINARY="$PROJECT_ROOT/target/release/oisp-sensor"
if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Building..."
    cd "$PROJECT_ROOT"
    cargo build --release
fi

echo ""
echo "  Starting OISP Sensor in DEMO mode"
echo ""
echo "  This generates test AI events without eBPF."
echo "  Perfect for UI development and testing."
echo ""
echo "  Web UI will be at: http://127.0.0.1:7777"
echo ""

# Run with provided options or defaults
"$BINARY" demo "$@"

