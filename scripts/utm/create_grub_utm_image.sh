#!/bin/bash

# Create GRUB-based UTM bootable image for RustOS
# This creates an ISO image that UTM can boot from more reliably

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
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

# Check dependencies
check_dependency() {
    if ! command -v "$1" >/dev/null 2>&1; then
        print_error "$1 is required but not installed"
        if [[ "$OSTYPE" == "darwin"* ]]; then
            print_status "Install with: brew install $2"
        fi
        return 1
    fi
}

print_status "Creating GRUB-based ISO for UTM..."

# Check for required tools
if ! check_dependency "grub-mkrescue" "grub"; then
    print_status "Attempting to build kernel directly instead..."
    KERNEL_PATH="target/x86_64-rustos/debug/rustos"
    
    if [ ! -f "$KERNEL_PATH" ]; then
        print_error "Kernel binary not found at $KERNEL_PATH"
        print_status "Building kernel first..."
        cargo build --target x86_64-rustos.json --bin rustos
    fi
    
    if [ -f "$KERNEL_PATH" ]; then
        print_success "Raw kernel binary available at: $KERNEL_PATH"
        print_status "For UTM, you can try booting this directly:"
        print_status "1. In UTM, create a new VM"
        print_status "2. Choose 'Virtualize' -> 'Other'"
        print_status "3. In boot settings, enable 'Boot from kernel image'"
        print_status "4. Point to: $(pwd)/$KERNEL_PATH"
        print_status "5. Set kernel arguments to: console=ttyS0"
        
        # Create a simple disk image for the kernel
        print_status "Creating kernel image for direct boot..."
        cp "$KERNEL_PATH" "rustos_kernel_utm"
        print_success "Kernel ready for UTM at: rustos_kernel_utm"
        
        exit 0
    fi
    
    exit 1
fi

# Create ISO directory structure
print_status "Creating ISO directory structure..."
rm -rf isodir
mkdir -p isodir/boot/grub

# Copy kernel
KERNEL_PATH="target/x86_64-rustos/debug/rustos"
if [ ! -f "$KERNEL_PATH" ]; then
    print_status "Building kernel..."
    cargo build --target x86_64-rustos.json --bin rustos
fi

cp "$KERNEL_PATH" isodir/boot/rustos

# Create GRUB config
print_status "Creating GRUB configuration..."
cat > isodir/boot/grub/grub.cfg << 'EOF'
set timeout=0
set default=0

menuentry "RustOS" {
    multiboot2 /boot/rustos
    boot
}
EOF

# Create the ISO
print_status "Creating bootable ISO..."
grub-mkrescue -o rustos_utm.iso isodir

if [ -f "rustos_utm.iso" ]; then
    ACTUAL_SIZE=$(ls -la rustos_utm.iso | awk '{print $5}')
    print_success "Bootable ISO created: rustos_utm.iso (${ACTUAL_SIZE} bytes)"
    
    echo
    echo "==== UTM ISO Setup Instructions ===="
    echo "1. Open UTM and create a new virtual machine"
    echo "2. Choose 'Virtualize' -> 'Other'"
    echo "3. Configure VM settings:"
    echo "   - Architecture: x86_64"
    echo "   - Memory: 256MB+"
    echo "   - CPU: 1-2 cores"
    echo "4. In storage settings:"
    echo "   - Add 'CD/DVD Drive'"
    echo "   - Import: $(pwd)/rustos_utm.iso"
    echo "5. In boot order:"
    echo "   - Set CD/DVD as first boot device"
    echo "6. Start the VM"
    echo
    print_success "RustOS should now boot from the ISO!"
else
    print_error "Failed to create ISO"
    exit 1
fi
