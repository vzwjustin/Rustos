# Memory Management System

Production-ready virtual memory management system for RustOS with x86_64 4-level paging support.

## Overview

This module provides comprehensive virtual memory management including:

- **Page Table Management**: x86_64 4-level paging (PML4, PDPT, PD, PT)
- **Virtual Memory Operations**: mmap, munmap, mprotect for POSIX compatibility
- **Heap Management**: brk/sbrk syscalls for dynamic memory allocation
- **Memory Regions**: Track and manage virtual memory mappings
- **Memory Protection**: Read, write, execute permissions with enforcement
- **Copy-on-Write**: Efficient memory sharing between processes

## Architecture

### Module Structure

```
memory_manager/
├── mod.rs              # Public API and module exports
├── page_table.rs       # Page table structures and operations
├── virtual_memory.rs   # Virtual memory manager implementation
├── memory_region.rs    # Memory region tracking and management
└── README.md          # This file
```

### Key Components

#### 1. Page Table Management (`page_table.rs`)

Implements x86_64 4-level paging:

- **PageTable**: Page table wrapper with mapping operations
- **PageTableManager**: Manages multiple page tables and TLB
- **PageTableFlags**: Hardware page table flags (present, writable, user-accessible, no-execute)

#### 2. Virtual Memory Manager (`virtual_memory.rs`)

Core virtual memory operations:

- **VirtualMemoryManager**: Main VM manager with region tracking
- **mmap/munmap**: Map and unmap virtual memory regions
- **mprotect**: Change memory protection on existing mappings
- **brk/sbrk**: Heap management for dynamic allocation
- **Page Fault Handling**: COW and permission fault resolution

#### 3. Memory Regions (`memory_region.rs`)

Track memory mappings:

- **MemoryRegion**: Descriptor for virtual memory region
- **MemoryType**: Anonymous, file-backed, shared, stack, heap, code, data
- **ProtectionFlags**: POSIX-style read/write/execute permissions
- **Region Operations**: Split, merge, overlap detection

## Usage

### Initialization

```rust
use memory_manager::init_virtual_memory;
use x86_64::VirtAddr;

// Initialize virtual memory manager with physical memory offset
let physical_memory_offset = VirtAddr::new(0xFFFF_8000_0000_0000);
init_virtual_memory(physical_memory_offset)?;
```

### Memory Mapping (mmap)

```rust
use memory_manager::api::*;
use memory_manager::{MmapFlags, ProtectionFlags};

// Allocate 4KB anonymous private memory
let size = 4096;
let prot = ProtectionFlags::READ_WRITE;
let flags = MmapFlags::anonymous_private();

let ptr = vm_mmap(0, size, prot, flags)?;
println!("Mapped at: {:p}", ptr);

// Use the memory
unsafe {
    *ptr = 42;
}
```

### Memory Unmapping (munmap)

```rust
// Unmap previously mapped region
let addr = ptr as usize;
vm_munmap(addr, size)?;
```

### Memory Protection (mprotect)

```rust
// Change protection to read-only
let new_prot = ProtectionFlags::READ;
vm_mprotect(addr, size, new_prot)?;
```

### Heap Management (brk/sbrk)

```rust
// Query current program break
let current_brk = vm_brk(0)?;

// Extend heap by 8KB
let new_brk = current_brk + 8192;
vm_brk(new_brk)?;

// Alternative: sbrk interface
let old_brk = vm_sbrk(8192)?;  // Increase by 8KB
let restored = vm_sbrk(-4096)?; // Decrease by 4KB
```

### Page Table Operations

```rust
use x86_64::{VirtAddr, PhysAddr};
use memory_manager::PageTableFlags;

// Create a new page table
let mut page_table = page_table_create()?;

// Map a virtual page to physical frame
let virt = VirtAddr::new(0x1000);
let phys = PhysAddr::new(0x10000);
let flags = PageTableFlags::PRESENT
    | PageTableFlags::WRITABLE
    | PageTableFlags::USER_ACCESSIBLE;

page_table_map(&mut page_table, virt, phys, flags)?;

// Translate virtual to physical
if let Some(phys_addr) = page_table_translate(&page_table, virt) {
    println!("Virtual {:?} -> Physical {:?}", virt, phys_addr);
}

// Unmap the page
page_table_unmap(&mut page_table, virt)?;
```

## Memory Layout

### User Space Layout (x86_64)

```
0x0000_0000_0000      ┌─────────────────────────────┐
                      │ NULL page (unmapped)         │
0x0000_1000_0000      ├─────────────────────────────┤
                      │ Heap (brk/sbrk)             │ 1GB
0x0000_4000_0000      ├─────────────────────────────┤
                      │ mmap allocations            │ 1GB
0x0000_8000_0000      ├─────────────────────────────┤
                      │ Stack (grows down)          │
                      └─────────────────────────────┘
```

### Kernel Space Layout

```
0xFFFF_8000_0000_0000 ┌─────────────────────────────┐
                      │ Physical memory mapping     │
                      │ (kernel heap, page tables)  │
                      └─────────────────────────────┘
```

## API Reference

### Public Functions

#### `vm_mmap(addr, length, prot, flags) -> Result<*mut u8>`
Map virtual memory region.

**Parameters:**
- `addr`: Hint for virtual address (0 for kernel to choose)
- `length`: Size in bytes (rounded up to page size)
- `prot`: Protection flags (READ, WRITE, EXECUTE)
- `flags`: Mapping flags (anonymous, shared, fixed, etc.)

**Returns:** Virtual address of mapped region

#### `vm_munmap(addr, length) -> Result<()>`
Unmap virtual memory region.

**Parameters:**
- `addr`: Start address (must be page-aligned)
- `length`: Size in bytes (rounded up to page size)

#### `vm_mprotect(addr, length, prot) -> Result<()>`
Change memory protection.

**Parameters:**
- `addr`: Start address (must be page-aligned)
- `length`: Size in bytes (rounded up to page size)
- `prot`: New protection flags

#### `vm_brk(addr) -> Result<usize>`
Change program break.

**Parameters:**
- `addr`: New break address (0 to query current)

**Returns:** Current program break address

#### `vm_sbrk(increment) -> Result<usize>`
Increment program break.

**Parameters:**
- `increment`: Bytes to add (positive) or remove (negative)

**Returns:** Previous program break address

#### `page_table_create() -> Result<PageTable>`
Create a new page table.

#### `page_table_map(table, virt, phys, flags) -> Result<()>`
Map virtual address to physical address.

#### `page_table_unmap(table, virt) -> Result<()>`
Unmap virtual address.

#### `page_table_translate(table, virt) -> Option<PhysAddr>`
Translate virtual to physical address.

### Memory Types

- **Anonymous**: Not backed by file, zero-initialized
- **FileBacked**: Backed by file descriptor
- **Shared**: Shared between processes
- **Stack**: Thread stack memory
- **Heap**: Process heap (brk/sbrk)
- **Code**: Executable code section
- **Data**: Data section
- **Device**: Memory-mapped I/O

### Protection Flags

- `ProtectionFlags::NONE`: No access
- `ProtectionFlags::READ`: Read permission
- `ProtectionFlags::WRITE`: Write permission
- `ProtectionFlags::EXECUTE`: Execute permission
- `ProtectionFlags::READ_WRITE`: Read and write
- `ProtectionFlags::READ_EXEC`: Read and execute
- `ProtectionFlags::READ_WRITE_EXEC`: All permissions

### Mapping Flags

- `fixed`: Use exact address (MAP_FIXED)
- `shared`: Shared mapping (MAP_SHARED)
- `private`: Private copy-on-write (MAP_PRIVATE)
- `anonymous`: Not backed by file (MAP_ANONYMOUS)

## Features

### Copy-on-Write (COW)

Efficient memory sharing with lazy copying:

```rust
// Mark region as copy-on-write
region.set_copy_on_write(true);

// Write fault triggers page copy
// Original page shared until write occurs
```

### Memory Statistics

```rust
let stats = get_memory_stats()?;
println!("Total allocated: {} bytes", stats.total_allocated);
println!("Active regions: {}", stats.region_count);
println!("Mapped pages: {}", stats.mapped_pages);
```

### Thread Safety

All operations are thread-safe using `spin::Mutex` for synchronization.

### Page Fault Handling

Automatic handling of:
- Copy-on-write faults
- Permission violations
- Access to unmapped memory

## Error Handling

All operations return `VmResult<T>` with detailed error types:

- `VmError::InvalidAddress`: Invalid virtual address
- `VmError::InvalidSize`: Invalid size parameter
- `VmError::OutOfMemory`: Physical memory exhausted
- `VmError::PermissionDenied`: Access violation
- `VmError::RegionNotFound`: No region at address
- `VmError::AlreadyMapped`: Region already exists
- `VmError::NotAligned`: Address not page-aligned

## Integration with Process Management

```rust
use process::Process;
use memory_manager::api::*;

// Allocate stack for new thread
let stack_size = 1024 * 1024; // 1MB
let stack_ptr = vm_mmap(
    0,
    stack_size,
    ProtectionFlags::READ_WRITE,
    MmapFlags::anonymous_private()
)?;

// Create process with custom memory layout
let mut process = Process::new();
process.set_stack(stack_ptr, stack_size);
```

## Performance Considerations

### Optimization Strategies

1. **Page Allocation**: Batch allocate pages when possible
2. **TLB Management**: Minimize TLB flushes by batching operations
3. **Region Merging**: Automatically merge contiguous compatible regions
4. **Lazy Allocation**: Defer physical allocation until first access

### Memory Overhead

- Page table: ~4KB per table level
- Region descriptor: ~128 bytes per region
- Global overhead: <1MB for typical workload

## Testing

Run memory manager tests:

```bash
cargo test -p rustos --lib memory_manager
```

Key test coverage:
- Page table operations
- Memory mapping/unmapping
- Protection changes
- Region split/merge
- Error conditions

## Future Enhancements

- [ ] Huge pages (2MB/1GB) support
- [ ] NUMA-aware allocation
- [ ] Memory compression
- [ ] Swap support
- [ ] Memory deduplication
- [ ] Fine-grained locking for scalability

## License

Part of RustOS kernel - same license as parent project.
