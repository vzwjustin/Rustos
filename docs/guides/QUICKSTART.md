# 🚀 RustOS Quick Start Guide

Welcome to RustOS! This guide will help you build and run the enterprise-grade RustOS kernel in minutes.

## 📋 Prerequisites

### Required Tools
- **Git** - For cloning the repository
- **Internet connection** - For downloading Rust and dependencies
- **Linux/macOS/WSL** - Windows users should use WSL2

### System Requirements
- **Memory**: At least 4GB RAM
- **Storage**: 2GB free space
- **Architecture**: x86_64 or AArch64

## ⚡ Quick Setup (One Command)

For the fastest setup, run this one-liner:

```bash
# Clone, install dependencies, and build
git clone <repository-url> RustOS && cd RustOS && ./build_rustos.sh --install-deps --bootimage
```

## 🔧 Step-by-Step Setup

### 1. Clone the Repository
```bash
git clone <repository-url> RustOS
cd RustOS
```

### 2. Install Dependencies
```bash
# Using the build script (recommended)
./build_rustos.sh --install-deps

# Or using Make
make install-deps
```

This will install:
- Rust nightly toolchain
- Required Rust components (rust-src, llvm-tools-preview)
- Bootimage tool
- QEMU (for testing)

### 3. Build the Kernel

#### Quick Build (Debug)
```bash
make build
# or
./build_rustos.sh
```

#### Release Build
```bash
make build-release
# or
./build_rustos.sh --release
```

#### Create Bootable Image
```bash
make bootimage
# or
./build_rustos.sh --bootimage
```

### 4. Run in QEMU

#### Run Debug Kernel
```bash
make run
# or
./build_rustos.sh --qemu
```

#### Run Release Kernel
```bash
make run-release
# or
./build_rustos.sh --release --qemu
```

## 🎯 Common Operations

### Build Commands
| Command | Description |
|---------|-------------|
| `make build` | Build debug kernel |
| `make build-release` | Build release kernel |
| `make bootimage` | Create bootable image |
| `make clean` | Clean build artifacts |
| `make rebuild` | Clean and build |

### Testing Commands
| Command | Description |
|---------|-------------|
| `make test` | Run kernel tests |
| `make check` | Check compilation only |
| `make run` | Build and run in QEMU |
| `make run-vnc` | Run with VNC display |

### Development Commands
| Command | Description |
|---------|-------------|
| `make dev` | Quick dev cycle (clean + build + run) |
| `make watch` | Auto-rebuild on file changes |
| `make format` | Format code with rustfmt |
| `make lint` | Lint code with clippy |

## 🏗️ Build Targets

### Architecture-Specific Builds
```bash
# Build for x86_64 (default)
make build-x86
./build_rustos.sh --target x86_64-rustos.json

# Build for AArch64
make build-arm  
./build_rustos.sh --target aarch64-apple-rustos.json
```

### Advanced Build Options
```bash
# Verbose build
./build_rustos.sh --verbose

# Clean build with bootimage
./build_rustos.sh --clean --bootimage

# Full pipeline: clean, build, test, create bootimage, run
./build_rustos.sh --clean --test --bootimage --qemu
```

## 🖥️ QEMU Usage

### Basic QEMU Commands
```bash
# Run with default settings
make run

# Run with VNC (headless)
make run-vnc

# Run with custom memory
qemu-system-x86_64 -m 512M -drive format=raw,file=target/x86_64-rustos.json/debug/bootimage-kernel.bin
```

### QEMU Controls
- **Exit QEMU**: `Ctrl+A, then X`
- **QEMU Monitor**: `Ctrl+A, then C`
- **Switch to console**: `Ctrl+A, then 1`
- **Force quit**: `Ctrl+C` (in terminal)

## 📊 Verification

### Check Build Success
```bash
# Show build information
make info

# Show kernel size
make size

# Verify kernel binary exists
ls -la target/x86_64-rustos.json/debug/kernel
```

### Expected Output
When you run `make run`, you should see:
```
RustOS - Hardware-Optimized AI Operating System
Architecture: x86_64/aarch64 compatible
Initializing hardware-focused AI kernel components...
GPU Acceleration: Available and Active
Peripheral Drivers: All hardware drivers initialized
RustOS AI kernel successfully initialized!
```

## 🐛 Troubleshooting

### Common Issues

#### "Rust not found"
```bash
# Install Rust manually
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup default nightly
```

#### "Target not found"
```bash
# Add required components
rustup component add rust-src llvm-tools-preview
```

#### "QEMU not found"
```bash
# Ubuntu/Debian
sudo apt install qemu-system-x86

# macOS
brew install qemu

# Arch Linux
sudo pacman -S qemu
```

#### "Permission denied: ./build_rustos.sh"
```bash
chmod +x build_rustos.sh
```

#### Build errors
```bash
# Clean everything and retry
make clean
cargo clean
rm -rf ~/.cargo/registry/index
./build_rustos.sh --install-deps
```

### Getting Help
```bash
# Show all available commands
make help

# Show build script options
./build_rustos.sh --help

# Check system requirements
make info
```

## 🔍 What's Included

RustOS includes these major components:
- **Memory Management**: Virtual memory, paging, heap allocation
- **Process Management**: Multi-process scheduling, IPC
- **System Calls**: 200+ POSIX-like + AI/GPU syscalls
- **Drivers**: Network, storage, USB, audio, input devices
- **AI Integration**: Neural networks, GPU acceleration
- **Security**: Memory protection, access control
- **Monitoring**: Real-time performance tracking

## 🚀 Next Steps

### Explore the Kernel
```bash
# View kernel features demo
make run  # Then watch the boot sequence

# Run comprehensive tests
make test

# Generate documentation
make docs
```

### Development Workflow
```bash
# Start development server
make watch  # Auto-rebuilds on changes

# Format and lint before committing  
make format lint

# Create distribution
make dist
```

### Advanced Features
```bash
# Profile kernel performance
make benchmark

# Debug with GDB
make debug

# Create kernel documentation
cargo doc --open
```

## 📚 Additional Resources

- **Main Documentation**: `docs/` directory
- **Architecture Guide**: `KERNEL_IMPROVEMENTS.md`
- **Feature Demo**: `demo.md` and `demo_advanced_features.md`
- **Build Configuration**: `Cargo.toml`, `Makefile`
- **Target Specifications**: `x86_64-rustos.json`, `aarch64-apple-rustos.json`

## 💡 Tips

1. **Use `make help`** to see all available commands
2. **Start with `make dev`** for quick development cycles
3. **Use `make run-vnc`** for headless systems
4. **Check `make info`** if something isn't working
5. **Run `make clean`** if you encounter weird build issues

---

**Happy Hacking with RustOS! 🦀🚀**

For more detailed information, see the full documentation in the `docs/` directory.