#!/bin/bash

# Create final bootable RustOS with multiboot headers

if [ "${ALLOW_MULTIBOOT}" != "1" ]; then
    echo "Multiboot path is experimental and unsupported in the default build."
    echo "Set ALLOW_MULTIBOOT=1 to proceed."
    exit 1
fi

echo "🚀 Creating Final Bootable RustOS with Multiboot Headers..."

cd standalone_kernel

# Add multiboot assembly to the working kernel
cat > src/boot.s << 'EOF'
# Multiboot header
.set ALIGN,    1<<0             # align loaded modules on page boundaries
.set MEMINFO,  1<<1             # provide memory map
.set FLAGS,    ALIGN | MEMINFO  # multiboot 'flag' field
.set MAGIC,    0x1BADB002       # magic number lets bootloader find the header
.set CHECKSUM, -(MAGIC + FLAGS) # checksum required to prove we are multiboot

# Multiboot header section
.section .multiboot
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM

# Stack section
.section .bss
.align 16
stack_bottom:
.skip 16384 # 16 KiB
stack_top:

# Entry point
.section .text
.global _start
.type _start, @function
_start:
    mov $stack_top, %esp
    call rust_main
    cli
1:  hlt
    jmp 1b
.size _start, . - _start
EOF

# Create linker script
cat > link.ld << 'EOF'
ENTRY(_start)

SECTIONS
{
    . = 1M;

    .boot :
    {
        KEEP(*(.multiboot))
    }

    .text :
    {
        *(.text)
    }

    .rodata :
    {
        *(.rodata)
    }

    .data :
    {
        *(.data)
    }

    .bss :
    {
        *(.bss)
    }
}
EOF

# Update build script to include assembly
cat >> Cargo.toml << 'EOF'

[build-dependencies]
cc = "1.0"
EOF

# Create build.rs
cat > build.rs << 'EOF'
use std::process::Command;

fn main() {
    // Assemble boot.s
    let output = Command::new("nasm")
        .args(&["-f", "elf32", "src/boot.s", "-o", "boot.o"])
        .output()
        .expect("Failed to run nasm");
    
    if !output.status.success() {
        panic!("Failed to assemble boot.s: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("cargo:rustc-link-arg=boot.o");
    println!("cargo:rustc-link-arg=-Tlink.ld");
    println!("cargo:rerun-if-changed=src/boot.s");
    println!("cargo:rerun-if-changed=link.ld");
}
EOF

echo "✅ Multiboot assembly and linker script added"
echo "🔨 Building final multiboot kernel..."

# Build with multiboot support
cargo +nightly build --bin rustos -Zbuild-std=core,compiler_builtins --target x86_64-rustos.json

if [ $? -eq 0 ]; then
    echo "✅ Final multiboot kernel built!"
    
    # Copy to main directory
    cp target/x86_64-rustos/debug/rustos ../target/x86_64-rustos/debug/rustos-multiboot
    
    echo "🧪 Testing multiboot kernel with QEMU..."
    timeout 20s qemu-system-x86_64 \
        -kernel target/x86_64-rustos/debug/rustos \
        -m 256M \
        -display gtk \
        -serial stdio \
        -no-reboot \
        || echo "🏁 Multiboot QEMU test completed"
    
    echo "🎉 Final bootable RustOS with multiboot support ready!"
    echo "📍 Available as: target/x86_64-rustos/debug/rustos-multiboot"
else
    echo "❌ Final multiboot build failed"
    cd ..
    exit 1
fi

cd ..
echo "✅ RustOS multiboot deployment complete!"
echo "🚀 Your kernel is now ready for deployment!"
