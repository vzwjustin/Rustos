#!/bin/bash

# RustOS Docker Script - Optimized for macOS
# This script provides an easy interface for Docker development on macOS

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
DOCKER_COMPOSE_FILE="docker-compose.macos.yml"
IMAGE_NAME="rustos:macos-latest"

# Print functions
print_header() {
    echo -e "${PURPLE}================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}================================${NC}"
}

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites specific to macOS
check_prerequisites() {
    print_header "Checking macOS Prerequisites"

    local issues=0

    # Check Docker
    if command_exists docker; then
        print_success "Docker found: $(docker --version | cut -d',' -f1)"

        # Check if Docker is running
        if ! docker info >/dev/null 2>&1; then
            print_error "Docker daemon not running. Please start Docker Desktop."
            issues=$((issues + 1))
        else
            print_success "Docker daemon is running"
        fi
    else
        print_error "Docker not found. Install Docker Desktop for macOS."
        print_status "Download from: https://www.docker.com/products/docker-desktop/"
        issues=$((issues + 1))
    fi

    # Check Docker Compose
    if command_exists docker-compose || docker compose version >/dev/null 2>&1; then
        print_success "Docker Compose available"
    else
        print_warning "Docker Compose not found. Using docker run fallback."
    fi

    # Check available disk space (Docker needs space)
    local available_gb=$(df -h . | tail -1 | awk '{print $4}' | sed 's/Gi\?//')
    if [[ "$available_gb" =~ ^[0-9]+$ ]] && [ "$available_gb" -gt 5 ]; then
        print_success "Sufficient disk space: ${available_gb}GB available"
    else
        print_warning "Low disk space detected. Docker builds need 5GB+ free space."
    fi

    # Check if we're in the right directory
    if [ -f "Cargo.toml" ] && [ -f "Dockerfile.macos" ]; then
        print_success "In RustOS project directory with macOS Docker files"
    else
        print_error "Missing files. Ensure you're in RustOS root with Dockerfile.macos"
        issues=$((issues + 1))
    fi

    # Check CPU architecture
    local arch=$(uname -m)
    if [ "$arch" = "arm64" ]; then
        print_warning "Apple Silicon (M1/M2) detected. Using x86_64 emulation."
        print_status "This may be slower but ensures compatibility."
    else
        print_success "Intel Mac detected. Native x86_64 execution."
    fi

    # Check Docker Desktop memory allocation
    local docker_memory=$(docker system info --format '{{.MemTotal}}' 2>/dev/null || echo "0")
    if [ "$docker_memory" -gt 3000000000 ]; then  # 3GB in bytes
        print_success "Docker has sufficient memory allocated"
    else
        print_warning "Docker may need more memory. Recommend 4GB+ in Docker Desktop settings."
    fi

    if [ $issues -eq 0 ]; then
        print_success "All prerequisites satisfied!"
        return 0
    else
        print_error "$issues issue(s) found. Please fix before continuing."
        return 1
    fi
}

# Setup cache directories for better performance
setup_cache_dirs() {
    print_status "Setting up cache directories..."

    mkdir -p /tmp/rustos-cargo-cache
    mkdir -p /tmp/rustos-cargo-git
    mkdir -p /tmp/rustos-build-cache

    print_success "Cache directories created in /tmp/"
}

# Build Docker image
build_image() {
    print_header "Building RustOS Docker Image for macOS"

    setup_cache_dirs

    print_status "Building optimized image for macOS..."
    print_warning "This may take 10-15 minutes on first build"

    # Build with specific platform for consistency
    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" build rustos-dev
    else
        docker build -f Dockerfile.macos -t "$IMAGE_NAME" --platform linux/amd64 .
    fi

    if [ $? -eq 0 ]; then
        print_success "Docker image built successfully!"

        # Show image info
        local image_size=$(docker images "$IMAGE_NAME" --format "{{.Size}}" | head -1)
        print_status "Image size: $image_size"
    else
        print_error "Image build failed!"
        return 1
    fi
}

# Quick development setup
quick_dev() {
    print_header "Quick Development Setup"

    print_status "Running complete development pipeline..."
    print_status "This will: check environment → build kernel → run tests → create bootimage"

    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" --profile pipeline up rustos-pipeline
    else
        docker run --rm -it \
            --platform linux/amd64 \
            -v "$(pwd):/home/rustdev/rustos:delegated" \
            -v "/tmp/rustos-cargo-cache:/home/rustdev/.cargo/registry" \
            -v "/tmp/rustos-build-cache:/home/rustdev/rustos/target" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            "$IMAGE_NAME" bash -c "
                echo '🚀 RustOS Quick Development'
                echo '=========================='
                check-env && build-kernel && test-kernel && create-bootimage
                echo '✅ Development setup complete!'
            "
    fi
}

# Interactive development shell
dev_shell() {
    print_header "Starting Development Shell"

    print_status "Starting interactive development environment..."
    print_warning "Inside the container, use: build-kernel, create-bootimage, test-kernel, run-qemu"

    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" --profile shell run --rm rustos-shell
    else
        docker run --rm -it \
            --platform linux/amd64 \
            -v "$(pwd):/home/rustdev/rustos:delegated" \
            -v "/tmp/rustos-cargo-cache:/home/rustdev/.cargo/registry" \
            -v "/tmp/rustos-build-cache:/home/rustdev/rustos/target" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            -e PS1='\[\033[01;32m\][RustOS]\[\033[00m\] \[\033[01;34m\]\w\[\033[00m\] \$ ' \
            --workdir /home/rustdev/rustos \
            "$IMAGE_NAME" /bin/bash
    fi
}

# Build kernel only
build_kernel() {
    print_header "Building RustOS Kernel"

    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" --profile build run --rm rustos-build
    else
        docker run --rm \
            --platform linux/amd64 \
            -v "$(pwd):/home/rustdev/rustos:ro" \
            -v "/tmp/rustos-build-cache:/home/rustdev/rustos/target" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            "$IMAGE_NAME" build-kernel
    fi
}

# Run tests
run_tests() {
    print_header "Running RustOS Tests"

    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" --profile test run --rm rustos-test
    else
        docker run --rm \
            --platform linux/amd64 \
            -v "$(pwd):/home/rustdev/rustos:delegated" \
            -v "/tmp/rustos-cargo-cache:/home/rustdev/.cargo/registry" \
            -v "/tmp/rustos-build-cache:/home/rustdev/rustos/target" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            "$IMAGE_NAME" test-kernel
    fi
}

# Run QEMU
run_qemu() {
    print_header "Running RustOS in QEMU"

    print_status "Starting RustOS in QEMU (headless mode)..."
    print_warning "Press Ctrl+C to stop QEMU"

    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" --profile qemu run --rm rustos-qemu
    else
        docker run --rm -it \
            --platform linux/amd64 \
            -v "$(pwd):/home/rustdev/rustos:delegated" \
            -v "/tmp/rustos-build-cache:/home/rustdev/rustos/target" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            "$IMAGE_NAME" bash -c "
                # Ensure bootimage exists
                if [ ! -f \$(find target -name 'bootimage-*.bin' | head -1) ]; then
                    echo 'Creating bootimage first...'
                    build-kernel && create-bootimage
                fi
                run-qemu
            "
    fi
}

# Clean up Docker resources
cleanup() {
    print_header "Cleaning Up Docker Resources"

    print_status "Stopping containers..."
    if command_exists docker-compose; then
        docker-compose -f "$DOCKER_COMPOSE_FILE" down --volumes 2>/dev/null || true
    fi

    print_status "Removing old containers..."
    docker ps -a --filter "ancestor=$IMAGE_NAME" --format "{{.ID}}" | xargs -r docker rm -f 2>/dev/null || true

    print_status "Removing image..."
    docker rmi "$IMAGE_NAME" 2>/dev/null || true

    print_status "Removing cache directories..."
    rm -rf /tmp/rustos-cargo-cache /tmp/rustos-cargo-git /tmp/rustos-build-cache 2>/dev/null || true

    print_success "Cleanup completed!"
}

# Show help
show_help() {
    cat << EOF
RustOS Docker Script for macOS

USAGE:
    $0 [COMMAND]

COMMANDS:
    check       Check prerequisites and setup
    build       Build Docker image for macOS
    dev         Quick development setup (build + test + bootimage)
    shell       Start interactive development shell
    kernel      Build kernel only
    test        Run kernel tests
    qemu        Run RustOS in QEMU
    clean       Clean up Docker resources
    help        Show this help

EXAMPLES:
    $0 check    # Check if everything is set up correctly
    $0 build    # Build the Docker image
    $0 dev      # Quick development pipeline
    $0 shell    # Interactive development
    $0 qemu     # Test kernel in QEMU

TROUBLESHOOTING:
    - Ensure Docker Desktop is running
    - Allocate 4GB+ memory to Docker in settings
    - For Apple Silicon Macs, x86_64 emulation is used
    - Build cache is stored in /tmp/ for performance

For more help, see: docs/BUILD_GUIDE.md

EOF
}

# Main function
main() {
    case "${1:-help}" in
        check)
            check_prerequisites
            ;;
        build)
            check_prerequisites || exit 1
            build_image
            ;;
        dev)
            check_prerequisites || exit 1
            if ! docker images "$IMAGE_NAME" --format "{{.Repository}}" | grep -q "rustos"; then
                print_status "Image not found. Building first..."
                build_image
            fi
            quick_dev
            ;;
        shell)
            check_prerequisites || exit 1
            if ! docker images "$IMAGE_NAME" --format "{{.Repository}}" | grep -q "rustos"; then
                print_status "Image not found. Building first..."
                build_image
            fi
            dev_shell
            ;;
        kernel)
            check_prerequisites || exit 1
            build_kernel
            ;;
        test)
            check_prerequisites || exit 1
            run_tests
            ;;
        qemu)
            check_prerequisites || exit 1
            run_qemu
            ;;
        clean)
            cleanup
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Trap Ctrl+C gracefully
trap 'print_warning "Interrupted by user"; exit 130' INT

# Run main function
main "$@"