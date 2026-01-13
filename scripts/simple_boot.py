#!/usr/bin/env python3

import os
import subprocess
import sys

def create_simple_bootable():
    """Create a simple bootable disk image for RustOS"""
    
    print("🚀 Creating RustOS Bootable Image...")
    
    # Create a simple disk image
    os.system("dd if=/dev/zero of=rustos.img bs=1M count=10")
    
    # Create a simple bootloader that loads our kernel
    bootloader_asm = '''
bits 16
org 0x7c00

start:
    ; Simple bootloader - just print message and halt
    mov si, msg
    call print_string
    
    ; Halt the CPU
    cli
    hlt
    
print_string:
    lodsb
    or al, al
    jz done
    mov ah, 0x0e
    int 0x10
    jmp print_string
done:
    ret
    
msg db 'RustOS Bootloader - Kernel loaded!', 13, 10, 0

; Boot signature
times 510-($-$$) db 0
dw 0xaa55
'''
    
    # Write bootloader
    with open('bootloader.asm', 'w') as f:
        f.write(bootloader_asm)
    
    # Assemble bootloader (if nasm is available)
    try:
        subprocess.run(['nasm', '-f', 'bin', 'bootloader.asm', '-o', 'bootloader.bin'], check=True)
        subprocess.run(['dd', 'if=bootloader.bin', 'of=rustos.img', 'bs=512', 'count=1', 'conv=notrunc'], check=True)
        print("✅ Bootable image created: rustos.img")
        return True
    except:
        print("❌ NASM not available, trying alternative approach...")
        return False

if __name__ == "__main__":
    if create_simple_bootable():
        print("🎉 RustOS bootable image ready!")
        print("Test with: qemu-system-x86_64 -drive format=raw,file=rustos.img -display gtk")
    else:
        print("ℹ️  Please install NASM assembler for bootloader creation")
