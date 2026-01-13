#!/bin/bash
# Quick verification that QEMU can start

echo "=== QEMU Binary Test ==="
which qemu-system-x86_64
qemu-system-x86_64 --version

echo ""
echo "=== Kernel File Test ==="
ls -la target/x86_64-rustos/debug/bootimage-rustos.bin

echo ""
echo "=== Attempting Boot (brief) ==="

# Brief boot test
timeout 3 bash -c 'qemu-system-x86_64 -drive format=raw,file=target/x86_64-rustos/debug/bootimage-rustos.bin -m 64M -machine pc -cpu qemu64 -nographic -no-reboot' 2>&1 | head -10

echo ""
echo "=== Test completed ==="
