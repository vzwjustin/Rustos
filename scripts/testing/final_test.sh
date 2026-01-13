#!/bin/bash
# QEMU test with proper serial configuration

echo "=== Testing RustOS Kernel ==="
echo "This will show kernel output via serial port"
echo ""

# Kill any existing QEMU processes
pkill -f qemu-system-x86_64 2>/dev/null || true

# Test with proper serial configuration
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 64M \
    -display none \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -chardev stdio,id=char0 \
    -serial chardev:char0 &
    
QEMU_PID=$!
sleep 8
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "=== Test completed ==="
echo "If you didn't see any kernel output, there might be an issue with the kernel itself."
