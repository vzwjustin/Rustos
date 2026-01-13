# RustOS Technology Stack

## Core Technologies
- **Language**: Rust (nightly toolchain required)
- **Edition**: 2021
- **Target**: `no_std` kernel environment
- **Architecture**: x86_64 primary, AArch64 secondary

## Build System
- **Primary**: Custom build script (`build_rustos.sh`)
- **Secondary**: Makefile with comprehensive targets
- **Bootloader**: bootloader crate v0.9.23 (bootimage compatible)
- **Cross-compilation**: Custom target specifications (x86_64-rustos.json, aarch64-apple-rustos.json)

## Key Dependencies
- **Hardware Access**: x86_64, volatile, pic8259, uart_16550
- **Memory Management**: linked_list_allocator, bootloader
- **Synchronization**: spin, lazy_static (with spin_no_std)
- **I/O**: pc-keyboard, heapless, embedded-hal
- **Utilities**: bitflags, libm, paste, log

## Required Tools
- Rust nightly with components: rust-src, llvm-tools-preview
- bootimage cargo plugin
- QEMU (qemu-system-x86_64, qemu-system-aarch64)
- Standard build tools (make, cc)

## Common Commands

### Setup
```bash
# Install dependencies
make install-deps
./build_rustos.sh --install-deps

# Check environment
make info
make check
```

### Building
```bash
# Debug build
make build
cargo build -Zbuild-std=core,compiler_builtins,alloc --target x86_64-rustos.json

# Release build
make build-release
./build_rustos.sh --release

# Cross-platform
make build-x86    # x86_64
make build-arm    # AArch64
```

### Testing & Running
```bash
# Run in QEMU
make run          # Debug with desktop
make run-release  # Optimized build
make run-vnc      # Headless with VNC

# Testing
make test
./build_rustos.sh --test

# Create bootable images
make bootimage
make bootimage-release
```

### Development
```bash
# Quick dev cycle
make dev          # clean + build + run
make watch        # Auto-rebuild on changes

# Code quality
make format       # rustfmt
make lint         # clippy
make docs         # Generate documentation

# Analysis
make size         # Binary sizes
make objdump      # Disassembly
make nm           # Symbols
```

## Build Configuration
- **Panic**: abort (both debug/release)
- **LTO**: Enabled in release
- **Codegen Units**: 1 (release optimization)
- **QEMU Args**: 512M RAM, serial stdio, Q35 chipset, APIC enabled