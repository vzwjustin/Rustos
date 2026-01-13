#!/bin/bash
# Force kill any stuck QEMU processes and try again
pkill -9 qemu-system-x86_64 2>/dev/null || true
sleep 1

echo "=== Fresh QEMU Boot Test ==="
echo "Starting QEMU directly..."
echo ""

# Direct boot test
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 64M \
    -machine pc \
    -cpu qemu64 \
    -nographic \
    -no-reboot &
    
QEMU_PID=$!
sleep 3
echo "QEMU PID: $QEMU_PID"
echo "Sending SIGTERM to QEMU..."
kill $QEMU_PID 2>/dev/null
sleep 1
kill -9 $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "=== Boot test completed ==="
