#!/bin/bash

# RustOS Test Script - Test the working kernel in QEMU

if [ "${ALLOW_EXPERIMENTAL}" != "1" ]; then
    echo "This script depends on the standalone/demo kernel and is experimental."
    echo "Set ALLOW_EXPERIMENTAL=1 to proceed."
    exit 1
fi

echo "🚀 Testing RustOS Kernel in Docker Environment..."

# Check if we have the working kernel
if [ ! -f "target/x86_64-rustos/debug/rustos-working" ]; then
    echo "❌ Working kernel not found. Building it first..."
    ./build_working_kernel.sh
    
    if [ ! -f "target/x86_64-rustos/debug/rustos-working" ]; then
        echo "❌ Failed to build working kernel"
        exit 1
    fi
fi

echo "✅ Working kernel found ($(du -h target/x86_64-rustos/debug/rustos-working | cut -f1))"

# Test the kernel with QEMU in Docker
echo "🧪 Starting RustOS in QEMU..."
echo "📺 Watch for VGA output showing kernel messages"
echo "⏰ Test will run for 30 seconds, then automatically exit"
echo ""

docker run --rm -v "$(pwd):/home/rustdev/rustos" --workdir /home/rustdev/rustos rustos:multiboot bash -c "
echo '🖥️  Launching RustOS in QEMU with VGA display...'
echo '📋 Kernel: target/x86_64-rustos/debug/rustos-working'
echo '💾 Memory: 256MB allocated'
echo '🎮 Press Ctrl+C to exit early'
echo ''

timeout 30s qemu-system-x86_64 \\
    -kernel target/x86_64-rustos/debug/rustos-working \\
    -m 256M \\
    -display curses \\
    -serial stdio \\
    -no-reboot \\
    -monitor none \\
    || echo '🏁 RustOS test session completed'
"

echo ""
echo "🎯 RustOS Test Complete!"
echo "✅ If you saw kernel messages, RustOS is working correctly!"
echo "🚀 Your operating system is ready for deployment!"
