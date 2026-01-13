#!/bin/bash
# UTM-Compatible QEMU Test
# This uses the exact same settings UTM would use

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found at $KERNEL_IMAGE"
    echo "Run 'cargo bootimage --bin rustos' first"
    exit 1
fi

echo "=== Testing RustOS with UTM-Compatible Settings ==="
echo "This mimics how UTM boots raw kernels"
echo ""

echo "Kernel: $KERNEL_IMAGE"
echo "Size: $(stat -f%z "$KERNEL_IMAGE") bytes"
echo ""

echo "Starting QEMU with UTM settings..."
echo "You should see your kernel boot messages"
echo ""

# UTM-compatible QEMU settings for raw kernel boot
qemu-system-x86_64 \
    -kernel "$KERNEL_IMAGE" \
    -m 256M \
    -smp 2 \
    -machine q35 \
    -cpu Nehalem \
    -display cocoa \
    -serial stdio \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -no-reboot
