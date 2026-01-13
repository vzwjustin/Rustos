#!/bin/bash

# RustOS Simple Kernel Build Script
# Builds and runs the simplified bootable RustOS kernel

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Configuration
KERNEL_NAME="rustos-simple"
TARGET="x86_64-rustos.json"
CARGO_FILE="Cargo.toml"

print_header() {
    echo -e "${CYAN}================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}================================${NC}"
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

# Install Rust if not present
install_rust() {
    if ! command_exists rustc; then
        print_status "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
        source ~/.cargo/env
        print_success "Rust installed"
    fi

    # Ensure nightly toolchain
    rustup default nightly
    rustup component add rust-src llvm-tools-preview

    # Install bootimage if not present
    if ! command_exists cargo-bootimage; then
        print_status "Installing bootimage..."
        cargo install bootimage
        print_success "Bootimage installed"
    fi
}

# Show help
show_help() {
    cat << EOF
RustOS Simple Kernel Build Script

Usage: $0 [OPTIONS]

OPTIONS:
    -h, --help          Show this help message
    -b, --build         Build the kernel
    -r, --run           Build and run in QEMU
    -c, --clean         Clean build artifacts
    -t, --test          Run kernel tests
    -i, --install       Install build dependencies
    -v, --verbose       Enable verbose output

EXAMPLES:
    $0 --install        # Install dependencies
    $0 --build          # Build kernel
    $0 --run            # Build and run in QEMU
    $0 --clean --build  # Clean build

EOF
}

# Clean build artifacts
clean_build() {
    print_status "Cleaning build artifacts..."
    rm -rf target/
    cargo clean --manifest-path "$CARGO_FILE" 2>/dev/null || true
    print_success "Build artifacts cleaned"
}

# Build the kernel
build_kernel() {
    print_header "Building RustOS Simple Kernel"

    print_status "Using Cargo file: $CARGO_FILE"
    print_status "Target: $TARGET"

    # Verify target specification exists
    if [ ! -f "$TARGET" ]; then
        print_error "Target specification not found: $TARGET"
        exit 1
    fi

    # Build kernel
    print_status "Compiling kernel..."

    if [ "$VERBOSE" = true ]; then
        cargo build --manifest-path "$CARGO_FILE" --target "$TARGET" --verbose
    else
        cargo build --manifest-path "$CARGO_FILE" --target "$TARGET"
    fi

    print_success "Kernel compiled successfully"
}

# Create bootable image
create_bootimage() {
    print_header "Creating Bootable Image"

    print_status "Creating bootable disk image..."

    if [ "$VERBOSE" = true ]; then
        cargo bootimage --manifest-path "$CARGO_FILE" --target "$TARGET" --verbose
    else
        cargo bootimage --manifest-path "$CARGO_FILE" --target "$TARGET"
    fi

    # Find the bootimage
    BOOTIMAGE_PATH="target/$TARGET/debug/bootimage-rustos-simple.bin"

    if [ -f "$BOOTIMAGE_PATH" ]; then
        print_success "Bootimage created: $BOOTIMAGE_PATH"

        # Show image information
        local size=$(ls -lh "$BOOTIMAGE_PATH" | awk '{print $5}')
        print_status "Bootimage size: $size"

        # Calculate checksum if available
        if command_exists sha256sum; then
            local checksum=$(sha256sum "$BOOTIMAGE_PATH" | cut -d' ' -f1)
            print_status "SHA256: $checksum"
        fi
    else
        print_error "Bootimage not found at: $BOOTIMAGE_PATH"
        exit 1
    fi
}

# Run kernel in QEMU
run_qemu() {
    print_header "Running RustOS in QEMU"

    if [ ! -f "$BOOTIMAGE_PATH" ]; then
        print_error "Bootimage not found. Build the kernel first."
        exit 1
    fi

    if ! command_exists qemu-system-x86_64; then
        print_error "QEMU not found. Please install QEMU to run the kernel."
        print_status "On macOS: brew install qemu"
        print_status "On Ubuntu: sudo apt install qemu-system-x86"
        exit 1
    fi

    print_status "Starting QEMU..."
    print_status "Press Ctrl+A then X to exit QEMU"
    print_status "Press Ctrl+A then C for QEMU monitor"
    echo

    # QEMU arguments
    local qemu_args=""
    qemu_args="$qemu_args -drive format=raw,file=$BOOTIMAGE_PATH"
    qemu_args="$qemu_args -serial stdio"
    qemu_args="$qemu_args -device isa-debug-exit,iobase=0xf4,iosize=0x04"
    qemu_args="$qemu_args -display gtk"
    qemu_args="$qemu_args -m 256M"
    qemu_args="$qemu_args -cpu qemu64"
    qemu_args="$qemu_args -enable-kvm"  # Enable KVM if available

    # Run QEMU
    qemu-system-x86_64 $qemu_args
}

# Run kernel tests
run_tests() {
    print_header "Running Kernel Tests"

    print_status "Running unit tests..."
    cargo test --manifest-path "$CARGO_FILE" --target "$TARGET"

    print_success "All tests passed"
}

# Show build summary
show_summary() {
    print_header "Build Summary"

    echo -e "${CYAN}Kernel:${NC} $KERNEL_NAME"
    echo -e "${CYAN}Target:${NC} $TARGET"
    echo -e "${CYAN}Cargo File:${NC} $CARGO_FILE"

    if [ -f "$BOOTIMAGE_PATH" ]; then
        local size=$(ls -lh "$BOOTIMAGE_PATH" | awk '{print $5}')
        echo -e "${CYAN}Bootimage:${NC} $BOOTIMAGE_PATH ($size)"
    fi

    echo -e "${CYAN}Rust Version:${NC} $(rustc --version)"
    echo -e "${CYAN}Build Time:${NC} $(date)"

    if command_exists qemu-system-x86_64; then
        echo -e "${CYAN}QEMU:${NC} Available"
    else
        echo -e "${CYAN}QEMU:${NC} Not installed"
    fi
}

# Parse command line arguments
INSTALL_DEPS=false
BUILD=false
RUN=false
CLEAN=false
TEST=false
VERBOSE=false
BOOTIMAGE_PATH=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -i|--install)
            INSTALL_DEPS=true
            shift
            ;;
        -b|--build)
            BUILD=true
            shift
            ;;
        -r|--run)
            BUILD=true
            RUN=true
            shift
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        -t|--test)
            TEST=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Main execution
main() {
    print_header "RustOS Simple Kernel Build System"

    # Install dependencies if requested
    if [ "$INSTALL_DEPS" = true ]; then
        install_rust
    fi

    # Clean if requested
    if [ "$CLEAN" = true ]; then
        clean_build
    fi

    # Run tests if requested
    if [ "$TEST" = true ]; then
        run_tests
    fi

    # Build kernel if requested
    if [ "$BUILD" = true ]; then
        build_kernel
        create_bootimage
    fi

    # Run in QEMU if requested
    if [ "$RUN" = true ]; then
        run_qemu
    fi

    # Show summary
    if [ "$BUILD" = true ] || [ "$INSTALL_DEPS" = true ]; then
        echo
        show_summary
    fi

    # Show usage if no options
    if [ "$INSTALL_DEPS" = false ] && [ "$BUILD" = false ] && [ "$CLEAN" = false ] && [ "$TEST" = false ]; then
        show_help
        exit 1
    fi

    print_success "Build script completed!"
}

# Run main function
main "$@"
