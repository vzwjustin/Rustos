# Dynamic Linker Integration Guide

This document explains how to integrate the RustOS dynamic linker with process execution and system initialization.

## Overview

The dynamic linker (`src/process/dynamic_linker.rs`) provides support for loading and executing dynamically-linked ELF binaries. It handles:

- PT_DYNAMIC section parsing
- Shared library dependency resolution
- Symbol table management
- GOT/PLT relocation processing
- Library search path management

## Initialization

The dynamic linker must be initialized during system startup:

```rust
use crate::process::dynamic_linker;

pub fn init_kernel_subsystems() {
    // ... other subsystem initialization ...
    
    // Initialize dynamic linker
    dynamic_linker::init_dynamic_linker();
    
    // ... continue initialization ...
}
```

## Integration with Process Execution

### Method 1: Using the Global Linker

The simplest approach uses the global dynamic linker instance:

```rust
use crate::process::{elf_loader::ElfLoader, dynamic_linker};

pub fn execute_binary(binary_data: &[u8]) -> Result<(), &'static str> {
    // Step 1: Load ELF binary
    let loader = ElfLoader::new(true, true); // ASLR + NX enabled
    let loaded = loader.load_elf_binary(binary_data, current_pid)?;
    
    // Step 2: Check if dynamic linking is needed
    if loaded.is_dynamic {
        // Perform dynamic linking
        let reloc_count = dynamic_linker::link_binary_globally(
            binary_data,
            &loaded.program_headers,
            loaded.base_address
        )?;
        
        println!("Applied {} relocations", reloc_count);
    }
    
    // Step 3: Jump to entry point
    unsafe {
        jump_to_address(loaded.entry_point);
    }
    
    Ok(())
}
```

### Method 2: Using a Custom Linker Instance

For more control, create a dedicated linker instance:

```rust
use crate::process::{elf_loader::ElfLoader, dynamic_linker::DynamicLinker};

pub fn execute_binary_custom(binary_data: &[u8]) -> Result<(), &'static str> {
    let loader = ElfLoader::new(true, true);
    let loaded = loader.load_elf_binary(binary_data, current_pid)?;
    
    if loaded.is_dynamic {
        // Create custom linker with specific search paths
        let mut linker = DynamicLinker::new();
        linker.add_search_path("/custom/lib".to_string());
        
        // Perform linking
        let reloc_count = linker.link_binary(
            binary_data,
            &loaded.program_headers,
            loaded.base_address
        )?;
        
        // Get statistics
        let stats = linker.get_stats();
        println!("Linked {} symbols, {} relocations", 
                 stats.global_symbols, reloc_count);
    }
    
    unsafe {
        jump_to_address(loaded.entry_point);
    }
    
    Ok(())
}
```

## Integration with exec() System Call

Update the exec system call handler to support dynamic linking:

```rust
// In src/process/syscalls.rs or src/process/integration.rs

pub fn sys_execve(path: &str, argv: &[&str], envp: &[&str]) -> Result<i64, &'static str> {
    // Load binary from filesystem
    let binary_data = load_file_from_path(path)?;
    
    // Load ELF binary
    let loader = ElfLoader::new(true, true);
    let loaded = loader.load_elf_binary(&binary_data, current_process_id())?;
    
    // Handle dynamic linking if needed
    if loaded.is_dynamic {
        use crate::process::dynamic_linker;
        
        dynamic_linker::link_binary_globally(
            &binary_data,
            &loaded.program_headers,
            loaded.base_address
        )?;
    }
    
    // Set up process environment
    setup_process_environment(argv, envp)?;
    
    // Jump to entry point (does not return)
    unsafe {
        jump_to_entry_point(loaded.entry_point, loaded.stack_top);
    }
}
```

## Advanced Features

### Custom Library Search Paths

```rust
let mut linker = DynamicLinker::new();

// Add custom paths
linker.add_search_path("/opt/lib".to_string());
linker.add_search_path("/usr/local/custom/lib".to_string());

// Use the linker...
```

### Symbol Resolution

```rust
// Manually resolve a symbol
if let Some(addr) = linker.resolve_symbol("printf") {
    println!("printf is at address: {:?}", addr);
}

// Resolve by index (used internally for relocations)
if let Some(addr) = linker.resolve_symbol_by_index(42) {
    println!("Symbol #42 is at address: {:?}", addr);
}
```

### Pre-loading Symbols

You can pre-populate the symbol table before linking:

```rust
let mut linker = DynamicLinker::new();

// Add kernel-provided symbols
linker.add_symbol("kernel_print".to_string(), VirtAddr::new(kernel_print_addr));
linker.add_symbol("kernel_malloc".to_string(), VirtAddr::new(kernel_malloc_addr));

// Now link the binary - it can reference these symbols
linker.link_binary(binary_data, program_headers, base_address)?;
```

## Workflow Diagram

```
┌─────────────────┐
│ Execute Binary  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Load ELF File  │
└────────┬────────┘
         │
         ▼
    ┌────────────┐
    │ Is Dynamic?├─── No ──► Jump to Entry Point
    └─────┬──────┘
          │ Yes
          ▼
┌──────────────────────┐
│ Initialize Linker    │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Parse PT_DYNAMIC     │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Resolve Library Names│
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Load Dependencies    │  ◄── Recursive for each .so
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Build Symbol Table   │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Parse Relocations    │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Apply Relocations    │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ Jump to Entry Point  │
└──────────────────────┘
```

## Error Handling

The dynamic linker provides detailed error information:

```rust
match linker.link_binary(binary_data, program_headers, base_address) {
    Ok(count) => println!("Successfully applied {} relocations", count),
    Err(e) => match e {
        DynamicLinkerError::LibraryNotFound(lib) => {
            eprintln!("Missing library: {}", lib);
        }
        DynamicLinkerError::SymbolNotFound(sym) => {
            eprintln!("Undefined symbol: {}", sym);
        }
        DynamicLinkerError::UnsupportedRelocation(r_type) => {
            eprintln!("Unsupported relocation type: {}", r_type);
        }
        _ => {
            eprintln!("Dynamic linking failed: {}", e);
        }
    }
}
```

## Current Limitations

1. **Filesystem Integration**: Library loading requires VFS to be mounted
2. **Lazy Binding**: PLT lazy binding not yet implemented
3. **TLS**: Thread-local storage not supported
4. **Symbol Versioning**: Symbol versions not handled
5. **RPATH/RUNPATH**: Custom library paths from ELF not processed

See [LINUX_APP_PROGRESS.md](LINUX_APP_PROGRESS.md) for implementation status.

## Testing

Example test for dynamic linking integration:

```rust
#[test]
fn test_dynamic_binary_execution() {
    // Create a simple dynamically-linked test binary
    let binary_data = create_test_dynamic_binary();
    
    // Load and link
    let loader = ElfLoader::new(true, true);
    let loaded = loader.load_elf_binary(&binary_data, 1).unwrap();
    
    let mut linker = DynamicLinker::new();
    let count = linker.link_binary(
        &binary_data,
        &loaded.program_headers,
        loaded.base_address
    ).unwrap();
    
    assert!(count > 0, "Should have applied relocations");
    
    let stats = linker.get_stats();
    assert!(stats.global_symbols > 0, "Should have loaded symbols");
}
```

## Performance Considerations

- Symbol table lookup is O(log n) using BTreeMap
- Relocation processing is O(n) where n is number of relocations
- Library search is O(p*d) where p is number of paths, d is depth check
- Memory usage: ~100 bytes per symbol + relocation data

## Future Enhancements

1. **Lazy Binding**: Defer symbol resolution until first use
2. **Symbol Caching**: Cache frequently-used symbol lookups
3. **Parallel Loading**: Load multiple libraries concurrently
4. **Prelink Support**: Use prelink information when available
5. **VDSO Support**: Virtual dynamic shared object for syscalls

## References

- [LINUX_APP_SUPPORT.md](LINUX_APP_SUPPORT.md) - Overall Linux app support strategy
- [LINUX_APP_PROGRESS.md](LINUX_APP_PROGRESS.md) - Current implementation status
- [ELF Specification](https://refspecs.linuxfoundation.org/elf/elf.pdf)
- [System V ABI](https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.99.pdf)
