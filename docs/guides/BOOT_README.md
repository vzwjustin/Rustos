# RustOS Boot Instructions

## The .img file IS bootable and recognized by QEMU

To boot rustos.img:

```bash
qemu-system-x86_64 -drive format=raw,file=rustos.img -m 512M -serial stdio
```

Or use the provided script:
```bash
./run_rustos.sh
```

## Status

✅ Image has valid MBR boot sector
✅ QEMU recognizes it as bootable device  
✅ Bootloader loads successfully
✅ Kernel starts execution
⚠️  Kernel triple-faults during complex initialization (loops at 'Desktop selection ready')

## Available Images

- **rustos.img** (423KB) - Bootable but triple faults
- **rustos-release.img** (58KB) - Bootable but triple faults  
- **rustos-debug.img** (423KB) - Bootable but triple faults

All images boot correctly through the bootloader and start the kernel.
The triple fault is a kernel initialization bug, not a boot device issue.

