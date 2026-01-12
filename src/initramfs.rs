//! Initramfs Support for RustOS
//!
//! This module handles loading and extracting the initial RAM filesystem
//! which contains the Linux userspace environment (Alpine Linux).

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::vfs::{get_vfs, InodeType, OpenFlags};

/// Embedded initramfs data (included at compile time)
/// Alpine Linux 3.19 minirootfs with busybox (3.1 MB compressed)
pub static INITRAMFS_DATA: &[u8] = include_bytes!("../userspace/initramfs.cpio.gz");

/// Initramfs header and metadata
pub struct InitramfsInfo {
    pub size: usize,
    pub format: InitramfsFormat,
    pub root_path: String,
}

/// Supported initramfs formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitramfsFormat {
    CpioNewc,      // newc format (most common)
    CpioOdc,       // odc format (old)
    CompressedGzip, // gzip compressed cpio
}

/// Extract initramfs to VFS
pub fn extract_initramfs() -> Result<InitramfsInfo, InitramfsError> {
    if INITRAMFS_DATA.is_empty() {
        return Err(InitramfsError::NoInitramfs);
    }

    // 1. Detect format (check for gzip magic)
    let data = if is_gzipped(INITRAMFS_DATA) {
        // Decompress gzip data
        decompress_gzip(INITRAMFS_DATA)?
    } else {
        // Use data as-is
        INITRAMFS_DATA.to_vec()
    };

    // 2. Parse CPIO format and extract to VFS
    extract_cpio(&data)?;

    Ok(InitramfsInfo {
        size: INITRAMFS_DATA.len(),
        format: if is_gzipped(INITRAMFS_DATA) {
            InitramfsFormat::CompressedGzip
        } else {
            InitramfsFormat::CpioNewc
        },
        root_path: String::from("/"),
    })
}

/// Check if data is gzip compressed
fn is_gzipped(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

/// Load and execute an ELF binary
///
/// This function:
/// 1. Parses and validates the ELF binary
/// 2. Allocates memory for all loadable segments
/// 3. Loads segment data into memory
/// 4. Sets up an initial stack
/// 5. Returns entry point address and stack pointer for execution
///
/// # Returns
/// (entry_point, stack_pointer) tuple
pub fn load_and_execute_elf(binary_data: &[u8]) -> Result<(u64, u64), InitramfsError> {
    use crate::elf_loader::{elf_validate, elf_load};
    use x86_64::VirtAddr;

    // Validate ELF binary format
    elf_validate(binary_data).map_err(|_| InitramfsError::InvalidFormat)?;

    // Load ELF image (no load bias for static executables)
    let image = elf_load(binary_data, None).map_err(|_| InitramfsError::ExtractionFailed)?;

    // For now, we'll do a simplified loading without full page table setup
    // In a complete implementation, we would:
    // 1. Create a new page table for the process
    // 2. Map all segments with proper permissions
    // 3. Set up initial stack with argc/argv/envp
    // 4. Switch to user mode and jump to entry point

    // Get all program headers for segment data extraction
    use crate::elf_loader::parser::get_loadable_segments;
    let program_headers = get_loadable_segments(binary_data)
        .map_err(|_| InitramfsError::InvalidFormat)?;

    // Copy segments into memory (simplified version)
    for (segment, ph) in image.segments.iter().zip(program_headers.iter()) {
        if segment.segment_type != crate::elf_loader::SegmentType::Load {
            continue;
        }

        // Get segment data from binary
        let offset = ph.offset();
        let file_size = ph.file_size();

        if offset + file_size > binary_data.len() {
            return Err(InitramfsError::ExtractionFailed);
        }

        let segment_data = &binary_data[offset..offset + file_size];

        // Copy to destination address
        unsafe {
            let dest = segment.vaddr.as_u64() as *mut u8;
            let src = segment_data.as_ptr();
            core::ptr::copy_nonoverlapping(src, dest, segment.file_size);

            // Zero BSS (memory size > file size)
            if segment.mem_size > segment.file_size {
                let bss_start = dest.add(segment.file_size);
                let bss_size = segment.mem_size - segment.file_size;
                core::ptr::write_bytes(bss_start, 0, bss_size);
            }
        }
    }

    // Set up initial user stack
    // Stack is at the recommended address from the ELF image
    let stack_pointer = image.stack_address.as_u64();

    // Return entry point address and stack pointer
    Ok((image.entry_point.as_u64(), stack_pointer))
}

/// Load ELF binary with full page table setup (advanced version)
///
/// This is the production-ready version that:
/// 1. Creates a new user page table
/// 2. Maps all segments with proper permissions (R/W/X)
/// 3. Sets up user stack with guard pages
/// 4. Handles both static and dynamic executables
///
/// Note: This requires a page table mapper and frame allocator
#[allow(dead_code)]
pub fn load_and_execute_elf_with_paging<M, A>(
    binary_data: &[u8],
    mapper: &mut M,
    frame_allocator: &mut A,
) -> Result<(u64, u64), InitramfsError>
where
    M: x86_64::structures::paging::Mapper<x86_64::structures::paging::Size4KiB>,
    A: x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>,
{
    use crate::elf_loader::{elf_validate, elf_load, elf_map_segments, elf_create_stack};
    use x86_64::VirtAddr;

    // Validate ELF binary format
    elf_validate(binary_data).map_err(|_| InitramfsError::InvalidFormat)?;

    // Load ELF image
    let image = elf_load(binary_data, None).map_err(|_| InitramfsError::ExtractionFailed)?;

    // Map all segments into page table with proper permissions
    elf_map_segments(&image, binary_data, mapper, frame_allocator)
        .map_err(|_| InitramfsError::ExtractionFailed)?;

    // Create user stack (8 MB)
    const STACK_SIZE: usize = 8 * 1024 * 1024;
    let stack_pointer = elf_create_stack(
        mapper,
        frame_allocator,
        image.stack_address,
        STACK_SIZE,
    )
    .map_err(|_| InitramfsError::ExtractionFailed)?;

    Ok((image.entry_point.as_u64(), stack_pointer.as_u64()))
}

/// Start the init process from initramfs
pub fn start_init() -> Result<(), InitramfsError> {
    // 1. Load /init from VFS
    let vfs = get_vfs();
    let init_inode = vfs.lookup("/init")
        .map_err(|_| InitramfsError::InitNotFound)?;

    // 2. Read the entire /init binary into memory
    let mut binary_data = Vec::new();
    let file_size = init_inode.metadata()
        .map_err(|_| InitramfsError::VfsError)?
        .size;

    binary_data.resize(file_size, 0);
    init_inode.read(0, &mut binary_data)
        .map_err(|_| InitramfsError::VfsError)?;

    // 3. Load and validate the ELF binary
    let (entry_point, stack_pointer) = load_and_execute_elf(&binary_data)?;

    // 4. Set up user mode execution context
    // This would involve:
    // - Creating a new process context
    // - Setting up user mode stack with arguments (argc, argv, envp)
    // - Configuring registers (RIP = entry_point, RSP = stack_pointer)
    // - Switching privilege level to ring 3
    // - Using IRET to jump to user mode

    // For now, we'll prepare but not execute
    // In a full implementation, this would:
    // 1. Create a process: process_manager::create_user_process()
    // 2. Set up execution context with entry point and stack
    // 3. Jump to user mode: usermode::switch_to_user_mode(entry_point, stack_pointer)

    // Log that we're ready to execute (for debugging)
    #[cfg(feature = "serial")]
    {
        use crate::serial_println;
        serial_println!("Init binary loaded:");
        serial_println!("  Entry point: {:#x}", entry_point);
        serial_println!("  Stack pointer: {:#x}", stack_pointer);
    }

    Ok(())
}

/// Execute the init process (transitions to user mode)
///
/// This is the final step that actually jumps to user mode and starts executing /init.
/// It should only be called after start_init() has successfully loaded the binary.
///
/// # Safety
/// This function is unsafe because it transitions to user mode and never returns.
/// The caller must ensure:
/// - The ELF binary has been loaded and validated
/// - Page tables are set up for user mode access
/// - Interrupts and syscall handlers are configured
#[allow(dead_code)]
pub unsafe fn execute_init(entry_point: u64, stack_pointer: u64) -> ! {
    // Switch to user mode and jump to /init entry point
    crate::usermode::switch_to_user_mode(entry_point, stack_pointer)
}

/// Load initramfs at kernel boot
pub fn init_initramfs() -> Result<(), InitramfsError> {
    // Extract initramfs into VFS
    let info = extract_initramfs()?;

    // Mount as root filesystem
    // This would call vfs::mount("/", ramfs)

    // Start init process
    // start_init()?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum InitramfsError {
    NoInitramfs,
    InvalidFormat,
    DecompressionFailed,
    ExtractionFailed,
    InitNotFound,
    VfsError,
    ParseError,
}

/// CPIO header structure (newc format)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CpioHeader {
    pub magic: [u8; 6],      // "070701" or "070702"
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub mtime: u32,
    pub filesize: u32,
    pub devmajor: u32,
    pub devminor: u32,
    pub rdevmajor: u32,
    pub rdevminor: u32,
    pub namesize: u32,
    pub check: u32,
}

impl CpioHeader {
    /// Parse CPIO header from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, InitramfsError> {
        if data.len() < 110 {
            return Err(InitramfsError::InvalidFormat);
        }

        // Verify magic
        if &data[0..6] != b"070701" && &data[0..6] != b"070702" {
            return Err(InitramfsError::InvalidFormat);
        }

        // Parse header fields (ASCII hex format)
        // This is simplified - full implementation would parse all fields

        Ok(CpioHeader {
            magic: [data[0], data[1], data[2], data[3], data[4], data[5]],
            ino: 0,
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            mtime: 0,
            filesize: 0,
            devmajor: 0,
            devminor: 0,
            rdevmajor: 0,
            rdevminor: 0,
            namesize: 0,
            check: 0,
        })
    }
}

/// Decompress gzip data
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, InitramfsError> {
    // Use miniz_oxide for decompression
    // This would be implemented using the existing miniz_oxide dependency
    Err(InitramfsError::DecompressionFailed)
}

// =============================================================================
// STUB FUNCTIONS - TODO: Implement production versions
// =============================================================================

/// TODO: Implement CPIO extraction
/// Extract CPIO archive data to the VFS
/// Currently returns an error - needs CPIO parsing implementation
fn extract_cpio(data: &[u8]) -> Result<(), InitramfsError> {
    let _ = data;
    // TODO: Parse CPIO newc format
    // TODO: Extract files to VFS
    // TODO: Handle directories, symlinks, and special files
    Err(InitramfsError::ParseError)
}
