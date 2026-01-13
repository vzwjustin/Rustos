#!/bin/bash

# RustOS Docker Quick Start Script
# This script helps you get started with RustOS development using Docker

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Print colored output
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

print_header() {
    echo -e "${PURPLE}================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}================================${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Show help information
show_help() {
    cat << EOF
RustOS Docker Quick Start

Usage: $0 [COMMAND]

COMMANDS:
    help            Show this help message
    check           Check prerequisites
    build           Build Docker image
    dev             Start development environment
    shell           Start interactive shell
    test            Run full test suite
    clean           Clean up containers and volumes
    qemu            Build and run in QEMU (headless)
    gui             Build and run with desktop GUI
    demo            Run a complete demo
    setup-x11       Set up X11 forwarding for GUI

EXAMPLES:
    $0              # Interactive menu
    $0 dev          # Start development environment
    $0 shell        # Start interactive shell
    $0 test         # Run tests
    $0 gui          # Test desktop environment with GUI
    $0 clean        # Clean up everything

For more advanced usage, see DOCKER.md or use the Makefile:
    make help       # Show Makefile commands
    make dev        # Development environment
    make test       # Run tests

EOF
}

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"

    local missing_deps=()

    # Check Docker
    if command_exists docker; then
        print_success "Docker found: $(docker --version | head -1)"

        # Check if Docker daemon is running
        if ! docker info >/dev/null 2>&1; then
            print_error "Docker daemon is not running. Please start Docker."
            missing_deps+=("docker-daemon")
        fi
    else
        print_error "Docker not found. Please install Docker."
        missing_deps+=("docker")
    fi

    # Check Docker Compose
    if command_exists docker-compose; then
        print_success "Docker Compose found: $(docker-compose --version | head -1)"
    elif docker compose version >/dev/null 2>&1; then
        print_success "Docker Compose (plugin) found: $(docker compose version | head -1)"
    else
        print_warning "Docker Compose not found. Will use docker run commands as fallback."
        print_status "For full functionality, install Docker Compose"
    fi

    # Check disk space (need at least 4GB)
    local available_space=$(df . | tail -1 | awk '{print $4}')
    local available_gb=$((available_space / 1024 / 1024))

    if [ $available_gb -gt 4 ]; then
        print_success "Sufficient disk space available: ${available_gb}GB"
    else
        print_warning "Low disk space: ${available_gb}GB available (4GB+ recommended)"
    fi

    # Check if we're in the right directory
    if [ -f "Cargo.toml" ] && [ -f "Dockerfile" ]; then
        print_success "In RustOS project directory"
    else
        print_error "Not in RustOS project directory. Please run this script from the project root."
        missing_deps+=("wrong-directory")
    fi

    if [ ${#missing_deps[@]} -eq 0 ]; then
        print_success "All prerequisites satisfied!"
        return 0
    else
        print_error "Missing prerequisites: ${missing_deps[*]}"
        echo ""
        print_status "Installation help:"
        echo "  Ubuntu/Debian: sudo apt-get install docker.io docker-compose-plugin"
        echo "  macOS: brew install --cask docker"
        echo "  Windows: Install Docker Desktop from https://docker.com"
        echo ""
        return 1
    fi
}

# Get Docker Compose command or fallback to docker
get_compose_cmd() {
    if command_exists docker-compose; then
        echo "docker-compose"
    elif docker compose version >/dev/null 2>&1; then
        echo "docker compose"
    else
        echo "docker"
    fi
}

# Check if we have compose functionality
has_compose() {
    local cmd=$(get_compose_cmd)
    [ "$cmd" != "docker" ]
}

# Run container with compose or docker
run_container() {
    local service_name=$1
    local command_args=$2
    local compose_cmd=$(get_compose_cmd)

    if has_compose; then
        $compose_cmd run --rm $service_name $command_args
    else
        # Fallback to plain docker
        print_warning "Using docker run fallback (limited functionality)"
        docker run --rm -it \
            -v "$(pwd):/home/rustdev/rustos" \
            -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
            -e DISPLAY="${DISPLAY:-:0}" \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            rustos:latest $command_args
    fi
}

# Build Docker image
build_image() {
    print_header "Building RustOS Docker Image"

    local compose_cmd=$(get_compose_cmd)

    print_status "Building Docker image (this may take several minutes)..."

    if has_compose; then
        $compose_cmd build rustos-dev
    else
        print_status "Building with docker build..."
        docker build -t rustos:latest .
    fi

    print_success "Docker image built successfully!"

    # Show image info
    local image_size=$(docker images rustos:latest --format "table {{.Size}}" | tail -1 2>/dev/null || echo "Unknown")
    print_status "Image size: $image_size"
}

# Start development environment
start_dev() {
    print_header "Starting RustOS Development Environment"

    local compose_cmd=$(get_compose_cmd)

    print_status "Starting development pipeline..."
    print_status "This will:"
    echo "  1. Build the RustOS kernel"
    echo "  2. Run tests"
    echo "  3. Create a bootable image"
    echo "  4. Show available commands"
    echo ""

    if has_compose; then
        $compose_cmd --profile dev up rustos-dev
    else
        run_container "rustos-dev" "./full_test.sh"
    fi

    print_success "Development pipeline completed!"
}

# Start interactive shell
start_shell() {
    print_header "Starting Interactive Development Shell"

    local compose_cmd=$(get_compose_cmd)

    if has_compose; then
        print_status "Starting development shell..."
        $compose_cmd --profile shell up -d rustos-shell

        print_status "Attaching to shell..."
        print_warning "Inside the container, you can use:"
        echo "  ./build_kernel.sh      - Build the kernel"
        echo "  ./create_bootimage.sh  - Create bootable image"
        echo "  ./run_qemu.sh          - Test in QEMU"
        echo "  cargo test --target x86_64-rustos.json  - Run tests"
        echo "  exit                   - Leave shell (container keeps running)"
        echo ""

        docker exec -it rustos-kernel-shell bash

        print_status "To stop the shell container: $compose_cmd stop rustos-shell"
    else
        print_status "Starting interactive shell..."
        print_warning "Inside the container, you can use:"
        echo "  ./build_kernel.sh      - Build the kernel"
        echo "  ./create_bootimage.sh  - Create bootable image"
        echo "  ./run_qemu.sh          - Test in QEMU"
        echo "  cargo test --target x86_64-rustos.json  - Run tests"
        echo "  exit                   - Leave shell"
        echo ""

        run_container "rustos-dev" "/bin/bash"
    fi
}

# Run tests
run_tests() {
    print_header "Running RustOS Test Suite"

    local compose_cmd=$(get_compose_cmd)

    print_status "Running comprehensive test suite..."

    if has_compose; then
        $compose_cmd --profile test up rustos-dev
    else
        run_container "rustos-dev" "./full_test.sh"
    fi

    print_success "Test suite completed!"
}

# Clean up
cleanup() {
    print_header "Cleaning Up Docker Resources"

    local compose_cmd=$(get_compose_cmd)

    if has_compose; then
        print_status "Stopping containers..."
        $compose_cmd down --volumes --remove-orphans
    else
        print_status "Stopping any running containers..."
        docker ps -a --filter "ancestor=rustos:latest" --format "{{.ID}}" | xargs -r docker rm -f
    fi

    print_status "Removing images..."
    docker images -q rustos | xargs -r docker rmi -f

    print_success "Cleanup completed!"
}

# Build and run in QEMU
run_qemu_demo() {
    print_header "RustOS QEMU Demo"

    local compose_cmd=$(get_compose_cmd)

    print_status "Building kernel and creating bootimage..."
    run_container "rustos-dev" "bash -c './build_kernel.sh && ./create_bootimage.sh'"

    print_status "Starting QEMU (headless mode for 30 seconds)..."
    print_warning "In a real scenario, you can interact with the kernel"
    run_container "rustos-dev" "bash -c 'timeout 30s ./run_qemu.sh || echo \"QEMU demo completed\"'"

    print_success "QEMU demo completed!"
}

# Build and run with GUI
run_gui_demo() {
    print_header "RustOS Desktop GUI Demo"

    # Check X11 setup
    if ! setup_x11_forwarding; then
        print_error "X11 forwarding not available. GUI demo requires X11."
        return 1
    fi

    local compose_cmd=$(get_compose_cmd)

    print_status "Building kernel and creating bootimage..."
    run_container "rustos-dev" "bash -c './build_kernel.sh && ./create_bootimage.sh'"

    print_status "Starting RustOS with desktop GUI..."
    print_warning "Close QEMU window or press Ctrl+A then X to exit"
    print_status "You should see the RustOS desktop environment with windows!"

    if has_compose; then
        GUI_MODE=1 $compose_cmd run --rm rustos-dev bash -c "GUI_MODE=1 ./run_qemu.sh"
    else
        docker run --rm -it \
            -v "$(pwd):/home/rustdev/rustos" \
            -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
            -e DISPLAY="${DISPLAY:-:0}" \
            -e GUI_MODE=1 \
            -e RUST_BACKTRACE=1 \
            -e RUST_TARGET_PATH=/home/rustdev/rustos \
            --workdir /home/rustdev/rustos \
            rustos:latest bash -c "GUI_MODE=1 ./run_qemu.sh"
    fi

    print_success "GUI demo completed!"
}

# Set up X11 forwarding
setup_x11_forwarding() {
    print_header "Setting up X11 Forwarding"

    # Check if we're on Linux with X11
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if ! command_exists xhost; then
            print_error "xhost not found. Install X11 utilities: sudo apt-get install x11-utils"
            return 1
        fi

        print_status "Allowing Docker containers to access X11..."
        xhost +local:docker >/dev/null 2>&1

        if [ -z "$DISPLAY" ]; then
            print_warning "DISPLAY not set. Setting to :0"
            export DISPLAY=:0
        fi

        print_success "X11 forwarding configured for Linux"
        return 0

    elif [[ "$OSTYPE" == "darwin"* ]]; then
        print_warning "macOS detected. X11 forwarding requires XQuartz."
        print_status "Install XQuartz from https://www.xquartz.org/"
        print_status "After installing, restart and run: xhost +localhost"

        if ! command_exists xhost; then
            print_error "XQuartz not installed or xhost not in PATH"
            return 1
        fi

        print_status "Configuring XQuartz for Docker..."
        xhost +localhost >/dev/null 2>&1
        export DISPLAY=host.docker.internal:0

        print_success "X11 forwarding configured for macOS"
        return 0

    else
        print_error "Unsupported OS for X11 forwarding: $OSTYPE"
        print_status "GUI demo requires Linux with X11 or macOS with XQuartz"
        return 1
    fi
}

# Run complete demo
run_demo() {
    print_header "RustOS Complete Demo"

    print_status "This demo will:"
    echo "  1. Check prerequisites"
    echo "  2. Build Docker image"
    echo "  3. Run development pipeline"
    echo "  4. Run tests"
    echo "  5. Create bootimage"
    echo "  6. Run QEMU demo"
    echo "  7. Test desktop GUI (if X11 available)"
    echo ""

    read -p "Continue? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_status "Demo cancelled"
        exit 0
    fi

    check_prerequisites || exit 1
    build_image
    start_dev
    run_qemu_demo

    # Try GUI demo if X11 is available
    if setup_x11_forwarding >/dev/null 2>&1; then
        print_status "X11 available, testing desktop GUI..."
        run_gui_demo
    else
        print_warning "X11 not available, skipping GUI demo"
        print_status "For GUI testing, set up X11 forwarding and run: $0 gui"
    fi

    print_success "Complete demo finished!"
    print_status "To clean up: $0 clean"
}

# Interactive menu
show_menu() {
    while true; do
        print_header "RustOS Docker Quick Start Menu"
        echo "Select an option:"
        echo ""
        echo "  1. Check prerequisites"
        echo "  2. Build Docker image"
        echo "  3. Start development environment"
        echo "  4. Start interactive shell"
        echo "  5. Run test suite"
        echo "  6. QEMU demo (headless)"
        echo "  7. Desktop GUI demo"
        echo "  8. Set up X11 forwarding"
        echo "  9. Complete demo"
        echo "  a. Clean up"
        echo "  h. Help"
        echo "  0. Exit"
        echo ""

        read -p "Enter choice [0-9]: " choice
        echo ""

        case $choice in
            1) check_prerequisites ;;
            2) build_image ;;
            3) start_dev ;;
            4) start_shell ;;
            5) run_tests ;;
            6) run_qemu_demo ;;
            7) run_gui_demo ;;
            8) setup_x11_forwarding ;;
            9) run_demo ;;
            a|A) cleanup ;;
            h|H) show_help ;;
            0) print_status "Goodbye!"; exit 0 ;;
            *) print_error "Invalid choice. Please try again." ;;
        esac

        echo ""
        read -p "Press Enter to continue..."
        echo ""
    done
}

# Main function
main() {
    # Handle command line arguments
    case "${1:-}" in
        help|--help|-h)
            show_help
            exit 0
            ;;
        check)
            check_prerequisites
            ;;
        build)
            check_prerequisites || exit 1
            build_image
            ;;
        dev)
            check_prerequisites || exit 1
            start_dev
            ;;
        shell)
            check_prerequisites || exit 1
            start_shell
            ;;
        test)
            check_prerequisites || exit 1
            run_tests
            ;;
        qemu)
            check_prerequisites || exit 1
            run_qemu_demo
            ;;
        gui)
            check_prerequisites || exit 1
            run_gui_demo
            ;;
        setup-x11)
            setup_x11_forwarding
            ;;
        demo)
            run_demo
            ;;
        clean)
            cleanup
            ;;
        "")
            # No arguments - show interactive menu
            show_menu
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Trap Ctrl+C
trap 'print_warning "Interrupted by user"; exit 130' INT

# Run main function
main "$@"
