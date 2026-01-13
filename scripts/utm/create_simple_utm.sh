#!/bin/bash

# Create a simple bootable image for UTM
# This creates a minimal bootable disk that UTM can handle

set -e

echo "Creating simple UTM bootable image..."

BOOTIMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"
OUTPUT="rustos_simple.img"

if [ ! -f "$BOOTIMAGE" ]; then
    echo "Error: Bootimage not found. Building kernel..."
    ./scripts/build_simple.sh --build
fi

echo "Creating raw bootable image..."

# Create a 10MB image and write bootloader at the beginning
dd if=/dev/zero of="$OUTPUT" bs=1M count=10
dd if="$BOOTIMAGE" of="$OUTPUT" conv=notrunc

echo "Success! Created: $OUTPUT"
echo "Size: $(ls -lh $OUTPUT | awk '{print $5}')"

echo ""
echo "UTM Setup Instructions for $OUTPUT:"
echo "1. Create new VM in UTM"
echo "2. Choose 'Virtualize' -> 'Other'"
echo "3. System: x86_64, 256MB RAM, 1 CPU"
echo "4. Boot: BIOS mode (not UEFI)"
echo "5. Drives: Import $OUTPUT as IDE drive"
echo "6. Display: Console Only"
echo "7. Start VM"
