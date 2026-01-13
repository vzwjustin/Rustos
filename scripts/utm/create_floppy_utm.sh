#!/bin/bash

# Create a floppy disk image for UTM
# Sometimes UTM boots floppy images more reliably

set -e

echo "Creating floppy disk image for UTM..."

BOOTIMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"
OUTPUT="rustos_floppy.img"

if [ ! -f "$BOOTIMAGE" ]; then
    echo "Error: Bootimage not found. Building kernel..."
    ./scripts/build_simple.sh --build
fi

# Create 1.44MB floppy image
echo "Creating 1.44MB floppy image..."
dd if=/dev/zero of="$OUTPUT" bs=512 count=2880

# Write bootloader
dd if="$BOOTIMAGE" of="$OUTPUT" conv=notrunc

echo "Success! Created floppy image: $OUTPUT"
echo "Size: $(ls -lh $OUTPUT | awk '{print $5}')"

echo ""
echo "UTM Floppy Setup:"
echo "1. Create new VM in UTM"
echo "2. Choose 'Virtualize' -> 'Other'"  
echo "3. System: x86_64, 256MB RAM"
echo "4. Boot: BIOS mode"
echo "5. Drives: Add 'Floppy Drive' and import $OUTPUT"
echo "6. Boot order: Floppy first"
echo "7. Display: Console Only"
echo "8. Start VM"
