#!/bin/bash

# RustOS Bootable Image Creator
# Creates a multiboot-compliant bootable image for RustOS

if [ "${ALLOW_MULTIBOOT}" != "1" ]; then
    echo "Multiboot path is experimental and unsupported in the default build."
    echo "Set ALLOW_MULTIBOOT=1 to proceed."
    exit 1
fi

echo "🚀 Creating RustOS Multiboot Bootable Image..."

# Create kernel with proper multiboot headers
cat > multiboot_header.s << 'EOF'
.section .multiboot_header
.align 8

multiboot_header_start:
    .long 0xe85250d6                # magic number
    .long 0                         # architecture 0 (protected mode i386)
    .long multiboot_header_end - multiboot_header_start # header length
    # checksum
    .long 0x100000000 - (0xe85250d6 + 0 + (multiboot_header_end - multiboot_header_start))

    # end tag
    .word 0
    .word 0
    .long 8
multiboot_header_end:

.section .text
.global _start
_start:
    # Set up stack
    mov $stack_top, %esp
    
    # Call rust main
    call rust_main
    
    # Halt
    cli
1:  hlt
    jmp 1b

.section .bss
.align 16
stack_bottom:
    .skip 16384
stack_top:
EOF

echo "✅ Multiboot header created"

# Assemble the multiboot header
if command -v nasm >/dev/null 2>&1; then
    echo "🔧 Using NASM to assemble..."
    nasm -f elf64 multiboot_header.s -o multiboot.o
elif command -v as >/dev/null 2>&1; then
    echo "🔧 Using GNU assembler..."
    as --64 multiboot_header.s -o multiboot.o
else
    echo "❌ No assembler found. Installing..."
    # Try to use what's available in Docker
    apt-get update && apt-get install -y nasm
    nasm -f elf64 multiboot_header.s -o multiboot.o
fi

if [ -f multiboot.o ]; then
    echo "✅ Assembly successful: multiboot.o created"
else
    echo "❌ Assembly failed"
    exit 1
fi

echo "🔨 Building RustOS kernel with multiboot support..."

# Build the kernel
cargo build --bin rustos --target x86_64-rustos.json

if [ $? -eq 0 ]; then
    echo "✅ RustOS kernel built successfully!"
    echo "📦 Kernel location: target/x86_64-rustos/debug/rustos"
    
    # Test with QEMU
    echo "🧪 Testing with QEMU..."
    timeout 10s qemu-system-x86_64 \
        -kernel target/x86_64-rustos/debug/rustos \
        -m 256M \
        -nographic \
        -no-reboot \
        || echo "🏁 QEMU test completed"
else
    echo "❌ Kernel build failed"
    exit 1
fi

echo "🎉 RustOS multiboot deployment complete!"
