# ELF Loader Quick Reference

## Key Functions

### Load ELF Binary (Simplified)
```rust
use crate::initramfs::load_and_execute_elf;

let binary_data: &[u8] = /* read ELF file */;
let (entry_point, stack_pointer) = load_and_execute_elf(binary_data)?;

// entry_point: u64 - Address to jump to
// stack_pointer: u64 - Initial stack pointer for user mode
```

### Load ELF Binary (Full Page Table Support)
```rust
use crate::initramfs::load_and_execute_elf_with_paging;

let (entry_point, stack_pointer) = load_and_execute_elf_with_paging(
    binary_data,
    &mut mapper,           // Page table mapper
    &mut frame_allocator   // Physical frame allocator
)?;
```

### Load /init from VFS
```rust
use crate::initramfs::start_init;

// Load /init from initramfs
start_init()?;

// Binary is now loaded and ready to execute
```

### Jump to User Mode
```rust
use crate::initramfs::execute_init;

// WARNING: Never returns!
unsafe {
    execute_init(entry_point, stack_pointer);
}
```

## API Overview

| Function | Purpose | Returns | Safe? |
|----------|---------|---------|-------|
| `load_and_execute_elf()` | Load ELF into memory | `(u64, u64)` | Yes |
| `load_and_execute_elf_with_paging()` | Load with page tables | `(u64, u64)` | Yes |
| `start_init()` | Load /init from VFS | `()` | Yes |
| `execute_init()` | Jump to user mode | Never returns | No (unsafe) |

## ELF Validation

Checks performed automatically:
- ✅ Magic number (0x7f 'E' 'L' 'F')
- ✅ ELF class (64-bit)
- ✅ Endianness (little-endian)
- ✅ Version (current)
- ✅ Machine architecture (x86_64)
- ✅ File type (ET_EXEC or ET_DYN)
- ✅ Segment alignment
- ✅ Segment overlap detection
- ✅ Buffer bounds checking

## Memory Layout

```
0x0000_7fff_ffff_0000  ← Stack top (default)
          ↓
       [Stack]
       8 MB, grows downward

0x0000_1000_0000       ← User space start

   [Code segments]
   R-X (read + execute)

   [Data segments]  
   RW- (read + write)

   [BSS segment]
   RW-, zero-initialized
```

## Error Handling

```rust
use crate::initramfs::InitramfsError;

match load_and_execute_elf(binary_data) {
    Ok((entry, stack)) => {
        // Success - binary loaded
    }
    Err(InitramfsError::InvalidFormat) => {
        // ELF validation failed
    }
    Err(InitramfsError::ExtractionFailed) => {
        // Segment loading failed
    }
    Err(_) => {
        // Other error
    }
}
```

## Integration Points

### VFS Integration
```rust
let vfs = crate::vfs::get_vfs();
let inode = vfs.lookup("/init")?;
let mut data = Vec::new();
inode.read(0, &mut data)?;
```

### User Mode Transition
```rust
use crate::usermode;

unsafe {
    usermode::switch_to_user_mode(entry_point, stack_pointer);
}
```

### Syscall Handling
- INT 0x80 handler already wired
- Register convention: RAX, RDI, RSI, RDX, R10, R8, R9
- Return value in RAX
- Dispatches to Linux compatibility layer

## Supported Binary Types

| Type | Description | Supported |
|------|-------------|-----------|
| Static | No dynamic linking | ✅ Yes |
| PIE | Position-independent | ✅ Yes |
| Dynamic | Needs ld-musl | ⚠️ Partial |

## Example Usage

```rust
// Complete flow for executing /init
pub fn boot_to_userspace() -> Result<(), InitramfsError> {
    // 1. Extract initramfs
    initramfs::init_initramfs()?;
    
    // 2. Load /init binary
    initramfs::start_init()?;
    
    // 3. Jump to user mode (never returns)
    // NOTE: This would be done by scheduler/process manager
    // unsafe {
    //     let (entry, stack) = /* get from process context */;
    //     execute_init(entry, stack);
    // }
    
    Ok(())
}
```

## Testing

```bash
# Compile and check
cargo check

# Build kernel
make build

# Test in QEMU
make run
```

## Files Modified

- `/Users/justin/Downloads/Rustos-main/src/initramfs.rs` - ELF loading functions
- `/Users/justin/Downloads/Rustos-main/src/main.rs` - Module declarations

## Dependencies

- `src/elf_loader/` - ELF parser and loader
- `src/usermode.rs` - User mode transitions
- `src/syscall_handler.rs` - Syscall interface
- `src/vfs/` - File system access

## Next Steps

1. Test with real Alpine Linux /init binary
2. Implement dynamic linker support for shared libraries
3. Add process management integration
4. Enable full user mode execution with syscalls
5. Test complete Linux compatibility layer
