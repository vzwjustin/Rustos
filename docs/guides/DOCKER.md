# Docker Guide for RustOS Kernel Development

This guide explains how to use Docker to build, test, and develop the RustOS kernel in a consistent, containerized environment.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Building the Docker Image](#building-the-docker-image)
- [Running Different Services](#running-different-services)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Advanced Usage](#advanced-usage)
- [Troubleshooting](#troubleshooting)
- [Docker Architecture](#docker-architecture)

## Prerequisites

- Docker Engine (version 20.10 or later)
- Docker Compose (version 2.0 or later)
- At least 4GB of free disk space
- 2GB of available RAM

### Installing Docker

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install docker.io docker-compose-plugin
sudo usermod -aG docker $USER
# Log out and back in for group changes to take effect
```

#### macOS
```bash
# Install Docker Desktop from https://docker.com/products/docker-desktop
# Or use Homebrew:
brew install --cask docker
```

#### Windows
Install Docker Desktop from https://docker.com/products/docker-desktop

## Quick Start

The fastest way to build and test RustOS:

```bash
# Clone the repository (if not already done)
git clone <repository-url>
cd Rustos-main

# Build and run full test pipeline
docker-compose up rustos-dev

# Or use the profile for cleaner output
docker-compose --profile test up
```

This will:
1. Build the Docker image with all dependencies
2. Compile the RustOS kernel
3. Run tests
4. Create a bootable image
5. Display usage instructions

## Building the Docker Image

### Manual Build
```bash
# Build the image manually
docker build -t rustos:latest .

# Build with specific tag
docker build -t rustos:dev-$(date +%Y%m%d) .
```

### Using Docker Compose
```bash
# Build all services
docker-compose build

# Build specific service
docker-compose build rustos-dev
```

## Running Different Services

### 1. Full Development Environment

Run the complete build and test pipeline:

```bash
docker-compose --profile dev up rustos-dev
```

**What it does:**
- Builds the kernel for x86_64
- Runs unit tests
- Creates a bootable image
- Shows available commands

### 2. Build Only

For CI/CD or when you only need to build:

```bash
docker-compose --profile build up rustos-build
```

**What it does:**
- Builds the kernel binary only
- Outputs build artifacts to volume
- Exits after successful build

### 3. Interactive Development Shell

For development and debugging:

```bash
docker-compose --profile shell up rustos-shell
```

**What it does:**
- Starts an interactive bash shell
- Mounts source code for live editing
- Preserves cargo cache between sessions

### 4. QEMU Testing

To run the kernel in QEMU emulator:

```bash
# First ensure you have a bootimage
docker-compose --profile build up rustos-build
docker-compose run --rm rustos-dev ./create_bootimage.sh

# Then run in QEMU (headless mode)
docker-compose --profile qemu up rustos-qemu
```

## Development Workflow

### 1. Interactive Development

```bash
# Start development shell
docker-compose --profile shell up -d rustos-shell

# Attach to the running container
docker exec -it rustos-kernel-shell bash

# Inside the container, you can:
./build_kernel.sh           # Build the kernel
./create_bootimage.sh      # Create bootable image  
./run_qemu.sh             # Test in QEMU
cargo test --target x86_64-rustos.json  # Run tests
```

### 2. Live Code Changes

The development container mounts your source code, so changes made on your host system are immediately available inside the container.

```bash
# Edit files on your host system using your preferred editor
vim src/main.rs

# Build inside the container to see changes
docker exec rustos-kernel-shell ./build_kernel.sh
```

### 3. Build Different Targets

```bash
# Build for x86_64 (default)
cargo build --target x86_64-rustos.json

# Build for AArch64 (if supported)
cargo build --target aarch64-apple-rustos.json

# Build in release mode
cargo build --target x86_64-rustos.json --release

# Create release bootimage
bootimage build --target x86_64-rustos.json --release
```

## Testing

### Unit Tests

```bash
# Run unit tests
docker-compose run --rm rustos-dev cargo test --target x86_64-rustos.json

# Run tests with output
docker-compose run --rm rustos-dev cargo test --target x86_64-rustos.json -- --nocapture
```

### Integration Testing

```bash
# Build and test complete pipeline
docker-compose --profile test up

# Test kernel in QEMU
docker-compose run --rm rustos-dev bash -c "
  ./build_kernel.sh &&
  ./create_bootimage.sh &&
  timeout 30s ./run_qemu.sh || echo 'QEMU test completed'
"
```

### Performance Testing

```bash
# Build optimized version
docker-compose run --rm rustos-dev cargo build --target x86_64-rustos.json --release

# Create optimized bootimage
docker-compose run --rm rustos-dev bootimage build --target x86_64-rustos.json --release
```

## Advanced Usage

### Custom Build Scripts

The container includes the original build script:

```bash
# Use original build script with options
docker-compose run --rm rustos-dev ./build_rustos.sh --help

# Clean build with bootimage
docker-compose run --rm rustos-dev ./build_rustos.sh --clean --release --bootimage

# Build and run in QEMU
docker-compose run --rm rustos-dev ./build_rustos.sh --clean --release --bootimage --qemu
```

### Volume Management

```bash
# Clean build cache
docker volume rm rustos-main_build-cache

# Clean cargo cache
docker volume rm rustos-main_cargo-cache

# Clean all volumes
docker-compose down --volumes
```

### Multi-Stage Builds

The Dockerfile supports multi-stage builds for optimization:

```bash
# Build only the base image
docker build --target base -t rustos:base .

# Build development image
docker build -t rustos:dev .
```

### Environment Variables

Customize the build environment:

```bash
# Set custom environment variables
docker-compose run --rm -e RUST_BACKTRACE=full rustos-dev ./build_kernel.sh

# Enable verbose output
docker-compose run --rm -e CARGO_VERBOSE=1 rustos-dev cargo build --target x86_64-rustos.json
```

## Troubleshooting

### Common Issues

#### 1. Permission Denied Errors

```bash
# Fix file permissions
sudo chown -R $USER:$USER .
```

#### 2. Out of Disk Space

```bash
# Clean up Docker images and volumes
docker system prune -a
docker volume prune
```

#### 3. Build Failures

```bash
# Clean rebuild
docker-compose build --no-cache
docker volume rm rustos-main_build-cache
docker-compose --profile dev up rustos-dev
```

#### 4. QEMU Display Issues

For QEMU GUI on Linux:

```bash
# Allow X11 forwarding
xhost +local:docker
docker-compose --profile qemu up rustos-qemu
```

#### 5. Cargo Cache Issues

```bash
# Reset cargo cache
docker volume rm rustos-main_cargo-cache rustos-main_cargo-git
docker-compose build
```

### Debug Information

```bash
# Check container logs
docker-compose logs rustos-dev

# Inspect running container
docker exec rustos-kernel-shell ps aux
docker exec rustos-kernel-shell df -h
docker exec rustos-kernel-shell free -h

# Check Rust installation
docker exec rustos-kernel-shell rustc --version
docker exec rustos-kernel-shell cargo --version
```

### Performance Optimization

```bash
# Use more CPU cores for building
docker-compose run --rm -e CARGO_BUILD_JOBS=4 rustos-dev ./build_kernel.sh

# Allocate more memory to container
docker run --memory=4g -it rustos:latest ./build_kernel.sh
```

## Docker Architecture

### Image Layers

1. **Base Layer**: Ubuntu 22.04 with system dependencies
2. **Rust Layer**: Rust nightly toolchain and components
3. **Tools Layer**: bootimage, cargo-binutils, and other tools
4. **Project Layer**: RustOS source code and build scripts

### Volumes

- `cargo-cache`: Caches downloaded Rust crates
- `cargo-git`: Caches Git repositories
- `build-cache`: Stores compiled artifacts
- `build-artifacts`: Output artifacts for CI/CD

### Networks

- `rustos-network`: Default network for all services

### Services

- **rustos-dev**: Full development environment
- **rustos-build**: Build-only service for CI/CD
- **rustos-shell**: Interactive development shell
- **rustos-qemu**: QEMU testing environment

## Best Practices

1. **Use profiles** for different workflows (`--profile dev`, `--profile test`)
2. **Preserve volumes** to avoid rebuilding dependencies
3. **Regular cleanup** to prevent disk space issues
4. **Version your images** for reproducible builds
5. **Use .dockerignore** to exclude unnecessary files

## Examples

### Complete Build and Test

```bash
# Full pipeline from scratch
docker-compose down --volumes
docker-compose build
docker-compose --profile test up
```

### Development Session

```bash
# Start development environment
docker-compose --profile shell up -d rustos-shell

# Work interactively
docker exec -it rustos-kernel-shell bash

# Inside container:
# Edit, build, test cycle
./build_kernel.sh
./create_bootimage.sh
cargo test --target x86_64-rustos.json

# Exit and cleanup
exit
docker-compose down
```

### CI/CD Integration

```bash
#!/bin/bash
# CI script example
set -e

echo "Building RustOS kernel..."
docker-compose --profile build up --abort-on-container-exit

echo "Running tests..."
docker-compose run --rm rustos-dev cargo test --target x86_64-rustos.json

echo "Creating artifacts..."
docker-compose run --rm rustos-dev ./create_bootimage.sh

echo "Build successful!"
```

For more information about RustOS development, see the main [README.md](README.md) and [QUICKSTART.md](QUICKSTART.md).