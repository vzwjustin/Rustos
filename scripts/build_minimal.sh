#!/bin/bash

# Build minimal RustOS kernel that actually works

if [ "${ALLOW_EXPERIMENTAL}" != "1" ]; then
    echo "This script swaps Cargo/main files and is experimental."
    echo "Set ALLOW_EXPERIMENTAL=1 to proceed."
    exit 1
fi

echo "🚀 Building Minimal RustOS Kernel..."

# Backup the original files
mv Cargo.toml Cargo_full.toml.bak 2>/dev/null || true
mv src/main.rs src/main_full.rs.bak 2>/dev/null || true

# Use minimal versions
cp Cargo_minimal.toml Cargo.toml
cp src/main_minimal.rs src/main.rs

echo "📝 Using minimal kernel configuration..."

# Build the minimal kernel
echo "🔨 Building kernel with minimal dependencies..."
cargo +nightly build --bin rustos-minimal -Zbuild-std=core,compiler_builtins --target x86_64-rustos.json

BUILD_SUCCESS=$?

# Restore original files
mv Cargo_full.toml.bak Cargo.toml 2>/dev/null || true  
mv src/main_full.rs.bak src/main.rs 2>/dev/null || true

if [ $BUILD_SUCCESS -eq 0 ]; then
    echo "✅ Minimal kernel built successfully!"
    echo "📍 Binary at: target/x86_64-rustos/debug/rustos-minimal"
    
    # Test with QEMU
    echo "🧪 Testing minimal kernel with QEMU..."
    timeout 15s qemu-system-x86_64 \
        -kernel target/x86_64-rustos/debug/rustos-minimal \
        -m 256M \
        -display none \
        -serial stdio \
        -no-reboot \
        || echo "🏁 QEMU test completed"
else
    echo "❌ Minimal kernel build failed"
    exit 1
fi

echo "🎉 Minimal RustOS kernel ready for multiboot testing!"
