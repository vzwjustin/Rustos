# ELF Binary Loader for RustOS

A complete, production-ready ELF64 binary loader for the RustOS kernel, enabling loading and execution of user-space programs.

## Features

- **ELF64 Format Support**: Full parsing of ELF64 executable format for x86_64 architecture
- **Multiple Executable Types**: Supports both static executables (ET_EXEC) and position-independent executables (ET_DYN/PIE)
- **Segment Loading**: Proper loading of code, data, and BSS segments with correct permissions
- **Memory Mapping**: Integration with kernel page tables for proper virtual memory setup
- **Security**: Enforces W^X (write XOR execute) policy and user/kernel space separation
- **Stack Creation**: Automated stack setup with proper alignment and permissions
- **Validation**: Comprehensive validation of ELF headers and segments

## Architecture

### Module Structure

```
src/elf_loader/
├── mod.rs         # Public API and main entry points
├── types.rs       # ELF64 data structures and constants
├── parser.rs      # ELF header and segment parsing
├── loader.rs      # Memory loading and page table mapping
├── tests.rs       # Comprehensive test suite
└── README.md      # This file
```

### Components

1. **types.rs**: Low-level ELF format definitions
   - ELF64 header structure
   - Program header structure
   - Constants for ELF magic, types, flags
   - Zero-copy parsing from raw bytes

2. **parser.rs**: Validation and parsing logic
   - Header validation (magic, class, endianness, machine)
   - Program header parsing and filtering
   - Segment validation and overlap detection
   - Address range calculation

3. **loader.rs**: Memory mapping implementation
   - Segment loading into virtual memory
   - Page table mapping with proper permissions
   - BSS initialization (zero-filled data)
   - Stack creation and setup

## API Reference

### Main Functions

#### `elf_validate(binary_data: &[u8]) -> Result<()>`

Validates an ELF binary without loading it.

**Parameters:**
- `binary_data`: Raw ELF binary data

**Returns:**
- `Ok(())` if valid
- `Err(ElfError)` describing the validation failure

**Example:**
```rust
let binary = include_bytes!("test_program");
if let Err(e) = elf_validate(binary) {
    println!("Invalid ELF: {:?}", e);
}
```

#### `elf_load(binary_data: &[u8], load_bias: Option<VirtAddr>) -> Result<ElfImage>`

Loads an ELF binary and prepares it for execution.

**Parameters:**
- `binary_data`: Raw ELF binary data
- `load_bias`: Optional base address for PIE executables (uses default if None)

**Returns:**
- `Ok(ElfImage)` containing entry point and segment information
- `Err(ElfError)` if loading fails

**Example:**
```rust
let binary = load_program_from_disk();
match elf_load(&binary, None) {
    Ok(image) => {
        println!("Entry point: {:?}", image.entry_point);
        println!("Segments: {}", image.segments.len());
    }
    Err(e) => println!("Load failed: {:?}", e),
}
```

#### `elf_get_entry_point(image: &ElfImage) -> VirtAddr`

Extracts the entry point address from a loaded image.

**Parameters:**
- `image`: Loaded ELF image

**Returns:**
- Virtual address of program entry point

#### `elf_map_segments<M, A>(image: &ElfImage, binary_data: &[u8], mapper: &mut M, frame_allocator: &mut A) -> Result<()>`

Maps ELF segments into process page table.

**Parameters:**
- `image`: Loaded ELF image
- `binary_data`: Original binary data
- `mapper`: Page table mapper implementing `Mapper<Size4KiB>`
- `frame_allocator`: Physical frame allocator implementing `FrameAllocator<Size4KiB>`

**Returns:**
- `Ok(())` if mapping succeeds
- `Err(ElfError)` if mapping fails

**Example:**
```rust
let image = elf_load(&binary, None)?;
elf_map_segments(&image, &binary, &mut mapper, &mut allocator)?;
```

#### `elf_create_stack<M, A>(mapper: &mut M, frame_allocator: &mut A, stack_bottom: VirtAddr, stack_size: usize) -> Result<VirtAddr>`

Creates and maps a process stack.

**Parameters:**
- `mapper`: Page table mapper
- `frame_allocator`: Physical frame allocator
- `stack_bottom`: Bottom (highest address) of stack region
- `stack_size`: Size of stack in bytes

**Returns:**
- `Ok(VirtAddr)` - Stack pointer (top of stack, aligned)
- `Err(ElfError)` if allocation/mapping fails

## Data Structures

### `ElfImage`

Represents a loaded ELF executable.

```rust
pub struct ElfImage {
    pub entry_point: VirtAddr,        // Program entry point
    pub segments: Vec<LoadedSegment>,  // All loaded segments
    pub is_pie: bool,                  // Position-independent?
    pub base_address: VirtAddr,        // Load base for PIE
    pub program_break: VirtAddr,       // Initial heap boundary
    pub stack_address: VirtAddr,       // Recommended stack location
}
```

### `LoadedSegment`

Represents a loaded memory segment.

```rust
pub struct LoadedSegment {
    pub vaddr: VirtAddr,           // Virtual load address
    pub mem_size: usize,           // Size in memory
    pub file_size: usize,          // Size in file (may be < mem_size)
    pub flags: SegmentFlags,       // Read/Write/Execute permissions
    pub segment_type: SegmentType, // Segment type (Load, Dynamic, etc.)
}
```

### `SegmentFlags`

Memory permissions for a segment.

```rust
pub struct SegmentFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}
```

### `ElfError`

Error types for ELF operations.

```rust
pub enum ElfError {
    InvalidMagic,           // Bad ELF magic number
    InvalidClass,           // Not 64-bit
    InvalidEndianness,      // Not little-endian
    InvalidVersion,         // Unsupported version
    InvalidType,            // Not executable/shared object
    InvalidMachine,         // Not x86_64
    InvalidEntryPoint,      // Invalid entry point
    NoLoadableSegments,     // No PT_LOAD segments
    InvalidAlignment,       // Bad segment alignment
    SizeOverflow,           // Size calculation overflow
    InvalidFlags,           // Invalid segment flags
    AllocationFailed,       // Physical frame allocation failed
    MappingFailed,          // Page mapping failed
    BufferTooSmall,         // Binary data too small
    InvalidProgramHeader,   // Malformed program header
    SegmentOverlap,         // Overlapping segments detected
}
```

## Usage Example

Complete example of loading and executing a user program:

```rust
use rustos::elf_loader::*;
use x86_64::VirtAddr;

// Load program from disk/memory
let binary_data = load_user_program("hello_world");

// Validate before loading
elf_validate(&binary_data)?;

// Load ELF image
let image = elf_load(&binary_data, None)?;

// Create process page table
let mut mapper = create_process_page_table();
let mut frame_allocator = get_frame_allocator();

// Map segments into page table
elf_map_segments(&image, &binary_data, &mut mapper, &mut frame_allocator)?;

// Create process stack (8 MB)
let stack_size = 8 * 1024 * 1024;
let stack_pointer = elf_create_stack(
    &mut mapper,
    &mut frame_allocator,
    image.stack_address,
    stack_size
)?;

// Get entry point
let entry_point = elf_get_entry_point(&image);

// Create process and start execution
let process = Process::new(entry_point, stack_pointer, mapper);
scheduler::spawn(process);
```

## Memory Layout

### Static Executables (ET_EXEC)

```
0x0000000000000000  +------------------------+
                    |      (unmapped)        |
0x0000000000400000  +------------------------+
                    |    .text (code)        |  RX
                    +------------------------+
                    |    .rodata (const)     |  R
                    +------------------------+
                    |    .data (init data)   |  RW
                    +------------------------+
                    |    .bss (zero data)    |  RW
                    +------------------------+
                    |      (unmapped)        |
                    +------------------------+
                    |    Heap (grows up)     |  RW
                    +------------------------+
                    |      (unmapped)        |
0x00007fffffffffff  +------------------------+
                    |   Stack (grows down)   |  RW, NX
0x00007fffffffffff  +------------------------+
```

### PIE Executables (ET_DYN)

Loaded at base address (default: 0x0000_5555_5555_0000), segments are offset from base.

## Security Features

1. **W^X Enforcement**: No pages are both writable and executable
2. **User Space Isolation**: All segments mapped with USER_ACCESSIBLE flag
3. **NO_EXECUTE Stack**: Stack is explicitly marked non-executable
4. **Segment Validation**: Comprehensive validation prevents malformed binaries
5. **Overflow Protection**: All size calculations checked for overflow
6. **Alignment Validation**: Ensures proper memory alignment

## Testing

The module includes comprehensive tests:

```bash
# Run all tests
cargo test --lib elf_loader

# Run specific test
cargo test --lib elf_loader::tests::test_load_minimal_executable
```

### Test Coverage

- Header validation (magic, class, endianness, machine)
- Program header parsing
- Segment overlap detection
- Permission flag conversion
- Address range calculation
- PIE executable handling
- Error condition handling

## Integration with RustOS

The ELF loader integrates with:

1. **Memory Management** (`src/memory.rs`): Uses page allocator and page table APIs
2. **Process Management** (`src/process/`): Provides binaries for process creation
3. **File System** (`src/fs/`): Reads executables from disk
4. **Scheduler** (`src/scheduler/`): Passes loaded programs to scheduler

## Limitations

Current implementation limitations:

1. **Static Linking Only**: No dynamic linker support yet
2. **No Relocations**: Doesn't process relocation entries
3. **No TLS**: Thread-Local Storage not implemented
4. **Basic Validation**: More extensive checks could be added

## Future Enhancements

Planned improvements:

- [ ] Dynamic linking and shared library support
- [ ] ELF relocation processing (GOT/PLT)
- [ ] Thread-Local Storage (TLS) support
- [ ] Support for auxiliary vectors (AT_* entries)
- [ ] Core dump generation for debugging
- [ ] More extensive validation and security checks
- [ ] Large page (2MB/1GB) support for performance

## References

- [ELF-64 Object File Format](https://uclibc.org/docs/elf-64-gen.pdf)
- [System V ABI AMD64 Architecture](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf)
- [Linux Kernel ELF Loader](https://github.com/torvalds/linux/blob/master/fs/binfmt_elf.c)

## License

Part of RustOS project. See main LICENSE file.
