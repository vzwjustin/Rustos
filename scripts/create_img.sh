#!/bin/bash
# Create .img format bootable image for RustOS

set -e

echo "🔨 Building RustOS kernel image..."

# Build the bootable kernel
cargo bootimage --target x86_64-rustos.json

# Copy to .img format for easier booting
cp target/x86_64-rustos/debug/bootimage-rustos.bin rustos.img

echo "✅ RustOS image created: rustos.img"
echo "📏 Size: $(ls -lh rustos.img | awk '{print $5}')"
echo ""
echo "🚀 To run in QEMU:"
echo "   qemu-system-x86_64 -drive file=rustos.img,format=raw,index=0,media=disk"
echo ""
echo "💾 To run in VirtualBox/VMware:"
echo "   Use rustos.img as a hard disk image"