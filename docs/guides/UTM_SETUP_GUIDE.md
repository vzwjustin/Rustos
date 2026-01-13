# RustOS UTM Setup Guide

This guide shows you how to boot RustOS in UTM on macOS. We've created two different approaches to solve the "booting from ROM" issue.

## Quick Fix Summary

The "booting from ROM" error occurs when UTM can't find a bootable operating system. RustOS needs proper boot configuration to work with UTM.

## Option 1: MBR Disk Image (Recommended)

### Step 1: Create the bootable disk image
```bash
# Make sure you have built RustOS first
./scripts/build_simple.sh --build

# Create UTM-compatible disk image
./create_utm_bootable.sh
```

### Step 2: Configure UTM
1. **Open UTM** and click "Create a New Virtual Machine"
2. **Choose "Virtualize"** (not Emulate)
3. **Select "Other"** as the operating system
4. **Configure System Settings:**
   - Architecture: `x86_64`
   - Memory: `256 MB` (minimum) or `512 MB` (recommended)
   - CPU Cores: `1-2`

### Step 3: Boot Configuration
1. In **"Boot"** settings:
   - Boot order: Make sure hard disk is first
   - **IMPORTANT**: Select "BIOS" (not UEFI)
   
### Step 4: Storage Setup
1. In **"Drives"** settings:
   - Remove any existing drives
   - Click **"New Drive"**
   - Choose **"Import Drive"**
   - Select: `/path/to/rustos_utm.img` (created by the script)
   - Interface: **"IDE"** 
   - ✅ Check **"Removable"** if available

### Step 5: Display Settings
1. In **"Display"** settings:
   - Choose **"Console Only"** or **"VGA"**
   - Resolution: Any standard resolution

### Step 6: Network (Optional)
1. In **"Network"** settings:
   - Network Mode: **"Shared Network"** or **"Bridged"**

### Step 7: Start the VM
- Click **"Save"** and then **"Start"**
- RustOS should boot and display its startup messages

## Option 2: Direct Kernel Boot (Alternative)

If Option 1 doesn't work, try direct kernel booting:

### Step 1: Prepare kernel
```bash
# Build the kernel binary
cargo build --target x86_64-rustos.json --bin rustos
./create_grub_utm_image.sh
```

### Step 2: UTM Configuration
1. Create new VM as above, but in **"Boot"** settings:
   - Enable **"Boot from kernel image"**
   - Kernel: Point to `rustos_kernel_utm`
   - Kernel arguments: `console=ttyS0`
   - Initial RAM disk: Leave empty

## Troubleshooting

### Still seeing "booting from ROM"?

1. **Check Boot Order:**
   - Ensure the disk drive is first in boot order
   - Make sure it's set to BIOS, not UEFI

2. **Verify Image:**
   ```bash
   # Check if the image was created correctly
   ls -la rustos_utm.img
   file rustos_utm.img  # Should show "DOS/MBR boot sector"
   ```

3. **Alternative Approach:**
   ```bash
   # Try using the original bootimage directly
   cp target/x86_64-rustos/debug/bootimage-rustos.bin rustos_direct.img
   ```
   Then use `rustos_direct.img` as the disk in UTM.

4. **Enable Debug Output:**
   - In UTM VM settings, go to "Serial"
   - Enable serial console to see boot messages

### Common Issues

- **"No bootable device"**: Make sure the drive interface is set to IDE
- **Black screen**: Try "Console Only" display mode
- **VM won't start**: Reduce memory to 128MB and try again
- **Boot loops**: Check that you're using BIOS boot, not UEFI

### Success Indicators

When RustOS boots successfully, you should see:
```
RustOS - Multiboot Kernel Started!
======================================
Boot information received successfully
System ready for operation
```

## Files Created

After running the setup scripts, you'll have:

- `rustos_utm.img` - MBR-formatted disk image (50MB)
- `rustos_kernel_utm` - Raw kernel binary for direct boot
- `target/x86_64-rustos/debug/bootimage-rustos.bin` - Original bootloader image

## Alternative: Test in QEMU First

If UTM still has issues, test RustOS in QEMU first:

```bash
# Test with QEMU
qemu-system-x86_64 -drive format=raw,file=rustos_utm.img -m 256M -display gtk

# Or use the build script
./scripts/build_simple.sh --run
```

If it works in QEMU but not UTM, the issue is UTM-specific configuration.

## Need Help?

1. Check UTM logs in Console.app for error messages
2. Try different UTM versions (some work better than others)
3. Verify your Mac supports virtualization features
4. Make sure UTM has necessary permissions in System Preferences

The most common fix is ensuring BIOS boot mode and IDE interface for the disk drive.
