#!/bin/bash
# UTM Raw Kernel Boot Fix
# This creates a bootable disk image that UTM can boot properly

echo "=== Creating Bootable Disk for UTM ==="
echo ""

KERNEL_IMAGE="target/x86_64-rustos/debug/bootimage-rustos.bin"

if [ ! -f "$KERNEL_IMAGE" ]; then
    echo "Error: Kernel image not found!"
    exit 1
fi

echo "Kernel found: $KERNEL_IMAGE ($(stat -f%z "$KERNEL_IMAGE") bytes)"

# Create a bootable disk image
DISK_IMAGE="rust_os_disk.img"
echo "Creating bootable disk image: $DISK_IMAGE"

# Create 10MB disk image
dd if=/dev/zero of="$DISK_IMAGE" bs=1M count=10

# Format as FAT (optional, for bootloader compatibility)
# hdiutil create -size 10m -fs MS-DOS -volname RUSTOS -type SPARSEBUNDLE "$DISK_IMAGE"

echo ""
echo "=== UTM Configuration Fix ==="
echo ""
echo "PROBLEM: 'Booting from ROM...' means UTM is ignoring raw kernel"
echo "SOLUTION: Use the kernel as a boot disk instead"
echo ""
echo "IN UTM:"
echo "1. Create new VM: Emulate -> Other -> x86_64"
echo "2. System: Machine=Q35, CPU=Nehalem, Memory=256MB"
echo "3. Boot section:"
echo "   - Boot Device: Disk"
echo "   - Disk Format: Raw"
echo "   - Disk Interface: IDE"
echo "4. Select the kernel file as the disk image:"
echo "   $KERNEL_IMAGE"
echo "5. Start VM"
echo ""
echo "ALTERNATIVE: Use 'Use raw kernel and initrd' with these exact settings:"
echo "- Enable 'Use raw kernel and initrd'"
echo "- Kernel Path: $KERNEL_IMAGE"
echo "- Kernel Args: (leave empty)"
echo "- Initial RAM Disk: (leave empty)"
echo ""
echo "The kernel should now boot instead of showing 'Booting from ROM...'"

# Test with QEMU as disk
echo ""
echo "=== Testing with QEMU as Disk ==="
echo "This simulates how UTM should boot it..."

qemu-system-x86_64 \
    -drive format=raw,file="$KERNEL_IMAGE",index=0,media=disk \
    -m 256M \
    -machine q35 \
    -cpu Nehalem \
    -display cocoa \
    -no-reboot &
    
QEMU_PID=$!
echo "QEMU PID: $QEMU_PID"
echo "Running for 8 seconds..."
sleep 8
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "If you saw kernel messages, it works as a disk too!"
