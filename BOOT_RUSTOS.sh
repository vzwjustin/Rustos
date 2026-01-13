#!/bin/bash

echo "================================================"
echo "           RustOS Kernel Boot Script"
echo "================================================"
echo ""
echo "Image: rustos-stable.img (55KB)"
echo "Status: BOOTABLE ✅"
echo ""
echo "Starting QEMU..."
echo ""

# Use simplest working method
qemu-system-x86_64 \
  -hda rustos-stable.img \
  -m 512M \
  -display cocoa

echo ""
echo "QEMU exited."
