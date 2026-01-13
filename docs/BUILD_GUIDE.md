# RustOS Build Guide

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Development Environment Setup](#development-environment-setup)
3. [Build Commands](#build-commands)
4. [Testing](#testing)
5. [Debugging](#debugging)
6. [Development Workflow](#development-workflow)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

#### Rust Toolchain
```bash
# Install Rust nightly (required for kernel development)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly

# Add required components
rustup component add rust-src llvm-tools-preview
rustup component add rustfmt clippy
```

#### Build Dependencies

**Linux/Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    qemu-system-x86 \
    qemu-system-misc \
    xorriso \
    grub-pc-bin \
    grub-common \
    mtools \
    nasm
```

**macOS:**
```bash
# Install Homebrew if not present
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install qemu nasm xorriso mtools
brew install --cask gcc-arm-embedded  # For ARM support
```

**Windows (WSL2):**
```bash
# Use Ubuntu/Debian instructions within WSL2
# Ensure WSL2 is installed with Ubuntu 20.04 or later
```

### Optional Tools

```bash
# Bootimage tool for creating bootable images
cargo install bootimage

# GDB for debugging
sudo apt install gdb  # Linux
brew install gdb     # macOS

# Cross-compilation tools (for ARM64)
rustup target add aarch64-unknown-none
```

---

## Development Environment Setup

### 1. Clone the Repository
```bash
git clone https://github.com/yourusername/rustos.git
cd rustos
```

### 2. Verify Installation
```bash
# Check Rust version (should be nightly)
rustc --version

# Check required components
rustup component list | grep -E "(rust-src|llvm-tools)"

# Check QEMU installation
qemu-system-x86_64 --version

# Verify build setup
make check
```

### 3. IDE Setup

#### Visual Studio Code
```bash
# Install recommended extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension serayuzgur.crates
```

**.vscode/settings.json:**
```json
{
    "rust-analyzer.cargo.target": "x86_64-rustos.json",
    "rust-analyzer.checkOnSave.allTargets": false,
    "rust-analyzer.cargo.features": ["no_std"],
    "rust-analyzer.diagnostics.disabled": ["unresolved-proc-macro"]
}
```

#### IntelliJ IDEA / CLion
- Install Rust plugin
- Configure custom target: Settings → Build → Rust → Custom target
- Set target specification: `x86_64-rustos.json`

---

## Build Commands

### Makefile Targets

```bash
# Display all available targets
make help

# Build debug kernel
make build

# Build optimized release kernel
make build-release

# Create bootable images
make bootimage          # Debug bootable image
make bootimage-release  # Release bootable image

# Run in QEMU
make run                # Run debug kernel
make run-release        # Run release kernel

# Testing
make test              # Run all tests
make test-unit         # Unit tests only
make test-integration  # Integration tests

# Code quality
make format            # Format code with rustfmt
make lint              # Run clippy linter
make check             # Check compilation without building

# Cleanup
make clean             # Remove build artifacts
make distclean         # Full cleanup including dependencies
```

### Direct Cargo Commands

```bash
# Build kernel binary
cargo +nightly build \
    --bin rustos \
    -Zbuild-std=core,compiler_builtins \
    --target x86_64-rustos.json

# Release build with optimizations
cargo +nightly build \
    --release \
    --bin rustos \
    -Zbuild-std=core,compiler_builtins \
    --target x86_64-rustos.json

# Check compilation
cargo +nightly check \
    --bin rustos \
    -Zbuild-std=core,compiler_builtins \
    --target x86_64-rustos.json
```

### Build Scripts

```bash
# Primary build script
./build_rustos.sh              # Build debug kernel
./build_rustos.sh --release    # Build release kernel
./build_rustos.sh --check-only # Check compilation only

# Create bootable images
./create_bootimage.sh          # Create BIOS bootable image
./create_final_multiboot.sh    # Create multiboot kernel

# Quick test run
./test_rustos.sh               # Build and run tests
```

### Building Specific Components

```bash
# Build only memory subsystem
cargo test -p rustos --lib memory

# Build network stack
cargo build --features network

# Build with GPU support
cargo build --features gpu

# Minimal kernel (for debugging)
cargo build --bin rustos --no-default-features --features minimal
```

---

## Testing

### Unit Tests

```bash
# Run all unit tests
make test-unit

# Run specific module tests
cargo test -p rustos --lib memory
cargo test -p rustos --lib process
cargo test -p rustos --lib net

# Run with verbose output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
make test-integration

# Run specific integration test
cargo test --test should_panic

# Run stress tests
cargo test --test stress_tests --release
```

### QEMU Testing

```bash
# Basic QEMU run
make run

# QEMU with debugging enabled
make run-debug

# QEMU with specific memory size
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 1G \
    -serial stdio

# QEMU with networking
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -netdev user,id=net0 \
    -device e1000,netdev=net0 \
    -serial stdio
```

### Performance Testing

```bash
# Build with benchmarking
cargo build --release --features benchmarks

# Run benchmarks
cargo bench --features benchmarks

# Profile with perf (Linux)
perf record -g make run-release
perf report
```

---

## Debugging

### GDB Debugging

```bash
# Start QEMU in debug mode
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -s -S \
    -serial stdio &

# Connect with GDB
gdb target/x86_64-rustos/debug/rustos
(gdb) target remote :1234
(gdb) break kernel_entry
(gdb) continue
```

### QEMU Monitor

```bash
# Start with monitor
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -monitor stdio

# Monitor commands
(qemu) info registers
(qemu) info mem
(qemu) info tlb
(qemu) x/10i $eip   # Examine instructions
```

### Serial Output Debugging

```rust
// In your kernel code
use crate::serial_println;

serial_println!("Debug: value = {:?}", some_value);
```

### Panic Handler

```rust
// src/kernel.rs
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC: {}", info);
    // Stack trace will be printed
    hlt_loop();
}
```

---

## Development Workflow

### 1. Feature Development

```bash
# Create feature branch
git checkout -b feature/network-improvements

# Make changes
vim src/net/tcp.rs

# Build and test
make build
make test

# Run in QEMU
make run

# Commit changes
git add -A
git commit -m "Improve TCP congestion control"
```

### 2. Code Quality Checks

```bash
# Format code
make format

# Run linter
make lint

# Check for common issues
cargo clippy -- -W clippy::all

# Security audit
cargo audit
```

### 3. Pre-commit Checklist

- [ ] Code compiles: `make build`
- [ ] Tests pass: `make test`
- [ ] Code formatted: `make format`
- [ ] No linter warnings: `make lint`
- [ ] Documentation updated
- [ ] CHANGELOG updated

### 4. Continuous Integration

**.github/workflows/ci.yml:**
```yaml
name: CI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          components: rust-src, llvm-tools-preview
      - run: make build
      - run: make test
      - run: make lint
```

---

## Build Configuration

### Target Specification (`x86_64-rustos.json`)

```json
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float"
}
```

### Cargo Configuration (`.cargo/config.toml`)

```toml
[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
build-std-features = ["compiler-builtins-mem"]

[build]
target = "x86_64-rustos.json"

[target.'cfg(target_os = "none")']
runner = "qemu-system-x86_64 -drive format=raw,file="

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
lto = true
opt-level = 3
```

---

## Troubleshooting

### Common Build Errors

#### Error: "can't find crate for `std`"
**Solution:** Ensure you're using `#![no_std]` and building with correct target:
```bash
cargo build --target x86_64-rustos.json
```

#### Error: "rust-lld not found"
**Solution:** Install LLVM tools:
```bash
rustup component add llvm-tools-preview
```

#### Error: "bootimage: command not found"
**Solution:** Install bootimage:
```bash
cargo install bootimage
```

#### Error: QEMU crashes immediately
**Solution:** Check boot image creation:
```bash
# Verify multiboot header
grub-file --is-x86-multiboot2 target/x86_64-rustos/debug/rustos

# Check image size
ls -lh target/x86_64-rustos/debug/bootimage-rustos.bin
```

### Performance Issues

#### Slow compilation
```bash
# Enable incremental compilation
export CARGO_INCREMENTAL=1

# Use sccache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

#### Out of memory during build
```bash
# Limit parallel jobs
export CARGO_BUILD_JOBS=2

# Or use make with limited jobs
make -j2 build
```

### Debugging Tips

1. **Enable verbose output:**
   ```bash
   RUST_BACKTRACE=1 make run
   ```

2. **Serial output for early boot:**
   ```rust
   // First thing in kernel_entry
   serial::init();
   serial_println!("Kernel starting...");
   ```

3. **Memory dumps:**
   ```bash
   # In QEMU monitor
   (qemu) dump-guest-memory kernel.dump
   # Analyze with gdb
   gdb -c kernel.dump target/x86_64-rustos/debug/rustos
   ```

---

## Contributing

### Code Style

- Follow Rust naming conventions
- Use `rustfmt` for formatting
- Keep functions under 50 lines
- Document public APIs
- Add unit tests for new features

### Commit Messages

```
component: Brief description

Longer explanation if needed.
Multiple paragraphs are fine.

Fixes: #issue-number
```

### Pull Request Process

1. Fork the repository
2. Create feature branch
3. Make changes with tests
4. Update documentation
5. Submit pull request
6. Address review feedback

---

## Resources

### Documentation
- [Rust OS Dev](https://os.phil-opp.com/)
- [OSDev Wiki](https://wiki.osdev.org/)
- [Intel Manuals](https://software.intel.com/content/www/us/en/develop/articles/intel-sdm.html)

### Tools
- [QEMU Documentation](https://www.qemu.org/documentation/)
- [GDB Documentation](https://www.gnu.org/software/gdb/documentation/)
- [Rust Book](https://doc.rust-lang.org/book/)

### Community
- RustOS Discord: [Join Server]
- GitHub Issues: [Report Bugs]
- Discussion Forum: [Ask Questions]

---

## Quick Reference Card

```bash
# Daily development
make build && make run          # Build and run
make test                        # Run tests
make format && make lint         # Code quality

# Debugging
make run-debug                   # QEMU with GDB
serial_println!("Debug: {}", x); # Debug output

# Release
make build-release               # Optimized build
make bootimage-release           # Release image

# Clean
make clean                       # Clean build
git clean -fdx                   # Full clean
```

---

For more information:
- [Architecture Overview](ARCHITECTURE.md)
- [API Reference](API_REFERENCE.md)
- [Module Index](MODULE_INDEX.md)
- [Driver Development](DRIVER_GUIDE.md)