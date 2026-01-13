#!/bin/bash
# RustOS QEMU Compatibility Script
# More conservative settings for older systems

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found at $KERNEL_IMAGE"
    echo "Run 'cargo bootimage --bin rustos' first to build the kernel"
    exit 1
fi

echo "Starting RustOS with QEMU (compatibility mode)..."
echo "Kernel image: $KERNEL_IMAGE"
echo ""

# Try with KVM first, fall back without it
if qemu-system-x86_64 --version >/dev/null 2>&1; then
    echo "Attempting with hardware acceleration..."
    qemu-system-x86_64 \
        -drive format=raw,file="$KERNEL_IMAGE" \
        -m 256M \
        -smp 1 \
        -serial stdio \
        -display gtk \
        -machine q35 \
        -cpu Nehalem \
        -enable-kvm 2>/dev/null || {
        
        echo "KVM failed, trying without hardware acceleration..."
        qemu-system-x86_64 \
            -drive format=raw,file="$KERNEL_IMAGE" \
            -m 256M \
            -smp 1 \
            -serial stdio \
            -display cocoa \
            -machine q35 \
            -cpu qemu64
    }
else
    echo "Error: qemu-system-x86_64 not found"
    echo "Install QEMU: brew install qemu"
    exit 1
fi
