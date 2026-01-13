#!/bin/bash

# Create UTM-compatible bootable disk image for RustOS
# This script creates a proper MBR-formatted disk image that UTM can boot from

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_status "Creating UTM-compatible RustOS bootable disk image..."

# Configuration
DISK_SIZE="50M"
BOOTIMAGE_PATH="target/x86_64-rustos/debug/bootimage-rustos.bin"
OUTPUT_IMAGE="rustos_utm.img"

# Check if bootimage exists
if [ ! -f "$BOOTIMAGE_PATH" ]; then
    print_error "Bootimage not found at $BOOTIMAGE_PATH"
    print_status "Please run: ./scripts/build_simple.sh --build"
    exit 1
fi

# Create empty disk image
print_status "Creating ${DISK_SIZE} disk image..."
dd if=/dev/zero of="$OUTPUT_IMAGE" bs=1M count=50 status=progress

# Create a single bootable partition using fdisk
print_status "Creating MBR partition table..."
(
echo o      # Create new empty DOS partition table
echo n      # Create new partition
echo p      # Primary partition
echo 1      # Partition number 1
echo        # First sector (default)
echo        # Last sector (default - use entire disk)
echo a      # Make partition bootable
echo 1      # Partition 1
echo w      # Write changes
) | fdisk "$OUTPUT_IMAGE" >/dev/null 2>&1

# Create loop device for the image (on macOS we'll use a different approach)
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS approach using hdiutil
    print_status "Setting up disk image for macOS..."
    
    # Create a temporary raw image with the bootloader
    cp "$OUTPUT_IMAGE" "${OUTPUT_IMAGE}.tmp"
    
    # Write the bootimage starting at sector 2048 (1MB offset for MBR)
    print_status "Writing RustOS bootloader to disk image..."
    dd if="$BOOTIMAGE_PATH" of="${OUTPUT_IMAGE}.tmp" bs=512 seek=2048 conv=notrunc status=progress
    
    # Replace original with updated version
    mv "${OUTPUT_IMAGE}.tmp" "$OUTPUT_IMAGE"
    
else
    # Linux approach using loop devices
    print_status "Setting up loop device..."
    LOOP_DEVICE=$(sudo losetup --find --show "$OUTPUT_IMAGE")
    
    # Partition the loop device
    sudo partprobe "$LOOP_DEVICE"
    
    # Write bootloader to the first partition
    print_status "Writing RustOS bootloader..."
    sudo dd if="$BOOTIMAGE_PATH" of="${LOOP_DEVICE}p1" bs=4096 status=progress
    
    # Clean up loop device
    sudo losetup -d "$LOOP_DEVICE"
fi

# Make the disk image bootable by writing MBR boot signature
print_status "Adding MBR boot signature..."
printf '\x55\xAA' | dd of="$OUTPUT_IMAGE" bs=1 seek=510 count=2 conv=notrunc

# Verify the image
print_status "Verifying disk image..."
ACTUAL_SIZE=$(ls -la "$OUTPUT_IMAGE" | awk '{print $5}')
print_success "Disk image created: $OUTPUT_IMAGE (${ACTUAL_SIZE} bytes)"

# Show file info
file "$OUTPUT_IMAGE" 2>/dev/null || true

print_success "UTM-compatible bootable image created successfully!"
echo
echo "==== UTM Setup Instructions ===="
echo "1. Open UTM and create a new virtual machine"
echo "2. Choose 'Virtualize' (not Emulate)"  
echo "3. Select 'Other' operating system"
echo "4. Configure the VM with these settings:"
echo "   - Architecture: x86_64"
echo "   - Memory: 256MB or higher"
echo "   - CPU Cores: 1-2"
echo "   - Boot: BIOS (not UEFI)"
echo "5. In storage settings:"
echo "   - Remove any existing drives"
echo "   - Click 'New Drive'"
echo "   - Choose 'Import' and select: $(pwd)/$OUTPUT_IMAGE"
echo "   - Set Interface to 'IDE'"
echo "   - Check 'Removable'"
echo "6. In Display settings:"
echo "   - Choose 'Console Only' or 'VGA'"
echo "7. Save and start the VM"
echo
print_status "The VM should now boot RustOS instead of showing 'booting from rom'"
