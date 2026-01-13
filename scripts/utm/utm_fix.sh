#!/bin/bash
# UTM Boot Fix Script
# This helps fix the "Booting from ROM..." issue

echo "=== UTM Boot Fix Guide ==="
echo ""
echo "PROBLEM: 'Booting from ROM...' means UTM is ignoring your kernel"
echo "SOLUTION: Fix the raw kernel boot configuration"
echo ""

echo "STEP-BY-STEP FIX:"
echo ""

echo "1. In UTM, edit your VM settings"
echo "2. Go to 'Boot' or 'QEMU' tab"
echo "3. Find 'Use raw kernel and initrd' - make sure it's ENABLED"
echo "4. Set Kernel Path to:"
echo "   /Users/justin/Downloads/Rustos-main/target/x86_64-rustos/debug/bootimage-rustos.bin"
echo "5. Leave Kernel Args empty"
echo "6. Leave Initial RAM Disk empty"
echo ""

echo "ALTERNATIVE METHOD - Use Boot Device:"
echo "1. Create a new VM with 'Emulate' -> 'Other'"
echo "2. In Boot section, set:"
echo "   - Boot Device: Disk"
echo "   - Disk Format: Raw"
echo "   - Disk Interface: IDE"
echo "3. Point the disk to your kernel image"
echo ""

echo "QEMU DIRECT TEST (to verify kernel works):"
echo "This bypasses UTM issues:"
echo ""

# Kill any existing QEMU
pkill -f qemu-system-x86_64 2>/dev/null || true

# Direct test with proper kernel boot
qemu-system-x86_64 \
    -kernel target/x86_64-rustos/debug/bootimage-rustos.bin \
    -m 256M \
    -machine q35 \
    -cpu Nehalem \
    -display cocoa \
    -serial stdio \
    -append "console=ttyS0" \
    -no-reboot &
    
QEMU_PID=$!
echo "QEMU PID: $QEMU_PID"
echo "Waiting 10 seconds for kernel output..."
sleep 10
kill $QEMU_PID 2>/dev/null
wait $QEMU_PID 2>/dev/null

echo ""
echo "If you saw kernel messages above, your kernel works!"
echo "The issue is with UTM configuration, not your kernel."
