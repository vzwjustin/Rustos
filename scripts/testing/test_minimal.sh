#!/bin/bash
# RustOS Minimal Test Script
# Tests with a very simple kernel configuration

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found at $KERNEL_IMAGE"
    exit 1
fi

echo "=== Testing with minimal QEMU configuration ==="
echo ""

# Try the simplest possible boot
echo "Running QEMU for 5 seconds..."
qemu-system-x86_64 \
    -drive format=raw,file="$KERNEL_IMAGE" \
    -m 64M \
    -display none \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -serial mon:stdio &
    
QEMU_PID=$!
sleep 5
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null
