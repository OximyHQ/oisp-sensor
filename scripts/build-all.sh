#!/bin/bash
# Build script for OISP Sensor with embedded frontend
# This builds both the React frontend and Rust binary

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Building OISP Sensor..."
echo "Project root: $PROJECT_ROOT"

# Build frontend
echo ""
echo "=== Building Frontend ==="
cd "$PROJECT_ROOT/frontend"

if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install --legacy-peer-deps
fi

echo "Building React app..."
npm run build

echo "Frontend built successfully in frontend/out/"

# Build Rust binary
echo ""
echo "=== Building Backend ==="
cd "$PROJECT_ROOT"

echo "Building release binary..."
cargo build --release

echo ""
echo "=== Build Complete ==="
echo "Binary: target/release/oisp-sensor"
echo ""
echo "Run with: ./target/release/oisp-sensor"
echo "Web UI: http://localhost:7777"

