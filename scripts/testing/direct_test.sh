#!/bin/bash
# Direct QEMU test without serial complications

echo "=== Direct QEMU Boot Test ==="
echo "This bypasses serial output issues"
echo ""

# Kill any existing QEMU processes
pkill -f qemu-system-x86_64 2>/dev/null || true

# Boot and capture any output
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 64M \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -monitor none \
    -parallel none \
    -no-reboot &
    
QEMU_PID=$!
sleep 5
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "=== Boot test completed ==="
echo "If the kernel is working, you should see boot messages above."
echo "If you only see QEMU messages, there might be an issue with the kernel."
