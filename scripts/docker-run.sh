#!/bin/bash
# OISP Sensor Docker Run Script
#
# Usage:
#   ./scripts/docker-run.sh              # Run in demo mode
#   ./scripts/docker-run.sh record       # Run with eBPF capture (Linux only)
#   ./scripts/docker-run.sh record --tui # Run with TUI (requires -it)
#   ./scripts/docker-run.sh record --web # Run with web UI

set -e

IMAGE_NAME="oisp-sensor:latest"
MODE="${1:-demo}"
shift || true

# Detect if we need interactive mode
INTERACTIVE=""
if [[ "$*" == *"--tui"* ]]; then
    INTERACTIVE="-it"
fi

case "$MODE" in
    demo)
        echo "Starting OISP Sensor in demo mode..."
        echo "Web UI available at: http://localhost:7777"
        docker run --rm \
            -p 7777:7777 \
            -v oisp-data:/var/lib/oisp \
            $INTERACTIVE \
            "$IMAGE_NAME" \
            demo --output /var/lib/oisp/events.jsonl --port 7777 "$@"
        ;;
    
    record)
        echo "Starting OISP Sensor with eBPF capture..."
        echo "Requires: Linux host, privileged mode"
        echo ""
        
        # Check if running on Linux
        if [[ "$(uname)" != "Linux" ]]; then
            echo "Warning: eBPF capture only works on Linux hosts."
            echo "Use 'demo' mode on macOS/Windows."
            exit 1
        fi
        
        # Detect libssl path
        LIBSSL_PATH=""
        for path in /lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/libssl.so.3 /lib/libssl.so.3 /usr/lib/libssl.so; do
            if [ -f "$path" ]; then
                LIBSSL_PATH="$path"
                break
            fi
        done
        
        if [ -z "$LIBSSL_PATH" ]; then
            echo "Warning: Could not find libssl.so on host"
        else
            echo "Found libssl at: $LIBSSL_PATH"
        fi
        
        docker run --rm \
            --privileged \
            --pid=host \
            --network=host \
            -v /sys/kernel/debug:/sys/kernel/debug:ro \
            -v /sys/fs/bpf:/sys/fs/bpf \
            -v /sys:/sys:ro \
            -v /proc:/proc:ro \
            -v /usr/lib:/usr/lib:ro \
            -v /lib:/lib:ro \
            -v /lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu:ro \
            -v oisp-data:/var/lib/oisp \
            -e RUST_LOG=info \
            $INTERACTIVE \
            "$IMAGE_NAME" \
            record --output /var/lib/oisp/events.jsonl "$@"
        ;;
    
    shell)
        echo "Starting interactive shell..."
        docker run --rm -it \
            --privileged \
            --pid=host \
            --network=host \
            -v /sys/kernel/debug:/sys/kernel/debug:ro \
            -v /sys/fs/bpf:/sys/fs/bpf \
            -v /sys:/sys:ro \
            -v /proc:/proc:ro \
            -v /usr/lib:/usr/lib:ro \
            -v /lib:/lib:ro \
            -v /lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu:ro \
            -v oisp-data:/var/lib/oisp \
            --entrypoint /bin/bash \
            "$IMAGE_NAME"
        ;;
    
    *)
        echo "Unknown mode: $MODE"
        echo ""
        echo "Usage: $0 [MODE] [OPTIONS]"
        echo ""
        echo "Modes:"
        echo "  demo     Run in demo mode with synthetic events (default)"
        echo "  record   Run with eBPF capture (Linux only, requires privileged)"
        echo "  shell    Start interactive shell in container"
        echo ""
        echo "Examples:"
        echo "  $0                     # Demo mode"
        echo "  $0 demo --interval 500 # Demo with faster events"
        echo "  $0 record --web        # eBPF capture with web UI"
        echo "  $0 record --tui        # eBPF capture with terminal UI"
        exit 1
        ;;
esac

