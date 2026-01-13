# Multiboot header
.set ALIGN,    1<<0             # align loaded modules on page boundaries
.set MEMINFO,  1<<1             # provide memory map
.set VIDEO,    1<<2             # request video mode
.set FLAGS,    ALIGN | MEMINFO | VIDEO  # multiboot 'flag' field
.set MAGIC,    0x1BADB002       # magic number lets bootloader find the header
.set CHECKSUM, -(MAGIC + FLAGS) # checksum required to prove we are multiboot

# Multiboot header section
.section .multiboot
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM
.long 0  # header_addr (unused for ELF)
.long 0  # load_addr
.long 0  # load_end_addr
.long 0  # bss_end_addr
.long 0  # entry_addr
.long 0  # mode_type (0 = linear framebuffer)
.long 1024  # width
.long 768   # height
.long 32    # depth (bits per pixel)

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
