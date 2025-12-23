#!/bin/bash
# Test OISP Sensor on Linux via Docker
#
# This script builds and runs the sensor in a Linux container.
# Note: Full eBPF capture requires running on a Linux host.
# On macOS/Windows, only the demo mode will work fully.
#
# Usage:
#   ./scripts/test-linux.sh          # Run full capture (Linux host only)
#   ./scripts/test-linux.sh demo     # Run demo mode (works anywhere)
#   ./scripts/test-linux.sh build    # Just build the image

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

MODE="${1:-full}"

case "$MODE" in
    build)
        echo "Building OISP Sensor Docker image..."
        docker build -t oisp-sensor:latest .
        echo ""
        echo "Build complete! Image: oisp-sensor:latest"
        ;;
    
    demo)
        echo "Running OISP Sensor in demo mode..."
        echo ""
        docker build -t oisp-sensor:latest .
        docker run --rm -it \
            -p 7777:7777 \
            oisp-sensor:latest \
            demo --interval 2000 --port 7777
        ;;
    
    full|record)
        echo "Running OISP Sensor with full eBPF capture..."
        echo ""
        echo "NOTE: This requires a Linux host. On macOS/Windows, use 'demo' mode."
        echo ""
        docker build -t oisp-sensor:latest .
        docker run --rm -it \
            --privileged \
            --pid=host \
            --network=host \
            -v /sys:/sys:ro \
            -v /usr:/usr:ro \
            -v /lib:/lib:ro \
            oisp-sensor:latest \
            record --port 7777
        ;;
    
    status)
        echo "Checking OISP Sensor status..."
        docker build -t oisp-sensor:latest .
        docker run --rm \
            --privileged \
            -v /sys:/sys:ro \
            oisp-sensor:latest \
            status
        ;;
    
    test)
        echo "Running OISP Sensor self-test..."
        docker build -t oisp-sensor:latest .
        docker run --rm oisp-sensor:latest test
        ;;
    
    shell)
        echo "Opening shell in OISP Sensor container..."
        docker build -t oisp-sensor:latest .
        docker run --rm -it \
            --privileged \
            --entrypoint /bin/bash \
            oisp-sensor:latest
        ;;
    
    *)
        echo "Usage: $0 [MODE]"
        echo ""
        echo "Modes:"
        echo "  demo    - Run in demo mode (works anywhere, no eBPF)"
        echo "  full    - Run with full eBPF capture (Linux host only)"
        echo "  build   - Just build the Docker image"
        echo "  status  - Check sensor status and capabilities"
        echo "  test    - Run self-test"
        echo "  shell   - Open a shell in the container"
        exit 1
        ;;
esac

