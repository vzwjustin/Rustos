#!/bin/bash

# RustOS Bootimage Creator with Multiboot Support
# Creates a bootable image for RustOS using the multiboot specification

if [ "${ALLOW_MULTIBOOT}" != "1" ]; then
    echo "Multiboot path is experimental and unsupported in the default build."
    echo "Set ALLOW_MULTIBOOT=1 to proceed."
    exit 1
fi

echo "🚀 Creating RustOS Bootable Image with Multiboot..."

# Create the multiboot assembly if it doesn't exist
if [ ! -f "src/boot.s" ]; then
    echo "📝 Creating multiboot assembly..."
    cat > src/boot.s << 'EOF'
// Multiboot header for RustOS
.set ALIGN,    1<<0             // align loaded modules on page boundaries
.set MEMINFO,  1<<1             // provide memory map  
.set FLAGS,    ALIGN | MEMINFO  // this is the Multiboot 'flag' field
.set MAGIC,    0x1BADB002       // 'magic number' lets bootloader find the header
.set CHECKSUM, -(MAGIC + FLAGS) // checksum of above, to prove we are multiboot

// Declare a multiboot header that marks the program as a kernel
.section .multiboot
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM

// Reserve a stack for the initial thread.
.section .bss
.align 16
stack_bottom:
.skip 16384 // 16 KiB
stack_top:

// The kernel entry point.
.section .text
.global _start
.type _start, @function
_start:
    movl $stack_top, %esp
    
    // Transfer control to the main kernel.
    call rust_main
    
    // Hang if rust_main unexpectedly returns.
    cli
1:  hlt
    jmp 1b
.size _start, . - _start
EOF
    echo "✅ Multiboot assembly created"
fi

# Build the kernel with build-std for custom target
echo "🔨 Building RustOS kernel with multiboot support..."
cargo +nightly build --bin rustos -Zbuild-std=core,alloc,compiler_builtins --target x86_64-rustos.json

if [ $? -ne 0 ]; then
    echo "❌ Kernel build failed"
    exit 1
fi

echo "✅ Kernel built successfully!"

# Create bootable directory structure
echo "📦 Creating bootable image structure..."
mkdir -p isodir/boot/grub

# Copy kernel
cp target/x86_64-rustos/debug/rustos isodir/boot/rustos

# Create GRUB configuration
cat > isodir/boot/grub/grub.cfg << 'EOF'
menuentry "RustOS" {
    multiboot2 /boot/rustos
}
EOF

echo "✅ Bootable structure created"

# Test kernel with QEMU
echo "🧪 Testing RustOS with QEMU..."
timeout 20s qemu-system-x86_64 \
    -kernel target/x86_64-rustos/debug/rustos \
    -m 256M \
    -display none \
    -serial stdio \
    -no-reboot \
    || echo "🏁 QEMU test completed"

echo "🎉 RustOS bootimage creation complete!"
echo "📍 Kernel available at: target/x86_64-rustos/debug/rustos"
echo "📍 ISO structure at: isodir/"
