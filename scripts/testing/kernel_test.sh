#!/bin/bash
# Simple Kernel Boot Test (no timeout dependency)

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

echo "=== Simple Kernel Boot Test ==="
echo "Testing with basic QEMU settings..."
echo ""

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found!"
    exit 1
fi

echo "Kernel size: $(stat -f%z "$KERNEL_IMAGE") bytes"

# Simple boot test - boot for 3 seconds then kill
echo "Booting kernel for 3 seconds..."
qemu-system-x86_64 \
    -drive format=raw,file="$KERNEL_IMAGE" \
    -m 64M \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -no-reboot &
    
QEMU_PID=$!
sleep 3
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "=== Test completed ==="
echo "If you saw any output above, the kernel is working."
