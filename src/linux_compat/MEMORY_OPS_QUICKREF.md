# Memory Operations Quick Reference

Quick reference for using memory operations in RustOS.

## Virtual Memory Allocation

### Allocate Anonymous Memory
```rust
use crate::linux_compat::memory_ops::{mmap, prot, map};

let addr = mmap(
    core::ptr::null_mut(),
    4096,
    prot::PROT_READ | prot::PROT_WRITE,
    map::MAP_PRIVATE | map::MAP_ANONYMOUS,
    -1, 0
)?;
```

### Change Permissions
```rust
use crate::linux_compat::memory_ops::{mprotect, prot};

mprotect(addr, size, prot::PROT_READ)?;
```

### Unmap Memory
```rust
use crate::linux_compat::memory_ops::munmap;

munmap(addr, size)?;
```

## See Also
- `/src/linux_compat/memory_ops.rs` - Full implementation
- `/MEMORY_OPS_COMPLETE.md` - Comprehensive documentation
