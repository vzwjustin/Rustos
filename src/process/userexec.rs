//! User Program Execution
//!
//! This module implements the complete user program execution pipeline for RustOS.
//! It orchestrates ELF loading, memory setup, stack initialization, and Ring 3 transition.
//!
//! # Pipeline
//! 1. Load ELF binary from filesystem
//! 2. Parse and validate ELF headers
//! 3. Map segments to user memory (.text, .data, .bss, .rodata)
//! 4. Handle dynamic linking if needed
//! 5. Set up user heap (brk)
//! 6. Allocate and initialize user stack
//! 7. Push argc, argv, envp, auxiliary vector
//! 8. Transition to Ring 3 (user mode)
//! 9. Jump to entry point
//!
//! # Security
//! - All user memory is validated and bounded
//! - NX bit enforcement (No-Execute protection)
//! - W^X enforcement (Write XOR Execute)
//! - Stack guard pages
//! - ASLR support
//!
//! # Example Usage
//! ```rust
//! use crate::process::userexec;
//!
//! // Execute /bin/hello with arguments
//! let argv = vec!["hello".to_string(), "world".to_string()];
//! let envp = vec!["PATH=/bin:/usr/bin".to_string()];
//!
//! match userexec::exec_user_program("/bin/hello", &argv, &envp) {
//!     Ok(pid) => println!("Started process {}", pid),
//!     Err(e) => println!("Failed to execute: {}", e),
//! }
//! ```

use alloc::vec::{self, Vec};
use alloc::string::{String, ToString};
use x86_64::{VirtAddr, PhysAddr};
use core::arch::asm;

use super::{Pid, ProcessControlBlock, ProcessState, Priority, get_process_manager};
use super::elf_loader::{ElfLoader, LoadedBinary, ElfLoaderError};
use super::dynamic_linker::{DynamicLinker, init_dynamic_linker};
use crate::memory::{MemoryRegionType, MemoryProtection, allocate_memory, PAGE_SIZE};
use crate::gdt;
use crate::fs;

/// Auxiliary vector entry types (AT_* constants from Linux)
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum AuxvType {
    /// End of vector
    Null = 0,
    /// Entry point of the program
    Entry = 9,
    /// Program headers address
    Phdr = 3,
    /// Program header count
    Phnum = 5,
    /// Program header size
    Phent = 4,
    /// Base address of interpreter
    Base = 7,
    /// Page size
    Pagesz = 6,
    /// Random bytes for stack canary
    Random = 25,
    /// UID
    Uid = 11,
    /// EUID
    Euid = 12,
    /// GID
    Gid = 13,
    /// EGID
    Egid = 14,
}

/// Auxiliary vector entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuxvEntry {
    pub a_type: u64,
    pub a_val: u64,
}

/// User program execution error
#[derive(Debug)]
pub enum UserExecError {
    /// File not found
    FileNotFound(String),
    /// Invalid ELF binary
    InvalidElf(ElfLoaderError),
    /// Memory allocation failed
    OutOfMemory,
    /// Process creation failed
    ProcessCreationFailed(&'static str),
    /// Stack setup failed
    StackSetupFailed,
    /// Dynamic linking failed
    DynamicLinkingFailed,
    /// Invalid arguments
    InvalidArguments,
}

impl From<ElfLoaderError> for UserExecError {
    fn from(err: ElfLoaderError) -> Self {
        UserExecError::InvalidElf(err)
    }
}

/// Execute a user program from an ELF binary file
///
/// This is the main entry point for executing user programs. It handles the complete
/// pipeline from loading the ELF binary to jumping to user mode.
///
/// # Arguments
/// * `path` - Path to the ELF binary file
/// * `argv` - Command-line arguments (argv[0] should be program name)
/// * `envp` - Environment variables
///
/// # Returns
/// * `Ok(Pid)` - Process ID of the new user process
/// * `Err(UserExecError)` - Error description
///
/// # Example
/// ```rust
/// let argv = vec!["hello".to_string()];
/// let envp = vec!["PATH=/bin".to_string()];
/// let pid = exec_user_program("/bin/hello", &argv, &envp)?;
/// ```
pub fn exec_user_program(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<Pid, UserExecError> {
    // Step 1: Load ELF binary from filesystem
    let binary_data = load_binary_from_filesystem(path)?;

    // Step 2: Create process control block
    let process_manager = get_process_manager();
    let current_pid = process_manager.current_process();
    let parent_pid = if current_pid == 0 { None } else { Some(current_pid) };

    let pid = process_manager.create_process(
        path,
        parent_pid,
        Priority::Normal,
    ).map_err(UserExecError::ProcessCreationFailed)?;

    // Step 3: Load and parse ELF binary
    let elf_loader = ElfLoader::new(true, true); // Enable ASLR and NX
    let loaded_binary = elf_loader.load_elf_binary(&binary_data, pid)?;

    // Step 4: Handle dynamic linking if needed
    if loaded_binary.is_dynamic {
        init_dynamic_linker();
        handle_dynamic_linking(&binary_data, &loaded_binary)?;
    }

    // Step 5: Set up user stack with argc, argv, envp, auxv
    let stack_top = setup_user_stack(
        &loaded_binary,
        argv,
        envp,
        pid,
    )?;

    // Step 6: Update process control block with loaded binary info
    update_pcb_with_binary_info(pid, &loaded_binary, stack_top)?;

    // Step 7: Set up kernel stack in TSS for syscall/interrupt handling
    setup_kernel_stack_for_process(pid)?;

    // Step 8: Transition to Ring 3 and jump to entry point
    transition_to_user_mode(
        loaded_binary.entry_point,
        stack_top,
        pid,
    );

    // This should not return - execution continues in user mode
    Ok(pid)
}

/// Load ELF binary from filesystem
fn load_binary_from_filesystem(path: &str) -> Result<Vec<u8>, UserExecError> {
    // Get file metadata to determine size
    let metadata = fs::vfs().stat(path)
        .map_err(|_| UserExecError::FileNotFound(path.to_string()))?;

    // Open file
    let fd = fs::vfs().open(path, fs::OpenFlags::read_only())
        .map_err(|_| UserExecError::FileNotFound(path.to_string()))?;

    // Read entire file
    let file_size = metadata.size as usize;
    let mut buffer = alloc::vec![0u8; file_size];

    let bytes_read = fs::vfs().read(fd, &mut buffer)
        .map_err(|_| UserExecError::FileNotFound(path.to_string()))?;

    // Close file
    let _ = fs::vfs().close(fd);

    if bytes_read != file_size {
        buffer.truncate(bytes_read);
    }

    Ok(buffer)
}

/// Handle dynamic linking for dynamically-linked ELF binaries
fn handle_dynamic_linking(
    binary_data: &[u8],
    loaded_binary: &LoadedBinary,
) -> Result<(), UserExecError> {
    use crate::process::dynamic_linker::link_binary_globally;

    // Use the global dynamic linker to resolve symbols and apply relocations
    link_binary_globally(
        binary_data,
        &loaded_binary.program_headers,
        loaded_binary.base_address,
    ).map_err(|_| UserExecError::DynamicLinkingFailed)?;

    Ok(())
}

/// Set up user stack with argc, argv, envp, and auxiliary vector
///
/// Stack layout (from high to low address):
/// ```text
/// [Stack Top - 16-byte aligned]
/// NULL
/// envp[n-1]
/// ...
/// envp[0]
/// NULL
/// argv[argc-1]
/// ...
/// argv[0]
/// argc
/// [Auxiliary Vector]
/// AT_NULL
/// ...
/// AT_PHDR
/// AT_ENTRY
/// [String Data]
/// envp strings
/// argv strings
/// [16-byte alignment padding]
/// ```
fn setup_user_stack(
    loaded_binary: &LoadedBinary,
    argv: &[String],
    envp: &[String],
    _pid: Pid,
) -> Result<VirtAddr, UserExecError> {
    let stack_top = loaded_binary.stack_top;
    let mut sp = stack_top.as_u64();

    // Ensure 16-byte alignment at the start
    sp &= !0xF;

    // Step 1: Copy argv strings to stack (from high to low addresses)
    let mut argv_ptrs = Vec::with_capacity(argv.len());
    for arg in argv.iter().rev() {
        sp -= arg.len() as u64 + 1; // +1 for null terminator
        copy_string_to_user_stack(sp, arg)?;
        argv_ptrs.push(sp);
    }
    argv_ptrs.reverse();

    // Step 2: Copy envp strings to stack
    let mut envp_ptrs = Vec::with_capacity(envp.len());
    for env in envp.iter().rev() {
        sp -= env.len() as u64 + 1; // +1 for null terminator
        copy_string_to_user_stack(sp, env)?;
        envp_ptrs.push(sp);
    }
    envp_ptrs.reverse();

    // Step 3: Align stack pointer to 16 bytes for data structures
    sp &= !0xF;

    // Step 4: Push auxiliary vector
    sp = push_auxiliary_vector(sp, loaded_binary)?;

    // Step 5: Push envp array (pointers) - NULL terminated
    sp -= 8; // NULL terminator
    write_u64_to_stack(sp, 0)?;

    for &ptr in envp_ptrs.iter().rev() {
        sp -= 8;
        write_u64_to_stack(sp, ptr)?;
    }

    // Step 6: Push argv array (pointers) - NULL terminated
    sp -= 8; // NULL terminator
    write_u64_to_stack(sp, 0)?;

    for &ptr in argv_ptrs.iter().rev() {
        sp -= 8;
        write_u64_to_stack(sp, ptr)?;
    }

    // Step 7: Push argc
    sp -= 8;
    write_u64_to_stack(sp, argv.len() as u64)?;

    // Final alignment check
    sp &= !0xF;

    Ok(VirtAddr::new(sp))
}

/// Push auxiliary vector to stack
///
/// The auxiliary vector provides information to the dynamic linker and user program
fn push_auxiliary_vector(
    mut sp: u64,
    loaded_binary: &LoadedBinary,
) -> Result<u64, UserExecError> {
    let auxv = build_auxiliary_vector(loaded_binary);

    // Push in reverse order (from high to low address)
    for entry in auxv.iter().rev() {
        sp -= 16; // Each auxv entry is 16 bytes (2 x u64)
        write_u64_to_stack(sp, entry.a_type)?;
        write_u64_to_stack(sp + 8, entry.a_val)?;
    }

    Ok(sp)
}

/// Build auxiliary vector for the program
fn build_auxiliary_vector(loaded_binary: &LoadedBinary) -> Vec<AuxvEntry> {
    let mut auxv = Vec::new();

    // AT_PAGESZ - Page size
    auxv.push(AuxvEntry {
        a_type: AuxvType::Pagesz as u64,
        a_val: PAGE_SIZE as u64,
    });

    // AT_PHDR - Program headers address
    if !loaded_binary.program_headers.is_empty() {
        auxv.push(AuxvEntry {
            a_type: AuxvType::Phdr as u64,
            a_val: loaded_binary.base_address.as_u64(), // Approximate - should be actual phdr location
        });
    }

    // AT_PHENT - Program header entry size
    auxv.push(AuxvEntry {
        a_type: AuxvType::Phent as u64,
        a_val: 56, // sizeof(Elf64_Phdr)
    });

    // AT_PHNUM - Number of program headers
    auxv.push(AuxvEntry {
        a_type: AuxvType::Phnum as u64,
        a_val: loaded_binary.program_headers.len() as u64,
    });

    // AT_BASE - Base address (for interpreter)
    auxv.push(AuxvEntry {
        a_type: AuxvType::Base as u64,
        a_val: loaded_binary.base_address.as_u64(),
    });

    // AT_ENTRY - Entry point
    auxv.push(AuxvEntry {
        a_type: AuxvType::Entry as u64,
        a_val: loaded_binary.entry_point.as_u64(),
    });

    // AT_UID, AT_EUID, AT_GID, AT_EGID
    auxv.push(AuxvEntry { a_type: AuxvType::Uid as u64, a_val: 0 });
    auxv.push(AuxvEntry { a_type: AuxvType::Euid as u64, a_val: 0 });
    auxv.push(AuxvEntry { a_type: AuxvType::Gid as u64, a_val: 0 });
    auxv.push(AuxvEntry { a_type: AuxvType::Egid as u64, a_val: 0 });

    // AT_RANDOM - Random bytes for stack canary (16 bytes)
    // In a real implementation, we would provide actual random data
    auxv.push(AuxvEntry {
        a_type: AuxvType::Random as u64,
        a_val: 0x0, // Placeholder - should point to 16 random bytes
    });

    // AT_NULL - End of auxiliary vector
    auxv.push(AuxvEntry {
        a_type: AuxvType::Null as u64,
        a_val: 0,
    });

    auxv
}

/// Copy a string to user stack memory
fn copy_string_to_user_stack(addr: u64, s: &str) -> Result<(), UserExecError> {
    use crate::memory::user_space::UserSpaceMemory;

    // Validate user pointer
    let len = s.len() as u64 + 1; // +1 for null terminator
    UserSpaceMemory::validate_user_ptr(addr, len, true)
        .map_err(|_| UserExecError::StackSetupFailed)?;

    // Copy string bytes
    let bytes = s.as_bytes();
    unsafe {
        let ptr = addr as *mut u8;
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        // Add null terminator
        *ptr.add(bytes.len()) = 0;
    }

    Ok(())
}

/// Write a u64 value to stack
fn write_u64_to_stack(addr: u64, value: u64) -> Result<(), UserExecError> {
    use crate::memory::user_space::UserSpaceMemory;

    // Validate user pointer
    UserSpaceMemory::validate_user_ptr(addr, 8, true)
        .map_err(|_| UserExecError::StackSetupFailed)?;

    unsafe {
        let ptr = addr as *mut u64;
        core::ptr::write(ptr, value);
    }

    Ok(())
}

/// Update process control block with loaded binary information
fn update_pcb_with_binary_info(
    pid: Pid,
    loaded_binary: &LoadedBinary,
    stack_top: VirtAddr,
) -> Result<(), UserExecError> {
    let process_manager = get_process_manager();

    let mut pcb = process_manager.get_process(pid)
        .ok_or(UserExecError::ProcessCreationFailed("Process not found"))?;

    // Update memory information
    if !loaded_binary.code_regions.is_empty() {
        pcb.memory.code_start = loaded_binary.code_regions[0].start.as_u64();
        pcb.memory.code_size = loaded_binary.code_regions.iter()
            .map(|r| r.size as u64)
            .sum();
    }

    if !loaded_binary.data_regions.is_empty() {
        pcb.memory.data_start = loaded_binary.data_regions[0].start.as_u64();
        pcb.memory.data_size = loaded_binary.data_regions.iter()
            .map(|r| r.size as u64)
            .sum();
    }

    pcb.memory.heap_start = loaded_binary.heap_start.as_u64();
    pcb.memory.heap_size = 8 * 1024; // Initial 8KB heap

    pcb.memory.stack_start = stack_top.as_u64();
    pcb.memory.stack_size = 8 * 1024 * 1024; // 8MB stack

    // Set entry point
    pcb.entry_point = loaded_binary.entry_point.as_u64();

    // Set initial CPU context for Ring 3
    pcb.context.rip = loaded_binary.entry_point.as_u64();
    pcb.context.rsp = stack_top.as_u64();
    pcb.context.rflags = 0x202; // Interrupts enabled, bit 1 always set

    // Set Ring 3 segment selectors
    pcb.context.cs = gdt::get_user_code_selector().0;
    pcb.context.ds = gdt::get_user_data_selector().0;
    pcb.context.es = gdt::get_user_data_selector().0;
    pcb.context.fs = gdt::get_user_data_selector().0;
    pcb.context.gs = gdt::get_user_data_selector().0;
    pcb.context.ss = gdt::get_user_data_selector().0;

    Ok(())
}

/// Set up kernel stack for the process in the TSS
///
/// This is required for handling syscalls and interrupts from user mode.
/// When a user process makes a syscall or receives an interrupt, the CPU
/// automatically switches to the kernel stack specified in the TSS.
fn setup_kernel_stack_for_process(pid: Pid) -> Result<(), UserExecError> {
    // Allocate kernel stack for this process
    let kernel_stack_size = 16 * 1024; // 16KB kernel stack
    let kernel_stack = allocate_memory(
        kernel_stack_size,
        MemoryRegionType::KernelStack,
        MemoryProtection::KERNEL_DATA,
    ).map_err(|_| UserExecError::OutOfMemory)?;

    // Stack grows downward, so top is at base + size
    let kernel_stack_top = VirtAddr::new(kernel_stack.as_u64() + kernel_stack_size as u64);

    // Set kernel stack in TSS (RSP0 - Ring 0 stack pointer)
    gdt::set_kernel_stack(kernel_stack_top);

    Ok(())
}

/// Transition to Ring 3 (user mode) and jump to entry point
///
/// This function performs the actual privilege level change from Ring 0 (kernel)
/// to Ring 3 (user mode) and transfers control to the user program's entry point.
///
/// # Arguments
/// * `entry_point` - Virtual address of the user program's entry point
/// * `stack_pointer` - Top of the user stack
/// * `pid` - Process ID
///
/// # Note
/// This function does not return - execution continues in user mode
fn transition_to_user_mode(
    entry_point: VirtAddr,
    stack_pointer: VirtAddr,
    pid: Pid,
) -> ! {
    let process_manager = get_process_manager();

    // Mark process as running
    if let Some(mut pcb) = process_manager.get_process(pid) {
        pcb.state = ProcessState::Running;
    }

    process_manager.set_current_process(pid);

    // Get Ring 3 segment selectors
    let user_cs = gdt::get_user_code_selector();
    let user_ds = gdt::get_user_data_selector();

    crate::serial_println!("Transitioning to Ring 3:");
    crate::serial_println!("  Entry: {:?}", entry_point);
    crate::serial_println!("  Stack: {:?}", stack_pointer);
    crate::serial_println!("  CS: 0x{:x}", user_cs.0);
    crate::serial_println!("  DS: 0x{:x}", user_ds.0);

    unsafe {
        // Set data segment selectors to user data segment
        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov fs, {0:x}",
            "mov gs, {0:x}",
            in(reg) user_ds.0,
            options(nostack, preserves_flags)
        );

        // Prepare IRETQ frame on stack:
        // SS (user data segment)
        // RSP (user stack pointer)
        // RFLAGS (0x202 - interrupts enabled)
        // CS (user code segment)
        // RIP (entry point)

        // Build IRETQ frame and jump to Ring 3
        asm!(
            // Push SS (Stack Segment)
            "push {user_ss}",
            // Push RSP (Stack Pointer)
            "push {rsp}",
            // Push RFLAGS (0x202 = interrupts enabled + reserved bit 1)
            "push 0x202",
            // Push CS (Code Segment)
            "push {user_cs}",
            // Push RIP (Instruction Pointer - entry point)
            "push {rip}",
            // Execute IRETQ to switch to Ring 3
            "iretq",
            user_ss = in(reg) user_ds.0 as u64,
            rsp = in(reg) stack_pointer.as_u64(),
            user_cs = in(reg) user_cs.0 as u64,
            rip = in(reg) entry_point.as_u64(),
            options(noreturn)
        );
    }
}

/// Execute a program in the current process (replace current process image)
///
/// This is similar to the Unix execve() syscall - it replaces the current
/// process's memory space with a new program.
///
/// # Arguments
/// * `path` - Path to the ELF binary
/// * `argv` - Command-line arguments
/// * `envp` - Environment variables
///
/// # Returns
/// This function does not return on success - the process is replaced.
/// If it returns, an error occurred.
///
/// # Note
/// To use the proper never type (!), enable #![feature(never_type)]
#[allow(unreachable_code)]
pub fn exec_replace_current(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<(), UserExecError> {
    let process_manager = get_process_manager();
    let current_pid = process_manager.current_process();

    if current_pid == 0 {
        return Err(UserExecError::ProcessCreationFailed("Cannot exec kernel process"));
    }

    // Clean up current process memory
    // (In a full implementation, we would deallocate all user memory)

    // Load new ELF binary
    let binary_data = load_binary_from_filesystem(path)?;
    let elf_loader = ElfLoader::new(true, true);
    let loaded_binary = elf_loader.load_elf_binary(&binary_data, current_pid)?;

    // Handle dynamic linking
    if loaded_binary.is_dynamic {
        init_dynamic_linker();
        handle_dynamic_linking(&binary_data, &loaded_binary)?;
    }

    // Set up new stack
    let stack_top = setup_user_stack(&loaded_binary, argv, envp, current_pid)?;

    // Update PCB
    update_pcb_with_binary_info(current_pid, &loaded_binary, stack_top)?;

    // Set up kernel stack
    setup_kernel_stack_for_process(current_pid)?;

    // Jump to new program
    transition_to_user_mode(
        loaded_binary.entry_point,
        stack_top,
        current_pid,
    );
}

/// Fork the current process and execute a new program in the child
///
/// This is a combined fork+exec operation commonly used for spawning new processes.
///
/// # Arguments
/// * `path` - Path to the ELF binary
/// * `argv` - Command-line arguments
/// * `envp` - Environment variables
///
/// # Returns
/// * Parent: Child PID
/// * Child: Does not return (executes new program)
pub fn fork_and_exec(
    path: &str,
    argv: &[String],
    envp: &[String],
) -> Result<Pid, UserExecError> {
    let process_manager = get_process_manager();
    let current_pid = process_manager.current_process();

    // Fork current process
    use crate::process::integration::get_integration_manager;
    let integration = get_integration_manager();
    let child_pid = integration.fork_process(current_pid)
        .map_err(|e| UserExecError::ProcessCreationFailed(e))?;

    // In child process (child_pid would be 0 in actual fork implementation)
    // For now, we assume we're in the parent and return the child PID
    // The child would execute the new program in its context

    // Load and execute program in child process
    // Note: This is simplified - in reality, the child would do this
    exec_user_program(path, argv, envp)?;

    Ok(child_pid)
}

/// Spawn a new user process with default environment
///
/// This is a convenience function for testing and simple program execution.
///
/// # Arguments
/// * `path` - Path to the ELF binary
///
/// # Returns
/// Process ID of the new process
pub fn spawn_user_process(path: &str) -> Result<Pid, UserExecError> {
    use alloc::vec;

    let argv = vec![path.to_string()];
    let envp = vec![
        "PATH=/bin:/usr/bin".to_string(),
        "HOME=/root".to_string(),
        "TERM=rustos".to_string(),
    ];

    exec_user_program(path, &argv, &envp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auxiliary_vector_building() {
        use crate::process::elf_loader::LoadedBinary;

        let loaded_binary = LoadedBinary {
            base_address: VirtAddr::new(0x400000),
            entry_point: VirtAddr::new(0x401000),
            heap_start: VirtAddr::new(0x600000),
            stack_top: VirtAddr::new(0x7FFFFFFF0),
            code_regions: alloc::vec![],
            data_regions: alloc::vec![],
            is_dynamic: false,
            program_headers: alloc::vec![],
        };

        let auxv = build_auxiliary_vector(&loaded_binary);

        // Should have at least AT_ENTRY and AT_NULL
        assert!(auxv.len() >= 2);

        // Last entry should be AT_NULL
        assert_eq!(auxv.last().unwrap().a_type, AuxvType::Null as u64);

        // Should contain AT_ENTRY
        assert!(auxv.iter().any(|e| e.a_type == AuxvType::Entry as u64));
    }

    #[test]
    fn test_auxv_entry_size() {
        use core::mem::size_of;

        // Verify AuxvEntry is exactly 16 bytes (2 x u64)
        assert_eq!(size_of::<AuxvEntry>(), 16);
    }

    #[test]
    fn test_user_exec_error_conversion() {
        let elf_error = ElfLoaderError::InvalidMagic;
        let user_error: UserExecError = elf_error.into();

        match user_error {
            UserExecError::InvalidElf(_) => {}, // Expected
            _ => panic!("Unexpected error type"),
        }
    }
}
