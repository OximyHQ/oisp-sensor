#!/bin/bash
# OISP Sensor Docker Build Script
#
# Usage:
#   ./scripts/docker-build.sh              # Build for current platform
#   ./scripts/docker-build.sh --multiarch  # Build for amd64 and arm64
#   ./scripts/docker-build.sh --push       # Build and push to registry

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Default values
IMAGE_NAME="oisp-sensor"
IMAGE_TAG="latest"
REGISTRY="ghcr.io/oximyhq"
MULTIARCH=false
PUSH=false
NO_CACHE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --multiarch)
            MULTIARCH=true
            shift
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --tag)
            IMAGE_TAG="$2"
            shift 2
            ;;
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        -h|--help)
            echo "OISP Sensor Docker Build Script"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --multiarch    Build for both amd64 and arm64"
            echo "  --push         Push images to registry"
            echo "  --no-cache     Build without Docker cache"
            echo "  --tag TAG      Set image tag (default: latest)"
            echo "  --registry REG Set registry (default: ghcr.io/oximyhq)"
            echo "  -h, --help     Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

FULL_IMAGE="${REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
CACHE_ARG=""
if [ "$NO_CACHE" = true ]; then
    CACHE_ARG="--no-cache"
fi

echo "Building OISP Sensor Docker image..."
echo "  Image: ${FULL_IMAGE}"
echo "  Multiarch: ${MULTIARCH}"
echo "  Push: ${PUSH}"
echo ""

if [ "$MULTIARCH" = true ]; then
    # Ensure buildx is available
    if ! docker buildx version > /dev/null 2>&1; then
        echo "Error: docker buildx is required for multi-arch builds"
        echo "Install with: docker buildx install"
        exit 1
    fi
    
    # Create buildx builder if not exists
    if ! docker buildx inspect oisp-builder > /dev/null 2>&1; then
        echo "Creating buildx builder..."
        docker buildx create --name oisp-builder --use
        docker buildx inspect --bootstrap
    else
        docker buildx use oisp-builder
    fi
    
    PLATFORMS="linux/amd64,linux/arm64"
    echo "Building for platforms: ${PLATFORMS}"
    
    if [ "$PUSH" = true ]; then
        docker buildx build \
            -f docker/Dockerfile.multiarch \
            --platform "${PLATFORMS}" \
            -t "${FULL_IMAGE}" \
            ${CACHE_ARG} \
            --push \
            .
    else
        # Build but don't push (load locally)
        # Note: Multi-arch builds can only be loaded if building for single platform
        echo "Building multi-arch without push (use --push to push to registry)"
        docker buildx build \
            -f docker/Dockerfile.multiarch \
            --platform "linux/amd64" \
            -t "${IMAGE_NAME}:${IMAGE_TAG}-amd64" \
            ${CACHE_ARG} \
            --load \
            .
        docker buildx build \
            -f docker/Dockerfile.multiarch \
            --platform "linux/arm64" \
            -t "${IMAGE_NAME}:${IMAGE_TAG}-arm64" \
            ${CACHE_ARG} \
            --load \
            .
    fi
else
    # Single platform build
    docker build \
        -f Dockerfile \
        -t "${IMAGE_NAME}:${IMAGE_TAG}" \
        ${CACHE_ARG} \
        .
    
    if [ "$PUSH" = true ]; then
        docker tag "${IMAGE_NAME}:${IMAGE_TAG}" "${FULL_IMAGE}"
        docker push "${FULL_IMAGE}"
    fi
fi

echo ""
echo "Build complete!"

if [ "$PUSH" = false ]; then
    echo ""
    echo "Run with:"
    echo "  Demo mode:  docker run -p 7777:7777 ${IMAGE_NAME}:${IMAGE_TAG}"
    echo "  eBPF mode:  docker run --privileged --pid=host --network=host \\"
    echo "                -v /sys/kernel/debug:/sys/kernel/debug:ro \\"
    echo "                -v /sys/fs/bpf:/sys/fs/bpf \\"
    echo "                ${IMAGE_NAME}:${IMAGE_TAG} record"
fi

