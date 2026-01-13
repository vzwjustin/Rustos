#!/bin/bash
echo "========================================="
echo "Testing RustOS Bootable Image"
echo "========================================="
echo ""
echo "Image info:"
ls -lh rustos-stable.img
file rustos-stable.img
echo ""
echo "Starting QEMU with correct parameters..."
echo "Command: qemu-system-x86_64 -drive file=rustos-stable.img,format=raw,if=ide"
echo ""
qemu-system-x86_64 \
  -drive file=rustos-stable.img,format=raw,if=ide \
  -m 512M \
  -serial stdio \
  -display cocoa \
  -cpu qemu64
