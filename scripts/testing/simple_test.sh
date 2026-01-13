#!/bin/bash
# Simple QEMU test - runs for 10 seconds then exits

echo "=== Starting QEMU test ==="
echo "If you see boot messages, the kernel is working!"
echo ""

# Kill any existing QEMU processes
pkill -f qemu-system-x86_64 2>/dev/null || true

# Simple boot test
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 64M \
    -display none \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -serial stdio \
    -boot d &
    
QEMU_PID=$!
sleep 10
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "=== Test completed ==="
