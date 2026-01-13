# User/Kernel Mode Switching - Quick Reference

## What Was Implemented

Complete Ring 0 (kernel) to Ring 3 (user) privilege switching for x86_64, enabling userspace program execution.

## Files Created

1. **src/usermode.rs** (383 lines)
   - `switch_to_user_mode()` - Ring 0→3 via IRETQ
   - `UserContext` - Complete CPU state management
   - `is_valid_user_address()` - Address validation
   - `return_to_kernel()` - Segment restoration

2. **src/syscall_fast.rs** (320 lines)
   - `init()` - Configure MSRs (STAR, LSTAR, FMASK, EFER)
   - `syscall_entry()` - Fast syscall entry point
   - `syscall_handler_wrapper()` - Register extraction
   - `is_supported()` - Check CPUID for SYSCALL support

3. **src/usermode_test.rs** (343 lines)
   - `test_privilege_levels()` - CPL validation tests
   - `test_address_validation()` - User space boundary checks
   - `test_user_context()` - Context setup validation
   - `demonstrate_user_mode_setup()` - Example setup

## Files Modified

1. **src/gdt.rs**
   - Enhanced `set_kernel_stack()` to set TSS.RSP0
   - Enables automatic stack switching on Ring 3→0

2. **src/syscall_handler.rs**
   - Enhanced `syscall_0x80_handler()` with register extraction
   - Full System V AMD64 ABI support
   - Return value handling in RAX

3. **src/main.rs**
   - Added module declarations (usermode, syscall_fast, usermode_test)
   - Added fast syscall initialization in kernel boot

## How to Use

### Enter User Mode

```rust
use crate::usermode::{UserContext, switch_to_user_mode};
use crate::gdt;

// 1. Set kernel stack in TSS (for interrupt returns)
gdt::set_kernel_stack(VirtAddr::new(kernel_stack_top));

// 2. Create user context
let mut context = UserContext::new();
context.set_entry_point(0x400000);  // User code address
context.set_stack_pointer(0x500000); // User stack address

// 3. Switch to user mode (never returns)
unsafe { context.restore_and_switch(); }

// OR use the simple interface:
unsafe {
    switch_to_user_mode(0x400000, 0x500000);
}
```

### Make Syscalls (from user mode)

**INT 0x80** (traditional):
```asm
mov rax, 1          ; sys_write
mov rdi, 1          ; fd (stdout)
mov rsi, buffer     ; message
mov rdx, 13         ; length
int 0x80            ; syscall
; Result in RAX
```

**SYSCALL** (modern, faster):
```asm
mov rax, 1          ; sys_write
mov rdi, 1          ; fd
mov rsi, buffer     ; message
mov rdx, 13         ; length
syscall             ; fast syscall
; Result in RAX
```

### Validate User Addresses

```rust
use crate::usermode::is_valid_user_address;

let addr = 0x400000u64;
let size = 4096;

if is_valid_user_address(addr, size) {
    // Safe to use
} else {
    // Invalid - reject
}
```

## Register Conventions

### Syscall Arguments (System V AMD64 ABI)

| Register | Purpose |
|----------|---------|
| RAX | Syscall number (input), Return value (output) |
| RDI | Argument 1 |
| RSI | Argument 2 |
| RDX | Argument 3 |
| R10 | Argument 4 (note: not RCX) |
| R8  | Argument 5 |
| R9  | Argument 6 |

### SYSCALL/SYSRET Specifics

| Register | SYSCALL | SYSRET |
|----------|---------|--------|
| RCX | Saved user RIP | Restored to RIP |
| R11 | Saved user RFLAGS | Restored to RFLAGS |
| CS | Loaded from STAR[47:32] | Loaded from STAR[63:48]+16 |
| SS | Loaded from STAR[47:32]+8 | Loaded from STAR[63:48]+8 |

## Memory Layout

```
0x0000_0000_0000_0000 - 0x0000_0000_0000_0FFF : Null page (unmapped)
0x0000_0000_0000_1000 - 0x7FFF_FFFF_FFFF_FFFF : User space
0x8000_0000_0000_0000 - 0xFFFF_7FFF_FFFF_FFFF : Non-canonical (invalid)
0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF : Kernel space
```

## Privilege Levels

| Ring | Name | Description |
|------|------|-------------|
| 0 | Kernel | Full privileges, I/O access, all instructions |
| 1-2 | Unused | Reserved for device drivers (not used) |
| 3 | User | Restricted privileges, no I/O, no privileged instructions |

## RFLAGS Configuration

User mode RFLAGS (0x202):
- Bit 1: Reserved (always 1)
- Bit 9: IF = 1 (interrupts enabled)
- Bits 12-13: IOPL = 0 (no I/O privilege)
- Other bits: cleared

## Security Features

✅ Address validation (user space only)
✅ Privilege enforcement (RPL=3 for user segments)
✅ IOPL=0 (no I/O port access)
✅ Separate kernel stack (TSS.RSP0)
✅ Null page protection
✅ Kernel space protection

## Testing

```rust
// Run all tests
crate::usermode_test::run_all_tests();

// Individual tests
crate::usermode_test::test_privilege_levels();
crate::usermode_test::demonstrate_user_mode_setup();
```

## MSR Configuration (SYSCALL/SYSRET)

| MSR | Address | Purpose |
|-----|---------|---------|
| STAR | 0xC000_0081 | Segment selectors |
| LSTAR | 0xC000_0082 | Syscall entry point |
| FMASK | 0xC000_0084 | RFLAGS mask |
| EFER | 0xC000_0080 | Enable SCE bit |

## Common Patterns

### Creating User Process

```rust
// 1. Allocate memory
let code_pages = allocate_user_pages(code_size);
let stack_pages = allocate_user_pages(stack_size);

// 2. Load program
load_elf(code_pages, elf_binary);

// 3. Set TSS
gdt::set_kernel_stack(kernel_stack);

// 4. Switch
let mut ctx = UserContext::new();
ctx.set_entry_point(code_pages.start);
ctx.set_stack_pointer(stack_pages.end);
unsafe { ctx.restore_and_switch(); }
```

### Handling Syscalls

Automatically handled by:
- `interrupts::init()` - Sets up INT 0x80 in IDT
- `syscall_fast::init()` - Sets up SYSCALL/SYSRET
- `syscall_handler::dispatch_syscall()` - Routes to implementations

## Performance

**INT 0x80**: ~200 cycles
- IDT lookup
- Privilege check
- Stack switch via TSS
- Full state save

**SYSCALL/SYSRET**: ~50-80 cycles
- Direct MSR-based entry
- No IDT lookup
- Minimal state save
- Hardware-optimized

## Next Steps

To execute actual user programs:

1. ✅ User mode switching (DONE)
2. ⏳ Virtual memory with page tables
3. ⏳ User space allocator
4. ⏳ ELF loader integration
5. ⏳ Process/task management
6. ⏳ Signal handling

## Status

**Implementation**: ✅ COMPLETE
**Compilation**: ✅ SUCCESS
**Testing**: ✅ READY
**Integration**: ✅ READY

The kernel can now switch to user mode and handle syscalls. Integration with process manager and ELF loader will enable execution of real userspace programs.
