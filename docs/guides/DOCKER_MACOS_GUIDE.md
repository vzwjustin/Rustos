# RustOS Docker Guide for macOS

## Quick Start

The fastest way to get RustOS running on macOS with Docker:

```bash
# 1. Check prerequisites
./scripts/docker-macos.sh check

# 2. Build optimized image
./scripts/docker-macos.sh build

# 3. Run complete development pipeline
./scripts/docker-macos.sh dev

# 4. Start interactive shell for development
./scripts/docker-macos.sh shell
```

---

## Common Issues and Solutions

### 🚨 "Build fails immediately"

**Problem**: Docker build fails right at the start
**Solution**:
```bash
# Check Docker is running
docker info

# If not running, start Docker Desktop
open -a Docker

# Wait for Docker to start, then retry
./scripts/docker-macos.sh check
```

### 🚨 "Platform warnings" or "Architecture mismatch"

**Problem**: Warnings about platform compatibility on Apple Silicon
**Solution**: This is normal and expected. The script uses `--platform linux/amd64` for consistency.

```bash
# For Apple Silicon Macs (M1/M2), this is normal:
WARNING: The requested image's platform (linux/amd64) does not match the detected host platform (linux/arm64/v8)

# No action needed - emulation works fine for kernel development
```

### 🚨 "No space left on device"

**Problem**: Docker runs out of disk space during build
**Solutions**:
```bash
# Clean up Docker
docker system prune -a

# Clean up RustOS caches
./scripts/docker-macos.sh clean

# Check Docker Desktop settings:
# - Increase disk image size (Settings → Resources → Advanced)
# - Recommended: 64GB+ for development
```

### 🚨 "Memory issues" or "Killed during build"

**Problem**: Build process runs out of memory
**Solutions**:
```bash
# Increase Docker memory (Docker Desktop → Settings → Resources)
# Recommended: 4GB+ for RustOS development

# Alternative: Use smaller parallel build
export CARGO_BUILD_JOBS=2
./scripts/docker-macos.sh build
```

### 🚨 "Target file not found" errors

**Problem**: x86_64-rustos.json not found
**Solution**:
```bash
# Ensure you're in the RustOS project root
ls -la x86_64-rustos.json Cargo.toml

# If missing, you're in wrong directory
cd /path/to/Rustos-main
./scripts/docker-macos.sh check
```

### 🚨 "Permission denied" errors

**Problem**: File permission issues with volumes
**Solution**:
```bash
# Fix ownership of project files
sudo chown -R $(whoami):$(id -gn) .

# Clean and rebuild
./scripts/docker-macos.sh clean
./scripts/docker-macos.sh build
```

### 🚨 "Cargo cache issues"

**Problem**: Corrupted cargo cache causing build failures
**Solution**:
```bash
# Clean all caches
./scripts/docker-macos.sh clean

# Remove cache directories
rm -rf /tmp/rustos-*

# Rebuild from scratch
./scripts/docker-macos.sh build
```

---

## Performance Optimization for macOS

### 1. Docker Desktop Settings

**Recommended Configuration**:
- **Memory**: 4GB minimum, 8GB preferred
- **CPU**: 4+ cores
- **Disk Image Size**: 64GB+
- **File Sharing**: Only share necessary directories

**Access Settings**:
```
Docker Desktop → Settings → Resources → Advanced
```

### 2. Apple Silicon (M1/M2) Optimization

The configuration automatically handles platform differences:

```bash
# The Dockerfile uses linux/amd64 platform for consistency
# This provides better compatibility at slight performance cost

# For maximum compatibility, use the optimized settings:
platform: linux/amd64  # Force x86_64 emulation
```

### 3. Volume Mount Optimization

The docker-compose file uses `:delegated` mounts for better performance:

```yaml
volumes:
  - .:/home/rustdev/rustos:delegated  # Faster on macOS
```

### 4. Cache Configuration

Optimized cache setup in `/tmp/` for speed:
- Cargo cache: `/tmp/rustos-cargo-cache`
- Build cache: `/tmp/rustos-build-cache`
- Git cache: `/tmp/rustos-cargo-git`

---

## Step-by-Step Troubleshooting

### Step 1: Basic Setup Check

```bash
# Run comprehensive check
./scripts/docker-macos.sh check

# Expected output:
✅ Docker found
✅ Docker daemon is running
✅ Sufficient disk space
✅ In RustOS project directory
✅ All prerequisites satisfied!
```

### Step 2: Build Image

```bash
# Build with verbose output
./scripts/docker-macos.sh build

# If this fails, check:
# 1. Docker memory allocation (4GB+)
# 2. Available disk space (5GB+)
# 3. Network connectivity for package downloads
```

### Step 3: Test Development Environment

```bash
# Run quick development test
./scripts/docker-macos.sh dev

# This should:
# ✅ Build kernel successfully
# ✅ Run tests (some may skip in container)
# ✅ Create bootimage
```

### Step 4: Interactive Development

```bash
# Start development shell
./scripts/docker-macos.sh shell

# Inside container, test commands:
check-env           # Verify environment
build-kernel        # Build the kernel
create-bootimage    # Create bootable image
test-kernel         # Run tests
run-qemu           # Test in QEMU
```

---

## Alternative Approaches

### Option 1: Direct Docker Commands

If the script doesn't work, use direct Docker commands:

```bash
# Build image
docker build -f Dockerfile.macos -t rustos:macos-latest --platform linux/amd64 .

# Run development container
docker run --rm -it \
  --platform linux/amd64 \
  -v "$(pwd):/home/rustdev/rustos" \
  -e RUST_BACKTRACE=1 \
  rustos:macos-latest /bin/bash
```

### Option 2: Docker Compose

```bash
# Use Docker Compose directly
docker-compose -f docker-compose.macos.yml --profile dev up

# Interactive shell
docker-compose -f docker-compose.macos.yml --profile shell run rustos-shell
```

### Option 3: Native Development

If Docker continues to fail, consider native development:

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview

# Install bootimage
cargo install bootimage

# Install QEMU
brew install qemu

# Build natively
cargo build --target x86_64-rustos.json
```

---

## Common Error Messages

### "docker: command not found"
```bash
# Install Docker Desktop for macOS
# Download from: https://www.docker.com/products/docker-desktop/

# Or use Homebrew
brew install --cask docker
```

### "Cannot connect to the Docker daemon"
```bash
# Start Docker Desktop
open -a Docker

# Wait for startup (check menubar icon)
# Retry after Docker is running
```

### "platform does not match"
```bash
# This warning is normal on Apple Silicon
# The build will work correctly with emulation
# No action needed
```

### "target/x86_64-rustos.json/debug/rustos: No such file"
```bash
# Build hasn't completed successfully
# Check build output for actual error
# Common issues: out of memory, missing dependencies
```

### "bootimage not found"
```bash
# Inside container, run:
build-kernel && create-bootimage

# Or check if bootimage tool is installed:
cargo install bootimage
```

---

## Debug Information

### Container Inspection

```bash
# Check running containers
docker ps

# Inspect container details
docker inspect rustos-macos-dev

# View container logs
docker logs rustos-macos-dev
```

### Volume Inspection

```bash
# Check mounted volumes
docker volume ls | grep rustos

# Inspect volume details
docker volume inspect rustos_build-cache-macos
```

### Resource Usage

```bash
# Check Docker resource usage
docker stats

# Check system resources
top -o cpu
```

---

## Getting Help

### 1. Enable Verbose Output

```bash
# Run with debug info
RUST_BACKTRACE=full ./scripts/docker-macos.sh dev

# Docker build with verbose output
docker build -f Dockerfile.macos -t rustos:macos-latest --progress=plain .
```

### 2. Collect Debug Information

```bash
# System info
uname -a
docker version
docker-compose version

# Project info
ls -la Cargo.toml x86_64-rustos.json Dockerfile.macos

# Resource info
df -h .
docker system df
```

### 3. Reset Everything

```bash
# Complete reset
./scripts/docker-macos.sh clean
docker system prune -a
rm -rf /tmp/rustos-*

# Start fresh
./scripts/docker-macos.sh check
./scripts/docker-macos.sh build
```

---

## Success Criteria

You know everything is working when:

✅ `./scripts/docker-macos.sh check` passes all checks
✅ `./scripts/docker-macos.sh build` completes without errors
✅ `./scripts/docker-macos.sh dev` builds kernel and creates bootimage
✅ `./scripts/docker-macos.sh shell` gives you an interactive environment
✅ Inside container: `build-kernel && create-bootimage && run-qemu` works

---

## Performance Expectations

**Build Times** (Apple Silicon M1):
- First build: 10-15 minutes
- Incremental builds: 1-3 minutes
- Clean rebuild: 5-8 minutes

**Build Times** (Intel Mac):
- First build: 8-12 minutes
- Incremental builds: 30s-2 minutes
- Clean rebuild: 4-6 minutes

**Memory Usage**:
- Docker container: 1-2GB during build
- Cached artifacts: 500MB-1GB
- Total project: 2-3GB

---

*For additional help, see the main BUILD_GUIDE.md or create an issue with your specific error messages.*