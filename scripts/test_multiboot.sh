#!/bin/bash

# Quick RustOS Multiboot Test
# Tests if the existing kernel binary can boot with multiboot

if [ "${ALLOW_MULTIBOOT}" != "1" ]; then
    echo "Multiboot path is experimental and unsupported in the default build."
    echo "Set ALLOW_MULTIBOOT=1 to proceed."
    exit 1
fi

echo "🚀 Testing RustOS Multiboot Compatibility..."

# Check if we have the kernel
if [ ! -f "target/x86_64-rustos/debug/rustos" ]; then
    echo "❌ No kernel binary found"
    exit 1
fi

echo "✅ Kernel binary exists ($(du -h target/x86_64-rustos/debug/rustos | cut -f1))"

# Test with QEMU in Docker - simple boot test
echo "🧪 Testing kernel boot with QEMU..."

docker run --rm -v "$(pwd):/home/rustdev/rustos" --workdir /home/rustdev/rustos rustos:latest bash -c "
echo '🖥️  Starting QEMU boot test...'
timeout 15s qemu-system-x86_64 \\
    -kernel target/x86_64-rustos/debug/rustos \\
    -m 256M \\
    -display none \\
    -serial stdio \\
    -no-reboot \\
    -monitor none \\
    -no-shutdown || echo '🏁 Boot test completed'
"

echo "🎯 Multiboot test finished!"
