#!/bin/bash
# Simple Kernel Boot Test
# Tests the kernel with minimal settings

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

echo "=== Simple Kernel Boot Test ==="
echo "Testing with basic QEMU settings..."
echo ""

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found!"
    exit 1
fi

echo "Kernel size: $(stat -f%z "$KERNEL_IMAGE") bytes"

# Simple boot test - just boot and see what happens
echo "Booting kernel for 5 seconds..."
timeout 5s qemu-system-x86_64 \
    -drive format=raw,file="$KERNEL_IMAGE" \
    -m 64M \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -no-reboot 2>&1 | head -20

echo ""
echo "=== Test completed ==="
echo "If you saw boot messages, the kernel is working."
