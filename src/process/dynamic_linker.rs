//! Dynamic Linker for RustOS
//!
//! This module implements dynamic linking support, enabling RustOS to load
//! and execute dynamically-linked ELF binaries. This is a critical component
//! for Linux application compatibility as ~95% of Linux binaries use dynamic linking.
//!
//! ## Features
//! - PT_DYNAMIC segment parsing
//! - Shared library (.so) loading
//! - Symbol resolution across loaded libraries
//! - Relocation processing (R_X86_64_* types)
//! - Library search path management
//!
//! ## Architecture
//! The dynamic linker works in phases:
//! 1. Parse PT_DYNAMIC segment from main executable
//! 2. Identify required shared libraries (DT_NEEDED entries)
//! 3. Load each shared library into memory
//! 4. Build global symbol table
//! 5. Process relocations to fix up addresses
//!
//! ## References
//! - ELF Specification: https://refspecs.linuxfoundation.org/elf/elf.pdf
//! - System V ABI: https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.99.pdf
//! - See docs/LINUX_APP_SUPPORT.md for implementation roadmap

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use x86_64::VirtAddr;
use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;

use super::elf_loader::{Elf64Header, Elf64ProgramHeader, elf_constants};
use crate::memory::{MemoryRegionType, MemoryProtection, PAGE_SIZE};
use crate::fs::{OpenFlags, FsResult, FsError};

/// dlopen() flags
pub mod dlopen_flags {
    /// Lazy function call resolution
    pub const RTLD_LAZY: i32 = 0x00001;
    /// Immediate function call resolution
    pub const RTLD_NOW: i32 = 0x00002;
    /// Make symbols globally available
    pub const RTLD_GLOBAL: i32 = 0x00100;
    /// Do not load dependencies
    pub const RTLD_NODELETE: i32 = 0x01000;
    /// Don't unload on dlclose
    pub const RTLD_NOLOAD: i32 = 0x00004;
    /// Use deep binding
    pub const RTLD_DEEPBIND: i32 = 0x00008;
}

/// Handle returned by dlopen
pub type DlHandle = usize;

/// Symbol versioning information
#[derive(Debug, Clone)]
pub struct SymbolVersion {
    /// Version name
    pub name: String,
    /// Version hash
    pub hash: u32,
    /// Is hidden version
    pub hidden: bool,
}

/// Thread Local Storage information
#[derive(Debug, Clone, Copy)]
pub struct TlsInfo {
    /// TLS module ID
    pub module_id: usize,
    /// TLS block offset
    pub offset: usize,
    /// TLS block size
    pub size: usize,
    /// TLS alignment
    pub alignment: usize,
    /// TLS initialization image address
    pub init_image: Option<VirtAddr>,
}

/// Dynamic linker for loading shared libraries and resolving symbols
#[derive(Clone)]
pub struct DynamicLinker {
    /// Library search paths (e.g., /lib, /usr/lib, /lib64)
    search_paths: Vec<String>,

    /// Cache of loaded shared libraries
    loaded_libraries: BTreeMap<String, LoadedLibrary>,

    /// Global symbol table mapping symbol names to addresses
    symbol_table: BTreeMap<String, VirtAddr>,

    /// Symbol table by index for current binary (used during relocation)
    symbol_index_table: Vec<(String, VirtAddr)>,

    /// Base address for library loading (managed with ASLR)
    next_base_address: VirtAddr,
}

/// Information about a loaded shared library
#[derive(Debug, Clone)]
pub struct LoadedLibrary {
    /// Library name (e.g., "libc.so.6")
    pub name: String,

    /// Base address where library is loaded
    pub base_address: VirtAddr,

    /// Size of library in memory
    pub size: usize,

    /// Entry point (if applicable)
    pub entry_point: Option<VirtAddr>,

    /// Dynamic section information
    pub dynamic_info: DynamicInfo,

    /// Reference count for dlopen/dlclose
    pub ref_count: usize,

    /// dlopen flags
    pub flags: i32,

    /// TLS information
    pub tls_info: Option<TlsInfo>,

    /// Symbols exported by this library
    pub symbols: BTreeMap<String, VirtAddr>,

    /// Dependencies (other libraries needed)
    pub dependencies: Vec<String>,

    /// Is this library globally visible?
    pub global: bool,

    /// Can this library be unloaded?
    pub deletable: bool,
}

/// Parsed PT_DYNAMIC section information
#[derive(Debug, Clone, Default)]
pub struct DynamicInfo {
    /// Required shared libraries (DT_NEEDED)
    pub needed: Vec<String>,
    
    /// String table address (DT_STRTAB)
    pub strtab: Option<VirtAddr>,
    
    /// String table size (DT_STRSZ)
    pub strsz: Option<usize>,
    
    /// Symbol table address (DT_SYMTAB)
    pub symtab: Option<VirtAddr>,
    
    /// Symbol table entry size (DT_SYMENT)
    pub syment: Option<usize>,
    
    /// Hash table address (DT_HASH)
    pub hash: Option<VirtAddr>,
    
    /// Relocation table address (DT_RELA)
    pub rela: Option<VirtAddr>,
    
    /// Size of relocation table (DT_RELASZ)
    pub relasz: Option<usize>,
    
    /// Relocation entry size (DT_RELAENT)
    pub relaent: Option<usize>,
    
    /// PLT relocations address (DT_JMPREL)
    pub jmprel: Option<VirtAddr>,
    
    /// Size of PLT relocations (DT_PLTRELSZ)
    pub pltrelsz: Option<usize>,
    
    /// Init function address (DT_INIT)
    pub init: Option<VirtAddr>,

    /// Fini function address (DT_FINI)
    pub fini: Option<VirtAddr>,

    /// GNU hash table address (DT_GNU_HASH)
    pub gnu_hash: Option<VirtAddr>,

    /// Version definitions (DT_VERDEF)
    pub verdef: Option<VirtAddr>,

    /// Number of version definitions (DT_VERDEFNUM)
    pub verdefnum: Option<usize>,

    /// Version needed (DT_VERNEED)
    pub verneed: Option<VirtAddr>,

    /// Number of version needed entries (DT_VERNEEDNUM)
    pub verneednum: Option<usize>,

    /// Version symbol table (DT_VERSYM)
    pub versym: Option<VirtAddr>,

    /// TLS module ID
    pub tls_module_id: Option<usize>,

    /// PT_TLS program header information
    pub tls_image: Option<VirtAddr>,
    pub tls_size: Option<usize>,
    pub tls_align: Option<usize>,
}

/// Relocation entry (RELA format)
#[derive(Debug, Clone, Copy)]
pub struct Relocation {
    /// Offset where to apply the relocation
    pub offset: VirtAddr,
    
    /// Relocation type (R_X86_64_*)
    pub r_type: u32,
    
    /// Symbol index
    pub symbol: u32,
    
    /// Addend value
    pub addend: i64,
}

/// ELF symbol table entry (Elf64_Sym)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Symbol {
    pub st_name: u32,      // Symbol name (string table index)
    pub st_info: u8,       // Symbol type and binding
    pub st_other: u8,      // Symbol visibility
    pub st_shndx: u16,     // Section index
    pub st_value: u64,     // Symbol value
    pub st_size: u64,      // Symbol size
}

impl Elf64Symbol {
    /// Get symbol binding (upper 4 bits of st_info)
    pub fn binding(&self) -> u8 {
        self.st_info >> 4
    }
    
    /// Get symbol type (lower 4 bits of st_info)
    pub fn symbol_type(&self) -> u8 {
        self.st_info & 0xf
    }
    
    /// Check if symbol is defined (not undefined)
    pub fn is_defined(&self) -> bool {
        self.st_shndx != 0  // SHN_UNDEF
    }
}

/// Symbol binding types
pub mod symbol_binding {
    pub const STB_LOCAL: u8 = 0;   // Local symbol
    pub const STB_GLOBAL: u8 = 1;  // Global symbol
    pub const STB_WEAK: u8 = 2;    // Weak symbol
}

/// Symbol types
pub mod symbol_type {
    pub const STT_NOTYPE: u8 = 0;  // No type
    pub const STT_OBJECT: u8 = 1;  // Data object
    pub const STT_FUNC: u8 = 2;    // Code object (function)
    pub const STT_SECTION: u8 = 3; // Section
    pub const STT_FILE: u8 = 4;    // File name
}

/// Dynamic section entry (Elf64_Dyn)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DynamicEntry {
    pub d_tag: i64,
    pub d_val: u64,
}

/// Dynamic section tags (DT_*)
pub mod dynamic_tags {
    pub const DT_NULL: i64 = 0;          // End of dynamic section
    pub const DT_NEEDED: i64 = 1;        // Name of needed library
    pub const DT_PLTRELSZ: i64 = 2;      // Size of PLT relocs
    pub const DT_PLTGOT: i64 = 3;        // PLT/GOT address
    pub const DT_HASH: i64 = 4;          // Symbol hash table address
    pub const DT_STRTAB: i64 = 5;        // String table address
    pub const DT_SYMTAB: i64 = 6;        // Symbol table address
    pub const DT_RELA: i64 = 7;          // Relocation table address
    pub const DT_RELASZ: i64 = 8;        // Size of relocation table
    pub const DT_RELAENT: i64 = 9;       // Size of relocation entry
    pub const DT_STRSZ: i64 = 10;        // Size of string table
    pub const DT_SYMENT: i64 = 11;       // Size of symbol table entry
    pub const DT_INIT: i64 = 12;         // Init function address
    pub const DT_FINI: i64 = 13;         // Fini function address
    pub const DT_SONAME: i64 = 14;       // Name of this shared object
    pub const DT_RPATH: i64 = 15;        // Library search path (deprecated)
    pub const DT_SYMBOLIC: i64 = 16;     // Start symbol search here
    pub const DT_REL: i64 = 17;          // REL format relocations
    pub const DT_RELSZ: i64 = 18;        // Size of REL relocations
    pub const DT_RELENT: i64 = 19;       // Size of REL entry
    pub const DT_PLTREL: i64 = 20;       // Type of PLT reloc (REL or RELA)
    pub const DT_DEBUG: i64 = 21;        // Debug info
    pub const DT_TEXTREL: i64 = 22;      // Reloc might modify text segment
    pub const DT_JMPREL: i64 = 23;       // PLT relocation entries
    pub const DT_BIND_NOW: i64 = 24;     // Process all relocs before executing
    pub const DT_RUNPATH: i64 = 29;      // Library search path
    pub const DT_FLAGS: i64 = 30;        // Flags
    pub const DT_PREINIT_ARRAY: i64 = 32;  // Array of pre-init functions
    pub const DT_PREINIT_ARRAYSZ: i64 = 33; // Size of pre-init array
    pub const DT_GNU_HASH: i64 = 0x6ffffef5; // GNU-style hash table
    pub const DT_VERSYM: i64 = 0x6ffffff0;   // Version symbol table
    pub const DT_VERDEF: i64 = 0x6ffffffc;   // Version definitions
    pub const DT_VERDEFNUM: i64 = 0x6ffffffd; // Number of version definitions
    pub const DT_VERNEED: i64 = 0x6ffffffe;  // Version dependencies
    pub const DT_VERNEEDNUM: i64 = 0x6fffffff; // Number of version dependencies
}

/// Relocation types for x86_64
pub mod relocation_types {
    pub const R_X86_64_NONE: u32 = 0;           // No relocation
    pub const R_X86_64_64: u32 = 1;             // Direct 64 bit
    pub const R_X86_64_PC32: u32 = 2;           // PC relative 32 bit signed
    pub const R_X86_64_GOT32: u32 = 3;          // 32 bit GOT entry
    pub const R_X86_64_PLT32: u32 = 4;          // 32 bit PLT address
    pub const R_X86_64_COPY: u32 = 5;           // Copy symbol at runtime
    pub const R_X86_64_GLOB_DAT: u32 = 6;       // Create GOT entry
    pub const R_X86_64_JUMP_SLOT: u32 = 7;      // Create PLT entry
    pub const R_X86_64_RELATIVE: u32 = 8;       // Adjust by program base
    pub const R_X86_64_GOTPCREL: u32 = 9;       // 32 bit signed PC relative offset to GOT
    pub const R_X86_64_32: u32 = 10;            // Direct 32 bit zero extended
    pub const R_X86_64_32S: u32 = 11;           // Direct 32 bit sign extended
    pub const R_X86_64_16: u32 = 12;            // Direct 16 bit zero extended
    pub const R_X86_64_PC16: u32 = 13;          // 16 bit sign extended PC relative
    pub const R_X86_64_8: u32 = 14;             // Direct 8 bit sign extended
    pub const R_X86_64_PC8: u32 = 15;           // 8 bit sign extended PC relative
    pub const R_X86_64_DTPMOD64: u32 = 16;      // ID of module containing symbol
    pub const R_X86_64_DTPOFF64: u32 = 17;      // Offset in TLS block
    pub const R_X86_64_TPOFF64: u32 = 18;       // Offset in initial TLS block
    pub const R_X86_64_TLSGD: u32 = 19;         // PC relative offset to GD GOT entry
    pub const R_X86_64_TLSLD: u32 = 20;         // PC relative offset to LD GOT entry
    pub const R_X86_64_DTPOFF32: u32 = 21;      // Offset in TLS block (32-bit)
    pub const R_X86_64_GOTTPOFF: u32 = 22;      // PC relative offset to IE GOT entry
    pub const R_X86_64_TPOFF32: u32 = 23;       // Offset in initial TLS block (32-bit)
    pub const R_X86_64_PC64: u32 = 24;          // PC relative 64 bit
    pub const R_X86_64_GOTOFF64: u32 = 25;      // 64 bit offset to GOT
    pub const R_X86_64_GOTPC32: u32 = 26;       // 32 bit signed PC relative offset to GOT
    pub const R_X86_64_SIZE32: u32 = 32;        // Size of symbol plus 32-bit addend
    pub const R_X86_64_SIZE64: u32 = 33;        // Size of symbol plus 64-bit addend
    pub const R_X86_64_IRELATIVE: u32 = 37;     // Adjust indirectly by program base
}

/// Errors that can occur during dynamic linking
#[derive(Debug, Clone)]
pub enum DynamicLinkerError {
    /// PT_DYNAMIC segment not found
    NoDynamicSegment,
    
    /// Invalid dynamic section entry
    InvalidDynamicEntry,
    
    /// Required library not found
    LibraryNotFound(String),
    
    /// Symbol not found
    SymbolNotFound(String),
    
    /// Unsupported relocation type
    UnsupportedRelocation(u32),
    
    /// Invalid memory address
    InvalidAddress,
    
    /// Memory allocation failed
    AllocationFailed,
    
    /// Invalid ELF file
    InvalidElf(String),
}

impl fmt::Display for DynamicLinkerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DynamicLinkerError::NoDynamicSegment => 
                write!(f, "PT_DYNAMIC segment not found in ELF binary"),
            DynamicLinkerError::InvalidDynamicEntry => 
                write!(f, "Invalid dynamic section entry"),
            DynamicLinkerError::LibraryNotFound(lib) => 
                write!(f, "Required library not found: {}", lib),
            DynamicLinkerError::SymbolNotFound(sym) => 
                write!(f, "Symbol not found: {}", sym),
            DynamicLinkerError::UnsupportedRelocation(r_type) => 
                write!(f, "Unsupported relocation type: {}", r_type),
            DynamicLinkerError::InvalidAddress => 
                write!(f, "Invalid memory address"),
            DynamicLinkerError::AllocationFailed => 
                write!(f, "Memory allocation failed"),
            DynamicLinkerError::InvalidElf(msg) => 
                write!(f, "Invalid ELF: {}", msg),
        }
    }
}

pub type DynamicLinkerResult<T> = Result<T, DynamicLinkerError>;

impl DynamicLinker {
    /// Create a new dynamic linker instance
    pub fn new() -> Self {
        let mut search_paths = Vec::new();
        // Standard Linux library search paths
        search_paths.push(String::from("/lib"));
        search_paths.push(String::from("/lib64"));
        search_paths.push(String::from("/usr/lib"));
        search_paths.push(String::from("/usr/lib64"));
        search_paths.push(String::from("/usr/local/lib"));
        
        Self {
            search_paths,
            loaded_libraries: BTreeMap::new(),
            symbol_table: BTreeMap::new(),
            symbol_index_table: Vec::new(),
            // Start library loading at a safe address (above user space)
            next_base_address: VirtAddr::new(0x400000_0000),
        }
    }
    
    /// Add a library search path
    pub fn add_search_path(&mut self, path: String) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }
    
    /// Parse PT_DYNAMIC segment from ELF binary
    pub fn parse_dynamic_section(
        &self,
        binary_data: &[u8],
        program_headers: &[Elf64ProgramHeader],
        base_address: VirtAddr,
    ) -> DynamicLinkerResult<DynamicInfo> {
        // Find PT_DYNAMIC segment
        let dynamic_phdr = program_headers.iter()
            .find(|phdr| phdr.p_type == elf_constants::PT_DYNAMIC)
            .ok_or(DynamicLinkerError::NoDynamicSegment)?;
        
        let mut dynamic_info = DynamicInfo::default();
        
        // Parse dynamic entries
        let dyn_offset = dynamic_phdr.p_offset as usize;
        let dyn_size = dynamic_phdr.p_filesz as usize;
        
        if dyn_offset + dyn_size > binary_data.len() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Dynamic section out of bounds")
            ));
        }
        
        let dyn_data = &binary_data[dyn_offset..dyn_offset + dyn_size];
        let entry_count = dyn_size / core::mem::size_of::<DynamicEntry>();
        
        for i in 0..entry_count {
            let entry = self.parse_dynamic_entry(dyn_data, i)?;
            
            if entry.d_tag == dynamic_tags::DT_NULL {
                break; // End of dynamic section
            }
            
            self.process_dynamic_entry(&mut dynamic_info, &entry, base_address);
        }
        
        Ok(dynamic_info)
    }
    
    /// Parse a single dynamic entry
    fn parse_dynamic_entry(&self, data: &[u8], index: usize) -> DynamicLinkerResult<DynamicEntry> {
        let offset = index * core::mem::size_of::<DynamicEntry>();
        
        if offset + core::mem::size_of::<DynamicEntry>() > data.len() {
            return Err(DynamicLinkerError::InvalidDynamicEntry);
        }
        
        // Read d_tag (8 bytes, little-endian)
        let d_tag = i64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
        ]);
        
        // Read d_val (8 bytes, little-endian)
        let d_val = u64::from_le_bytes([
            data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11],
            data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15],
        ]);
        
        Ok(DynamicEntry { d_tag, d_val })
    }
    
    /// Process a dynamic entry and update DynamicInfo
    fn process_dynamic_entry(&self, info: &mut DynamicInfo, entry: &DynamicEntry, base: VirtAddr) {
        match entry.d_tag {
            dynamic_tags::DT_NEEDED => {
                info.needed.push(format!("offset:{}", entry.d_val));
            }
            dynamic_tags::DT_STRTAB => {
                info.strtab = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_STRSZ => {
                info.strsz = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_SYMTAB => {
                info.symtab = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_SYMENT => {
                info.syment = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_HASH => {
                info.hash = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_GNU_HASH => {
                info.gnu_hash = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_RELA => {
                info.rela = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_RELASZ => {
                info.relasz = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_RELAENT => {
                info.relaent = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_JMPREL => {
                info.jmprel = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_PLTRELSZ => {
                info.pltrelsz = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_INIT => {
                info.init = Some(VirtAddr::new(base.as_u64() + entry.d_val));
            }
            dynamic_tags::DT_FINI => {
                info.fini = Some(VirtAddr::new(base.as_u64() + entry.d_val));
            }
            dynamic_tags::DT_VERSYM => {
                info.versym = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_VERDEF => {
                info.verdef = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_VERDEFNUM => {
                info.verdefnum = Some(entry.d_val as usize);
            }
            dynamic_tags::DT_VERNEED => {
                info.verneed = Some(VirtAddr::new(entry.d_val));
            }
            dynamic_tags::DT_VERNEEDNUM => {
                info.verneednum = Some(entry.d_val as usize);
            }
            _ => {
                // Ignore other tags
            }
        }
    }
    
    /// Load required dependencies for a binary
    pub fn load_dependencies(&mut self, needed: &[String], flags: i32) -> DynamicLinkerResult<Vec<String>> {
        let mut loaded = Vec::new();

        for lib_name in needed {
            // Skip if already loaded
            if self.loaded_libraries.contains_key(lib_name) {
                // Increment reference count
                if let Some(lib) = self.loaded_libraries.get_mut(lib_name) {
                    lib.ref_count += 1;
                }
                loaded.push(lib_name.clone());
                continue;
            }

            // Try to find and load the library
            match self.find_library(lib_name) {
                Some(path) => {
                    match self.load_shared_library(&path, flags) {
                        Ok(lib) => {
                            loaded.push(lib_name.clone());
                            // Recursively load dependencies
                            let deps = lib.dependencies.clone();
                            self.loaded_libraries.insert(lib_name.clone(), lib);
                            self.load_dependencies(&deps, flags)?;
                        }
                        Err(e) => {
                            // If library load fails, it might not exist in VFS yet
                            // This is acceptable during early boot
                            if matches!(e, DynamicLinkerError::LibraryNotFound(_)) {
                                loaded.push(lib_name.clone());
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
                None => {
                    // Library not found in search paths
                    // This might be OK if filesystem isn't fully mounted yet
                    loaded.push(lib_name.clone());
                }
            }
        }

        Ok(loaded)
    }
    
    /// Search for a library in search paths
    fn find_library(&self, name: &str) -> Option<String> {
        for path in &self.search_paths {
            let full_path = format!("{}/{}", path, name);
            // Check if file exists via VFS
            if self.check_file_exists(&full_path) {
                return Some(full_path);
            }
        }
        None
    }
    
    /// Check if a file exists in the filesystem
    fn check_file_exists(&self, path: &str) -> bool {
        use crate::fs::SyscallOpenFlags;

        if let Some(vfs) = get_vfs_manager() {
            vfs.open(path, SyscallOpenFlags::READ, 0).is_ok()
        } else {
            // VFS not initialized yet, assume file might exist
            true
        }
    }

    /// Load a shared library file from filesystem
    ///
    /// Returns the library data if successfully loaded
    pub fn load_library_file(&self, path: &str) -> DynamicLinkerResult<Vec<u8>> {
        use crate::fs::SyscallOpenFlags;

        let vfs = get_vfs_manager()
            .ok_or_else(|| DynamicLinkerError::LibraryNotFound(
                format!("{} (VFS not initialized)", path)
            ))?;

        // Open the file
        let inode = vfs.open(path, SyscallOpenFlags::READ, 0)
            .map_err(|_| DynamicLinkerError::LibraryNotFound(path.to_string()))?;

        // Get file size
        let size = inode.size() as usize;
        if size == 0 {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Library file is empty")
            ));
        }

        // Read file data
        let mut buffer = vec![0u8; size];
        let bytes_read = inode.read(0, &mut buffer)
            .map_err(|_| DynamicLinkerError::InvalidElf(
                String::from("Failed to read library")
            ))?;

        if bytes_read != size {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Incomplete library read")
            ));
        }

        Ok(buffer)
    }
    
    /// Resolve a symbol by name across all loaded libraries
    pub fn resolve_symbol(&self, name: &str) -> Option<VirtAddr> {
        self.symbol_table.get(name).copied()
    }
    
    /// Add a symbol to the global symbol table
    pub fn add_symbol(&mut self, name: String, address: VirtAddr) {
        self.symbol_table.insert(name, address);
    }
    
    /// Apply relocations to a loaded binary
    pub fn apply_relocations(
        &self,
        relocations: &[Relocation],
        base_address: VirtAddr,
    ) -> DynamicLinkerResult<()> {
        for reloc in relocations {
            let target = VirtAddr::new(base_address.as_u64() + reloc.offset.as_u64());

            match reloc.r_type {
                relocation_types::R_X86_64_NONE => {
                    // No relocation needed
                }
                relocation_types::R_X86_64_RELATIVE => {
                    // Adjust by program base address: B + A
                    let value = base_address.as_u64() + reloc.addend as u64;
                    unsafe {
                        self.write_relocation_value(target, value)?;
                    }
                }
                relocation_types::R_X86_64_IRELATIVE => {
                    // Indirect relative: B + A, then call the result as a function
                    let resolver_addr = base_address.as_u64() + reloc.addend as u64;
                    // Call resolver function to get actual address
                    // For now, just use the resolver address directly
                    unsafe {
                        self.write_relocation_value(target, resolver_addr)?;
                    }
                }
                relocation_types::R_X86_64_GLOB_DAT => {
                    // Symbol value: S
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        unsafe {
                            self.write_relocation_value(target, symbol_addr.as_u64())?;
                        }
                    } else {
                        return Err(DynamicLinkerError::SymbolNotFound(
                            format!("symbol index {}", reloc.symbol)
                        ));
                    }
                }
                relocation_types::R_X86_64_JUMP_SLOT => {
                    // PLT entry: S
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        unsafe {
                            self.write_relocation_value(target, symbol_addr.as_u64())?;
                        }
                    }
                    // For lazy binding, leave unresolved if symbol not found
                }
                relocation_types::R_X86_64_64 => {
                    // Direct 64-bit: S + A
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = symbol_addr.as_u64() + reloc.addend as u64;
                        unsafe {
                            self.write_relocation_value(target, value)?;
                        }
                    } else {
                        return Err(DynamicLinkerError::SymbolNotFound(
                            format!("symbol index {}", reloc.symbol)
                        ));
                    }
                }
                relocation_types::R_X86_64_PC32 => {
                    // PC-relative 32-bit: S + A - P
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = (symbol_addr.as_u64() as i64 + reloc.addend - target.as_u64() as i64) as u32;
                        unsafe {
                            self.write_relocation_value_32(target, value)?;
                        }
                    } else {
                        return Err(DynamicLinkerError::SymbolNotFound(
                            format!("symbol index {}", reloc.symbol)
                        ));
                    }
                }
                relocation_types::R_X86_64_32 => {
                    // Direct 32-bit zero extended: S + A
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = (symbol_addr.as_u64() + reloc.addend as u64) as u32;
                        unsafe {
                            self.write_relocation_value_32(target, value)?;
                        }
                    } else {
                        return Err(DynamicLinkerError::SymbolNotFound(
                            format!("symbol index {}", reloc.symbol)
                        ));
                    }
                }
                relocation_types::R_X86_64_32S => {
                    // Direct 32-bit sign extended: S + A
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = (symbol_addr.as_u64() as i64 + reloc.addend) as i32 as u32;
                        unsafe {
                            self.write_relocation_value_32(target, value)?;
                        }
                    } else {
                        return Err(DynamicLinkerError::SymbolNotFound(
                            format!("symbol index {}", reloc.symbol)
                        ));
                    }
                }
                relocation_types::R_X86_64_COPY => {
                    // Copy relocation: copy from shared object
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        // Get symbol size from symbol table
                        // Copy data from symbol_addr to target
                        // This is complex, skip for now
                    }
                }
                relocation_types::R_X86_64_DTPMOD64 => {
                    // TLS module ID
                    // For now, write 0 (single module)
                    unsafe {
                        self.write_relocation_value(target, 0)?;
                    }
                }
                relocation_types::R_X86_64_DTPOFF64 => {
                    // TLS offset
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = symbol_addr.as_u64() + reloc.addend as u64;
                        unsafe {
                            self.write_relocation_value(target, value)?;
                        }
                    }
                }
                relocation_types::R_X86_64_TPOFF64 => {
                    // TLS initial offset
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = symbol_addr.as_u64() + reloc.addend as u64;
                        unsafe {
                            self.write_relocation_value(target, value)?;
                        }
                    }
                }
                relocation_types::R_X86_64_GOTPCREL => {
                    // GOT-relative PC-relative: G + GOT + A - P
                    // Simplified: treat as PC-relative
                    if let Some(symbol_addr) = self.resolve_symbol_by_index(reloc.symbol) {
                        let value = (symbol_addr.as_u64() as i64 + reloc.addend - target.as_u64() as i64) as u32;
                        unsafe {
                            self.write_relocation_value_32(target, value)?;
                        }
                    }
                }
                _ => {
                    // Unsupported relocation type - log but don't fail
                    // This allows partial linking to continue
                }
            }
        }

        Ok(())
    }
    
    /// Get list of loaded libraries
    pub fn loaded_libraries(&self) -> Vec<&LoadedLibrary> {
        self.loaded_libraries.values().collect()
    }
    
    /// Check if a library is loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded_libraries.contains_key(name)
    }
    
    /// Complete dynamic linking workflow for a binary
    ///
    /// This is the main entry point that orchestrates:
    /// 1. Parsing PT_DYNAMIC section
    /// 2. Resolving library names from string table
    /// 3. Loading dependencies
    /// 4. Building symbol table
    /// 5. Parsing and applying relocations
    ///
    /// # Arguments
    /// * `binary_data` - The ELF binary data
    /// * `program_headers` - Program headers from the ELF
    /// * `base_address` - Base address where binary is loaded
    ///
    /// # Returns
    /// Number of relocations applied
    pub fn link_binary(
        &mut self,
        binary_data: &[u8],
        program_headers: &[super::elf_loader::Elf64ProgramHeader],
        base_address: VirtAddr,
    ) -> DynamicLinkerResult<usize> {
        // Step 1: Parse dynamic section
        let mut dynamic_info = self.parse_dynamic_section(
            binary_data,
            program_headers,
            base_address
        )?;

        // Step 2: Resolve library names from string table
        self.resolve_library_names(binary_data, &mut dynamic_info)?;

        // Step 3: Load required dependencies (use RTLD_NOW for eager binding)
        let _loaded_libs = self.load_dependencies(&dynamic_info.needed, dlopen_flags::RTLD_NOW)?;

        // Step 4: Load symbols from this binary into global symbol table
        let _symbol_count = self.load_symbols_from_binary(
            binary_data,
            &dynamic_info,
            base_address
        )?;

        // Step 5: Parse relocations
        let relocations = self.parse_relocations(binary_data, &dynamic_info)?;
        let reloc_count = relocations.len();

        // Step 6: Apply relocations
        self.apply_relocations(&relocations, base_address)?;

        Ok(reloc_count)
    }

    /// dlopen() - Open a shared library
    ///
    /// # Arguments
    /// * `filename` - Path to the shared library, or None for main program
    /// * `flags` - RTLD_* flags controlling binding behavior
    ///
    /// # Returns
    /// Handle to the loaded library
    pub fn dlopen(&mut self, filename: Option<&str>, flags: i32) -> DynamicLinkerResult<DlHandle> {
        let lib_name = match filename {
            Some(name) => {
                // Check if already loaded
                if let Some(lib) = self.loaded_libraries.get_mut(name) {
                    lib.ref_count += 1;
                    return Ok(lib.base_address.as_u64() as DlHandle);
                }

                // Try to find library in search paths
                let path = self.find_library(name)
                    .ok_or_else(|| DynamicLinkerError::LibraryNotFound(name.to_string()))?;

                // Load the library
                let lib = self.load_shared_library(&path, flags)?;
                let handle = lib.base_address.as_u64() as DlHandle;

                // Load dependencies recursively
                let deps = lib.dependencies.clone();
                self.loaded_libraries.insert(name.to_string(), lib);
                self.load_dependencies(&deps, flags)?;

                handle
            }
            None => {
                // Return handle to main program
                0 as DlHandle
            }
        };

        Ok(lib_name)
    }

    /// dlsym() - Look up a symbol in a loaded library
    ///
    /// # Arguments
    /// * `handle` - Handle from dlopen, or special values RTLD_DEFAULT/RTLD_NEXT
    /// * `symbol` - Symbol name to look up
    ///
    /// # Returns
    /// Address of the symbol
    pub fn dlsym(&self, handle: DlHandle, symbol: &str) -> DynamicLinkerResult<VirtAddr> {
        if handle == 0 {
            // Search all loaded libraries
            self.resolve_symbol(symbol)
                .ok_or_else(|| DynamicLinkerError::SymbolNotFound(symbol.to_string()))
        } else {
            // Search specific library
            let base_addr = VirtAddr::new(handle as u64);
            for lib in self.loaded_libraries.values() {
                if lib.base_address == base_addr {
                    return lib.symbols.get(symbol)
                        .copied()
                        .ok_or_else(|| DynamicLinkerError::SymbolNotFound(symbol.to_string()));
                }
            }
            Err(DynamicLinkerError::InvalidElf(String::from("Invalid handle")))
        }
    }

    /// dlclose() - Close a shared library
    ///
    /// # Arguments
    /// * `handle` - Handle from dlopen
    ///
    /// # Returns
    /// Ok if successful
    pub fn dlclose(&mut self, handle: DlHandle) -> DynamicLinkerResult<()> {
        if handle == 0 {
            // Can't close main program
            return Ok(());
        }

        let base_addr = VirtAddr::new(handle as u64);

        // Find library by handle
        let lib_name = {
            let mut found_name = None;
            for (name, lib) in &mut self.loaded_libraries {
                if lib.base_address == base_addr {
                    lib.ref_count -= 1;
                    if lib.ref_count == 0 && lib.deletable {
                        found_name = Some(name.clone());
                    }
                    break;
                }
            }
            found_name
        };

        // Remove library if reference count reached zero
        if let Some(name) = lib_name {
            self.loaded_libraries.remove(&name);
        }

        Ok(())
    }

    /// dlerror() - Get last error message
    ///
    /// Returns the last error that occurred in dlopen/dlsym/dlclose
    pub fn dlerror(&self) -> Option<String> {
        // In a full implementation, this would track the last error
        // For now, return None
        None
    }
    
    /// Get linking statistics
    pub fn get_stats(&self) -> DynamicLinkerStats {
        DynamicLinkerStats {
            loaded_libraries: self.loaded_libraries.len(),
            global_symbols: self.symbol_table.len(),
            search_paths: self.search_paths.len(),
        }
    }
    
    /// Parse string table and resolve library names
    pub fn resolve_library_names(
        &self,
        binary_data: &[u8],
        dynamic_info: &mut DynamicInfo,
    ) -> DynamicLinkerResult<()> {
        // Check if we have string table information
        let strtab_addr = dynamic_info.strtab
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table")))?;
        let strtab_size = dynamic_info.strsz
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table size")))?;
        
        // In a real implementation, strtab_addr would be a virtual address
        // For now, we'll treat it as an offset into the binary
        let strtab_offset = strtab_addr.as_u64() as usize;
        
        if strtab_offset + strtab_size > binary_data.len() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("String table out of bounds")
            ));
        }
        
        let strtab = &binary_data[strtab_offset..strtab_offset + strtab_size];
        
        // Resolve library names from offsets
        let mut resolved_names = Vec::new();
        for name_ref in &dynamic_info.needed {
            if name_ref.starts_with("offset:") {
                let offset_str = &name_ref[7..];
                if let Ok(offset) = offset_str.parse::<usize>() {
                    if let Some(name) = self.read_string_from_table(strtab, offset) {
                        resolved_names.push(name);
                    }
                }
            } else {
                // Already resolved
                resolved_names.push(name_ref.clone());
            }
        }
        
        dynamic_info.needed = resolved_names;
        Ok(())
    }
    
    /// Read a null-terminated string from the string table
    fn read_string_from_table(&self, strtab: &[u8], offset: usize) -> Option<String> {
        if offset >= strtab.len() {
            return None;
        }
        
        let mut end = offset;
        while end < strtab.len() && strtab[end] != 0 {
            end += 1;
        }
        
        if end > offset {
            String::from_utf8(strtab[offset..end].to_vec()).ok()
        } else {
            None
        }
    }
    
    /// Parse symbol table from ELF binary
    pub fn parse_symbol_table(
        &self,
        binary_data: &[u8],
        dynamic_info: &DynamicInfo,
    ) -> DynamicLinkerResult<Vec<(String, VirtAddr, Elf64Symbol)>> {
        let symtab_addr = dynamic_info.symtab
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No symbol table")))?;
        let strtab_addr = dynamic_info.strtab
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table")))?;
        let strtab_size = dynamic_info.strsz
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table size")))?;
        
        // Calculate symbol table bounds
        // We'll use hash table to determine the number of symbols if available
        let symtab_offset = symtab_addr.as_u64() as usize;
        let strtab_offset = strtab_addr.as_u64() as usize;
        
        if strtab_offset + strtab_size > binary_data.len() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("String table out of bounds")
            ));
        }
        
        let strtab = &binary_data[strtab_offset..strtab_offset + strtab_size];
        
        // Calculate number of symbols
        // Symbol table ends where string table begins (common layout)
        let sym_count = if strtab_offset > symtab_offset {
            (strtab_offset - symtab_offset) / core::mem::size_of::<Elf64Symbol>()
        } else {
            // Fallback: parse until we run out of data or hit invalid entries
            100 // Conservative estimate
        };
        
        let mut symbols = Vec::new();
        
        for i in 0..sym_count {
            let sym_offset = symtab_offset + i * core::mem::size_of::<Elf64Symbol>();
            
            if sym_offset + core::mem::size_of::<Elf64Symbol>() > binary_data.len() {
                break;
            }
            
            // Parse symbol entry
            let symbol = unsafe {
                core::ptr::read(binary_data[sym_offset..].as_ptr() as *const Elf64Symbol)
            };
            
            // Skip undefined symbols
            if !symbol.is_defined() {
                continue;
            }
            
            // Read symbol name from string table
            if let Some(name) = self.read_string_from_table(strtab, symbol.st_name as usize) {
                if !name.is_empty() {
                    symbols.push((name, VirtAddr::new(symbol.st_value), symbol));
                }
            }
        }
        
        Ok(symbols)
    }
    
    /// Load symbols into global symbol table and index table
    pub fn load_symbols_from_binary(
        &mut self,
        binary_data: &[u8],
        dynamic_info: &DynamicInfo,
        base_address: VirtAddr,
    ) -> DynamicLinkerResult<usize> {
        // First, build the complete symbol table with indices
        self.build_symbol_index_table(binary_data, dynamic_info, base_address)?;

        // Then add defined symbols to global symbol table
        let count = self.symbol_index_table.len();

        // Collect symbols to add to avoid borrowing issues
        let symbols_to_add: Vec<_> = self.symbol_index_table.iter()
            .filter(|(name, _)| !name.is_empty())
            .map(|(name, addr)| (name.clone(), *addr))
            .collect();

        for (name, addr) in symbols_to_add {
            self.add_symbol(name, addr);
        }

        Ok(count)
    }
    
    /// Build symbol index table from binary
    /// 
    /// This creates a complete mapping of symbol indices to (name, address) pairs,
    /// including undefined symbols (which will have address 0).
    fn build_symbol_index_table(
        &mut self,
        binary_data: &[u8],
        dynamic_info: &DynamicInfo,
        base_address: VirtAddr,
    ) -> DynamicLinkerResult<()> {
        let symtab_addr = dynamic_info.symtab
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No symbol table")))?;
        let strtab_addr = dynamic_info.strtab
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table")))?;
        let strtab_size = dynamic_info.strsz
            .ok_or(DynamicLinkerError::InvalidElf(String::from("No string table size")))?;
        
        let symtab_offset = symtab_addr.as_u64() as usize;
        let strtab_offset = strtab_addr.as_u64() as usize;
        
        if strtab_offset + strtab_size > binary_data.len() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("String table out of bounds")
            ));
        }
        
        let strtab = &binary_data[strtab_offset..strtab_offset + strtab_size];
        
        // Calculate number of symbols
        let sym_count = if strtab_offset > symtab_offset {
            (strtab_offset - symtab_offset) / core::mem::size_of::<Elf64Symbol>()
        } else {
            100 // Conservative estimate
        };
        
        // Clear and rebuild index table
        self.symbol_index_table.clear();
        
        for i in 0..sym_count {
            let sym_offset = symtab_offset + i * core::mem::size_of::<Elf64Symbol>();
            
            if sym_offset + core::mem::size_of::<Elf64Symbol>() > binary_data.len() {
                break;
            }
            
            // Parse symbol entry
            let symbol = unsafe {
                core::ptr::read(binary_data[sym_offset..].as_ptr() as *const Elf64Symbol)
            };
            
            // Get symbol name
            let name = self.read_string_from_table(strtab, symbol.st_name as usize)
                .unwrap_or_else(|| String::new());
            
            // Calculate address (0 for undefined symbols)
            let addr = if symbol.is_defined() {
                if symbol.symbol_type() == symbol_type::STT_FUNC ||
                   symbol.symbol_type() == symbol_type::STT_OBJECT {
                    VirtAddr::new(base_address.as_u64() + symbol.st_value)
                } else {
                    VirtAddr::new(symbol.st_value)
                }
            } else {
                VirtAddr::new(0) // Undefined - will need to be resolved from other libraries
            };
            
            self.symbol_index_table.push((name, addr));
        }
        
        Ok(())
    }
    
    /// Resolve symbol by index (used during relocation)
    pub fn resolve_symbol_by_index(&self, index: u32) -> Option<VirtAddr> {
        let idx = index as usize;
        if idx < self.symbol_index_table.len() {
            let (_name, addr) = &self.symbol_index_table[idx];
            if addr.as_u64() != 0 {
                Some(*addr)
            } else {
                // Symbol is undefined in current binary, try global symbol table
                let (name, _) = &self.symbol_index_table[idx];
                self.resolve_symbol(name)
            }
        } else {
            None
        }
    }
    
    /// Parse relocations from RELA section
    pub fn parse_relocations(
        &self,
        binary_data: &[u8],
        dynamic_info: &DynamicInfo,
    ) -> DynamicLinkerResult<Vec<Relocation>> {
        let mut relocations = Vec::new();
        
        // Parse regular relocations (DT_RELA)
        if let (Some(rela_addr), Some(rela_size)) = (dynamic_info.rela, dynamic_info.relasz) {
            let rela_offset = rela_addr.as_u64() as usize;
            let reloc_entry_size = dynamic_info.relaent.unwrap_or(24); // Standard RELA entry size
            let reloc_count = rela_size / reloc_entry_size;
            
            for i in 0..reloc_count {
                let offset = rela_offset + i * reloc_entry_size;
                if let Some(reloc) = self.parse_single_relocation(binary_data, offset)? {
                    relocations.push(reloc);
                }
            }
        }
        
        // Parse PLT relocations (DT_JMPREL)
        if let (Some(jmprel_addr), Some(jmprel_size)) = (dynamic_info.jmprel, dynamic_info.pltrelsz) {
            let jmprel_offset = jmprel_addr.as_u64() as usize;
            let reloc_entry_size = 24; // RELA entry size
            let reloc_count = jmprel_size / reloc_entry_size;
            
            for i in 0..reloc_count {
                let offset = jmprel_offset + i * reloc_entry_size;
                if let Some(reloc) = self.parse_single_relocation(binary_data, offset)? {
                    relocations.push(reloc);
                }
            }
        }
        
        Ok(relocations)
    }
    
    /// Parse a single relocation entry
    fn parse_single_relocation(
        &self,
        binary_data: &[u8],
        offset: usize,
    ) -> DynamicLinkerResult<Option<Relocation>> {
        const RELA_ENTRY_SIZE: usize = 24; // r_offset (8) + r_info (8) + r_addend (8)
        
        if offset + RELA_ENTRY_SIZE > binary_data.len() {
            return Ok(None);
        }
        
        let data = &binary_data[offset..offset + RELA_ENTRY_SIZE];
        
        // Parse r_offset
        let r_offset = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        
        // Parse r_info
        let r_info = u64::from_le_bytes([
            data[8], data[9], data[10], data[11],
            data[12], data[13], data[14], data[15],
        ]);
        
        // Parse r_addend
        let r_addend = i64::from_le_bytes([
            data[16], data[17], data[18], data[19],
            data[20], data[21], data[22], data[23],
        ]);
        
        // Extract symbol and type from r_info
        let r_type = (r_info & 0xffffffff) as u32;
        let r_sym = (r_info >> 32) as u32;
        
        Ok(Some(Relocation {
            offset: VirtAddr::new(r_offset),
            r_type,
            symbol: r_sym,
            addend: r_addend,
        }))
    }
    
    /// Write 64-bit value to memory (helper for relocations)
    ///
    /// # Safety
    /// This function writes to arbitrary memory addresses.
    /// Caller must ensure the address is valid and writable.
    unsafe fn write_relocation_value(&self, addr: VirtAddr, value: u64) -> DynamicLinkerResult<()> {
        let ptr = addr.as_u64() as *mut u64;
        core::ptr::write_volatile(ptr, value);
        Ok(())
    }

    /// Write 32-bit value to memory (helper for relocations)
    ///
    /// # Safety
    /// This function writes to arbitrary memory addresses.
    /// Caller must ensure the address is valid and writable.
    unsafe fn write_relocation_value_32(&self, addr: VirtAddr, value: u32) -> DynamicLinkerResult<()> {
        let ptr = addr.as_u64() as *mut u32;
        core::ptr::write_volatile(ptr, value);
        Ok(())
    }

    /// Load a shared library from disk and parse it
    ///
    /// This function:
    /// 1. Loads the library file from disk
    /// 2. Parses the ELF headers
    /// 3. Loads segments into memory
    /// 4. Parses dynamic section
    /// 5. Extracts symbols
    /// 6. Returns a LoadedLibrary structure
    fn load_shared_library(&mut self, path: &str, flags: i32) -> DynamicLinkerResult<LoadedLibrary> {
        // Load file data
        let data = self.load_library_file(path)?;

        // Parse ELF header
        if data.len() < core::mem::size_of::<Elf64Header>() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("File too small for ELF header")
            ));
        }

        let header = unsafe {
            core::ptr::read(data.as_ptr() as *const Elf64Header)
        };

        // Verify ELF magic number
        if &header.e_ident[0..4] != b"\x7fELF" {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Invalid ELF magic number")
            ));
        }

        // Verify it's a shared object
        if header.e_type != elf_constants::ET_DYN {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Not a shared object")
            ));
        }

        // Parse program headers
        let phdr_offset = header.e_phoff as usize;
        let phdr_size = header.e_phentsize as usize;
        let phdr_num = header.e_phnum as usize;

        if phdr_offset + phdr_size * phdr_num > data.len() {
            return Err(DynamicLinkerError::InvalidElf(
                String::from("Program headers out of bounds")
            ));
        }

        let mut program_headers = Vec::new();
        for i in 0..phdr_num {
            let offset = phdr_offset + i * phdr_size;
            let phdr = unsafe {
                core::ptr::read(data[offset..].as_ptr() as *const Elf64ProgramHeader)
            };
            program_headers.push(phdr);
        }

        // Calculate total memory size needed
        let mut min_addr = u64::MAX;
        let mut max_addr = 0u64;
        for phdr in &program_headers {
            if phdr.p_type == elf_constants::PT_LOAD {
                let start = phdr.p_vaddr;
                let end = phdr.p_vaddr + phdr.p_memsz;
                min_addr = min_addr.min(start);
                max_addr = max_addr.max(end);
            }
        }

        let size = (max_addr - min_addr) as usize;
        let base_address = self.next_base_address;

        // Allocate memory for library (simplified - in real implementation would use memory manager)
        // For now, just update the next address
        self.next_base_address = VirtAddr::new(
            base_address.as_u64() + ((size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE) as u64
        );

        // Parse dynamic section
        let dynamic_info = self.parse_dynamic_section(
            &data,
            &program_headers,
            base_address
        ).unwrap_or_default();

        // Resolve library names
        let mut dynamic_info_mut = dynamic_info.clone();
        let _ = self.resolve_library_names(&data, &mut dynamic_info_mut);

        // Extract library name from path
        let name = path.split('/').last().unwrap_or(path).to_string();

        // Parse TLS information
        let tls_info = self.parse_tls_info(&program_headers, base_address);

        // Build symbol table for this library
        let symbols = self.parse_symbol_table(&data, &dynamic_info_mut)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, addr, _)| (name, VirtAddr::new(base_address.as_u64() + addr.as_u64())))
            .collect();

        Ok(LoadedLibrary {
            name,
            base_address,
            size,
            entry_point: if header.e_entry != 0 {
                Some(VirtAddr::new(base_address.as_u64() + header.e_entry))
            } else {
                None
            },
            dynamic_info: dynamic_info_mut.clone(),
            ref_count: 1,
            flags,
            tls_info,
            symbols,
            dependencies: dynamic_info_mut.needed.clone(),
            global: (flags & dlopen_flags::RTLD_GLOBAL) != 0,
            deletable: (flags & dlopen_flags::RTLD_NODELETE) == 0,
        })
    }

    /// Parse TLS information from program headers
    fn parse_tls_info(&self, program_headers: &[Elf64ProgramHeader], base: VirtAddr) -> Option<TlsInfo> {
        for phdr in program_headers {
            if phdr.p_type == elf_constants::PT_TLS {
                return Some(TlsInfo {
                    module_id: 0, // Will be assigned by TLS manager
                    offset: 0,
                    size: phdr.p_memsz as usize,
                    alignment: phdr.p_align as usize,
                    init_image: if phdr.p_filesz > 0 {
                        Some(VirtAddr::new(base.as_u64() + phdr.p_vaddr))
                    } else {
                        None
                    },
                });
            }
        }
        None
    }
}

impl Default for DynamicLinker {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic linker statistics
#[derive(Debug, Clone, Copy)]
pub struct DynamicLinkerStats {
    pub loaded_libraries: usize,
    pub global_symbols: usize,
    pub search_paths: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dynamic_linker_creation() {
        let linker = DynamicLinker::new();
        assert_eq!(linker.search_paths.len(), 5);
        assert!(linker.search_paths.contains(&String::from("/lib")));
    }
    
    #[test]
    fn test_add_search_path() {
        let mut linker = DynamicLinker::new();
        linker.add_search_path(String::from("/custom/lib"));
        assert!(linker.search_paths.contains(&String::from("/custom/lib")));
    }
    
    #[test]
    fn test_symbol_resolution() {
        let mut linker = DynamicLinker::new();
        let addr = VirtAddr::new(0x1000);
        linker.add_symbol(String::from("test_symbol"), addr);
        
        assert_eq!(linker.resolve_symbol("test_symbol"), Some(addr));
        assert_eq!(linker.resolve_symbol("nonexistent"), None);
    }
    
    #[test]
    fn test_string_table_reading() {
        let linker = DynamicLinker::new();
        let strtab = b"\x00hello\x00world\x00test\x00";
        
        assert_eq!(linker.read_string_from_table(strtab, 1), Some(String::from("hello")));
        assert_eq!(linker.read_string_from_table(strtab, 7), Some(String::from("world")));
        assert_eq!(linker.read_string_from_table(strtab, 13), Some(String::from("test")));
        assert_eq!(linker.read_string_from_table(strtab, 0), None); // Empty string
    }
    
    #[test]
    fn test_elf_symbol_binding() {
        let symbol = Elf64Symbol {
            st_name: 0,
            st_info: (symbol_binding::STB_GLOBAL << 4) | symbol_type::STT_FUNC,
            st_other: 0,
            st_shndx: 1,
            st_value: 0x1000,
            st_size: 100,
        };
        
        assert_eq!(symbol.binding(), symbol_binding::STB_GLOBAL);
        assert_eq!(symbol.symbol_type(), symbol_type::STT_FUNC);
        assert!(symbol.is_defined());
    }
    
    #[test]
    fn test_library_loaded_check() {
        let linker = DynamicLinker::new();
        assert!(!linker.is_loaded("libc.so.6"));
        assert_eq!(linker.loaded_libraries().len(), 0);
    }
    
    #[test]
    fn test_symbol_index_resolution() {
        let mut linker = DynamicLinker::new();
        
        // Manually populate symbol index table for testing
        linker.symbol_index_table.push((String::from("sym1"), VirtAddr::new(0x1000)));
        linker.symbol_index_table.push((String::from("sym2"), VirtAddr::new(0x2000)));
        linker.symbol_index_table.push((String::from(""), VirtAddr::new(0))); // Undefined
        
        // Test defined symbols
        assert_eq!(linker.resolve_symbol_by_index(0), Some(VirtAddr::new(0x1000)));
        assert_eq!(linker.resolve_symbol_by_index(1), Some(VirtAddr::new(0x2000)));
        
        // Test undefined symbol (should return None unless in global table)
        assert_eq!(linker.resolve_symbol_by_index(2), None);
        
        // Test out of bounds
        assert_eq!(linker.resolve_symbol_by_index(99), None);
    }
    
    #[test]
    fn test_linker_stats() {
        let mut linker = DynamicLinker::new();
        linker.add_symbol(String::from("test"), VirtAddr::new(0x1000));
        
        let stats = linker.get_stats();
        assert_eq!(stats.search_paths, 5);
        assert_eq!(stats.global_symbols, 1);
        assert_eq!(stats.loaded_libraries, 0);
    }
}

/// Global dynamic linker instance
lazy_static! {
    static ref GLOBAL_DYNAMIC_LINKER: Mutex<Option<DynamicLinker>> = Mutex::new(None);
}

/// Initialize the global dynamic linker
pub fn init_dynamic_linker() {
    *GLOBAL_DYNAMIC_LINKER.lock() = Some(DynamicLinker::new());
}

/// Get a reference to the global dynamic linker
/// 
/// # Safety
/// This function provides mutable access to the global dynamic linker.
/// Caller must ensure proper synchronization when using the returned reference.
pub fn with_dynamic_linker<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut DynamicLinker) -> R,
{
    let mut linker = GLOBAL_DYNAMIC_LINKER.lock();
    linker.as_mut().map(f)
}

/// Link a binary using the global dynamic linker
/// 
/// This is a convenience function that can be called from the process module
/// to handle dynamic linking during process execution.
/// 
/// # Arguments
/// * `binary_data` - The ELF binary data
/// * `program_headers` - Program headers from the ELF
/// * `base_address` - Base address where binary is loaded
/// 
/// # Returns
/// Number of relocations applied, or error message
pub fn link_binary_globally(
    binary_data: &[u8],
    program_headers: &[super::elf_loader::Elf64ProgramHeader],
    base_address: VirtAddr,
) -> Result<usize, &'static str> {
    let mut linker = get_dynamic_linker()
        .ok_or("Dynamic linker not initialized")?;
    
    linker.link_binary(binary_data, program_headers, base_address)
        .map_err(|_| "Failed to link binary")
}

// =============================================================================
// Global Helper Functions
// =============================================================================

/// Get a reference to the global dynamic linker
fn get_dynamic_linker() -> Option<DynamicLinker> {
    let linker_guard = GLOBAL_DYNAMIC_LINKER.lock();
    (*linker_guard).clone()
}

/// Get VFS for file operations
fn get_vfs_manager() -> Option<&'static crate::fs::VFS> {
    Some(crate::fs::get_vfs())
}

/// Call init functions for a loaded library
pub fn call_library_init(lib: &LoadedLibrary) {
    if let Some(init_addr) = lib.dynamic_info.init {
        unsafe {
            let init_fn: extern "C" fn() = core::mem::transmute(init_addr.as_u64());
            init_fn();
        }
    }
}

/// Call fini functions for a library being unloaded
pub fn call_library_fini(lib: &LoadedLibrary) {
    if let Some(fini_addr) = lib.dynamic_info.fini {
        unsafe {
            let fini_fn: extern "C" fn() = core::mem::transmute(fini_addr.as_u64());
            fini_fn();
        }
    }
}

// =============================================================================
// Public API Functions for dlopen/dlsym/dlclose
// =============================================================================

/// dlopen() - Open a shared library (global API)
pub fn dlopen(filename: Option<&str>, flags: i32) -> Result<DlHandle, &'static str> {
    with_dynamic_linker(|linker| {
        linker.dlopen(filename, flags)
            .map_err(|_| "Failed to open library")
    })
    .ok_or("Dynamic linker not initialized")?
}

/// dlsym() - Look up a symbol (global API)
pub fn dlsym(handle: DlHandle, symbol: &str) -> Result<VirtAddr, &'static str> {
    with_dynamic_linker(|linker| {
        linker.dlsym(handle, symbol)
            .map_err(|_| "Symbol not found")
    })
    .ok_or("Dynamic linker not initialized")?
}

/// dlclose() - Close a library (global API)
pub fn dlclose(handle: DlHandle) -> Result<(), &'static str> {
    with_dynamic_linker(|linker| {
        linker.dlclose(handle)
            .map_err(|_| "Failed to close library")
    })
    .ok_or("Dynamic linker not initialized")?
}

/// dlerror() - Get last error message (global API)
pub fn dlerror() -> Option<String> {
    with_dynamic_linker(|linker| {
        linker.dlerror()
    })
    .flatten()
}
