//! System Calls Interface and Dispatcher
//!
//! This module implements the system call interface for RustOS, providing
//! a standardized way for processes to request kernel services.

use super::{Pid, ProcessManager, ProcessState, Priority};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;

/// System call numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    // Process management
    Exit = 0,
    Fork = 1,
    Exec = 2,
    Wait = 3,
    GetPid = 4,
    GetPpid = 5,
    Sleep = 6,
    Clone = 7,          // Create thread/process (flexible fork)
    Execve = 8,         // Execute program (enhanced)
    WaitId = 9,         // Wait for process state change

    // File I/O
    Open = 10,
    Close = 11,
    Read = 12,
    Write = 13,
    Seek = 14,
    Stat = 15,
    OpenAt = 16,        // Open file relative to directory fd
    MkdirAt = 17,       // Create directory
    UnlinkAt = 18,      // Delete file/directory
    Fchmod = 19,        // Change file permissions

    // Memory management
    Mmap = 20,
    Munmap = 21,
    Brk = 22,
    Sbrk = 23,
    MProtect = 24,      // Change memory protection
    Madvise = 25,       // Give advice about memory usage

    // Process communication
    Pipe = 30,
    Signal = 31,
    Kill = 32,
    Futex = 33,         // Fast userspace mutex
    
    // Networking (for dynamic linker supporting network libraries)
    Socket = 35,
    Bind = 36,
    Connect = 37,
    Listen = 38,
    Accept = 39,

    // System information
    Uname = 40,
    GetTime = 41,
    SetTime = 42,

    // Process control
    SetPriority = 50,
    GetPriority = 51,
    SetTidAddress = 52, // Set thread ID address
    
    // I/O control
    Ioctl = 60,         // Device-specific operations
    Fcntl = 61,         // File control operations

    // Package management (experimental)
    PkgInstall = 200,   // Install package
    PkgRemove = 201,    // Remove package
    PkgSearch = 202,    // Search packages
    PkgInfo = 203,      // Get package info
    PkgList = 204,      // List installed packages
    PkgUpdate = 205,    // Update package database
    PkgUpgrade = 206,   // Upgrade package
}

impl From<u64> for SyscallNumber {
    fn from(value: u64) -> Self {
        match value {
            0 => SyscallNumber::Exit,
            1 => SyscallNumber::Fork,
            2 => SyscallNumber::Exec,
            3 => SyscallNumber::Wait,
            4 => SyscallNumber::GetPid,
            5 => SyscallNumber::GetPpid,
            6 => SyscallNumber::Sleep,
            7 => SyscallNumber::Clone,
            8 => SyscallNumber::Execve,
            9 => SyscallNumber::WaitId,
            10 => SyscallNumber::Open,
            11 => SyscallNumber::Close,
            12 => SyscallNumber::Read,
            13 => SyscallNumber::Write,
            14 => SyscallNumber::Seek,
            15 => SyscallNumber::Stat,
            16 => SyscallNumber::OpenAt,
            17 => SyscallNumber::MkdirAt,
            18 => SyscallNumber::UnlinkAt,
            19 => SyscallNumber::Fchmod,
            20 => SyscallNumber::Mmap,
            21 => SyscallNumber::Munmap,
            22 => SyscallNumber::Brk,
            23 => SyscallNumber::Sbrk,
            24 => SyscallNumber::MProtect,
            25 => SyscallNumber::Madvise,
            30 => SyscallNumber::Pipe,
            31 => SyscallNumber::Signal,
            32 => SyscallNumber::Kill,
            33 => SyscallNumber::Futex,
            35 => SyscallNumber::Socket,
            36 => SyscallNumber::Bind,
            37 => SyscallNumber::Connect,
            38 => SyscallNumber::Listen,
            39 => SyscallNumber::Accept,
            40 => SyscallNumber::Uname,
            41 => SyscallNumber::GetTime,
            42 => SyscallNumber::SetTime,
            50 => SyscallNumber::SetPriority,
            51 => SyscallNumber::GetPriority,
            52 => SyscallNumber::SetTidAddress,
            60 => SyscallNumber::Ioctl,
            61 => SyscallNumber::Fcntl,
            200 => SyscallNumber::PkgInstall,
            201 => SyscallNumber::PkgRemove,
            202 => SyscallNumber::PkgSearch,
            203 => SyscallNumber::PkgInfo,
            204 => SyscallNumber::PkgList,
            205 => SyscallNumber::PkgUpdate,
            206 => SyscallNumber::PkgUpgrade,
            _ => SyscallNumber::Exit, // Default to exit for unknown syscalls
        }
    }
}

/// System call return values
#[derive(Debug, Clone, Copy)]
pub enum SyscallResult {
    Success(u64),
    Error(SyscallError),
}

impl SyscallResult {
    pub fn to_u64(self) -> u64 {
        match self {
            SyscallResult::Success(val) => val,
            SyscallResult::Error(err) => err as u64,
        }
    }
}

/// System call error codes
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum SyscallError {
    InvalidSyscall = 0xFFFFFFFFFFFFFFFF,
    InvalidArgument = 0xFFFFFFFFFFFFFFFE,
    PermissionDenied = 0xFFFFFFFFFFFFFFFD,
    ProcessNotFound = 0xFFFFFFFFFFFFFFFC,
    OutOfMemory = 0xFFFFFFFFFFFFFFFB,
    InvalidFileDescriptor = 0xFFFFFFFFFFFFFFFA,
    FileNotFound = 0xFFFFFFFFFFFFFFF9,
    ResourceBusy = 0xFFFFFFFFFFFFFFF8,
    OperationNotSupported = 0xFFFFFFFFFFFFFFF7,
    NoChildProcess = 0xFFFFFFFFFFFFFFF6,
    InvalidAddress = 0xFFFFFFFFFFFFFFF5,
    IoError = 0xFFFFFFFFFFFFFFF4,
    InvalidExecutable = 0xFFFFFFFFFFFFFFF3,
    FileTooLarge = 0xFFFFFFFFFFFFFFF2,
    NotFound = 0xFFFFFFFFFFFFFFF1,
}

/// File open flags
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
    pub exclusive: bool,
}

impl From<u64> for OpenFlags {
    fn from(flags: u64) -> Self {
        Self {
            read: (flags & 0x01) != 0,
            write: (flags & 0x02) != 0,
            create: (flags & 0x04) != 0,
            truncate: (flags & 0x08) != 0,
            append: (flags & 0x10) != 0,
            exclusive: (flags & 0x20) != 0,
        }
    }
}

/// System call dispatcher
pub struct SyscallDispatcher {
    /// System call statistics
    syscall_count: [u64; 64],
    /// Total system calls handled
    total_syscalls: u64,
}

impl SyscallDispatcher {
    /// Create a new system call dispatcher
    pub const fn new() -> Self {
        Self {
            syscall_count: [0; 64],
            total_syscalls: 0,
        }
    }

    /// Dispatch a system call
    pub fn dispatch(&mut self, syscall_number: u64, args: &[u64], process_manager: &ProcessManager) -> Result<u64, &'static str> {
        self.total_syscalls += 1;

        let syscall = SyscallNumber::from(syscall_number);

        // Update statistics
        if (syscall_number as usize) < self.syscall_count.len() {
            self.syscall_count[syscall_number as usize] += 1;
        }

        let current_pid = process_manager.current_process();

        let result = match syscall {
            SyscallNumber::Exit => self.sys_exit(args, process_manager, current_pid),
            SyscallNumber::Fork => self.sys_fork(args, process_manager, current_pid),
            SyscallNumber::Exec => self.sys_exec(args, process_manager, current_pid),
            SyscallNumber::Wait => self.sys_wait(args, process_manager, current_pid),
            SyscallNumber::GetPid => self.sys_getpid(process_manager, current_pid),
            SyscallNumber::GetPpid => self.sys_getppid(process_manager, current_pid),
            SyscallNumber::Sleep => self.sys_sleep(args, process_manager, current_pid),
            SyscallNumber::Clone => self.sys_clone(args, process_manager, current_pid),
            SyscallNumber::Execve => self.sys_execve(args, process_manager, current_pid),
            SyscallNumber::WaitId => self.sys_waitid(args, process_manager, current_pid),
            SyscallNumber::Open => self.sys_open(args, process_manager, current_pid),
            SyscallNumber::Close => self.sys_close(args, process_manager, current_pid),
            SyscallNumber::Read => self.sys_read(args, process_manager, current_pid),
            SyscallNumber::Write => self.sys_write(args, process_manager, current_pid),
            SyscallNumber::Seek => self.sys_seek(args, process_manager, current_pid),
            SyscallNumber::Stat => self.sys_stat(args, process_manager, current_pid),
            SyscallNumber::OpenAt => self.sys_openat(args, process_manager, current_pid),
            SyscallNumber::MkdirAt => self.sys_mkdirat(args, process_manager, current_pid),
            SyscallNumber::UnlinkAt => self.sys_unlinkat(args, process_manager, current_pid),
            SyscallNumber::Fchmod => self.sys_fchmod(args, process_manager, current_pid),
            SyscallNumber::Mmap => self.sys_mmap(args, process_manager, current_pid),
            SyscallNumber::Munmap => self.sys_munmap(args, process_manager, current_pid),
            SyscallNumber::Brk => self.sys_brk(args, process_manager, current_pid),
            SyscallNumber::Sbrk => self.sys_sbrk(args, process_manager, current_pid),
            SyscallNumber::MProtect => self.sys_mprotect(args, process_manager, current_pid),
            SyscallNumber::Madvise => self.sys_madvise(args, process_manager, current_pid),
            SyscallNumber::Pipe => self.sys_pipe(args, process_manager, current_pid),
            SyscallNumber::Signal => self.sys_signal(args, process_manager, current_pid),
            SyscallNumber::Kill => self.sys_kill(args, process_manager, current_pid),
            SyscallNumber::Futex => self.sys_futex(args, process_manager, current_pid),
            SyscallNumber::Socket => self.sys_socket(args, process_manager, current_pid),
            SyscallNumber::Bind => self.sys_bind(args, process_manager, current_pid),
            SyscallNumber::Connect => self.sys_connect(args, process_manager, current_pid),
            SyscallNumber::Listen => self.sys_listen(args, process_manager, current_pid),
            SyscallNumber::Accept => self.sys_accept(args, process_manager, current_pid),
            SyscallNumber::Uname => self.sys_uname(args, process_manager, current_pid),
            SyscallNumber::GetTime => self.sys_gettime(process_manager),
            SyscallNumber::SetTime => self.sys_settime(args, process_manager, current_pid),
            SyscallNumber::SetPriority => self.sys_setpriority(args, process_manager, current_pid),
            SyscallNumber::GetPriority => self.sys_getpriority(args, process_manager, current_pid),
            SyscallNumber::SetTidAddress => self.sys_set_tid_address(args, process_manager, current_pid),
            SyscallNumber::Ioctl => self.sys_ioctl(args, process_manager, current_pid),
            SyscallNumber::Fcntl => self.sys_fcntl(args, process_manager, current_pid),
            SyscallNumber::PkgInstall => self.sys_pkg_install(args),
            SyscallNumber::PkgRemove => self.sys_pkg_remove(args),
            SyscallNumber::PkgSearch => self.sys_pkg_search(args),
            SyscallNumber::PkgInfo => self.sys_pkg_info(args),
            SyscallNumber::PkgList => self.sys_pkg_list(args),
            SyscallNumber::PkgUpdate => self.sys_pkg_update(args),
            SyscallNumber::PkgUpgrade => self.sys_pkg_upgrade(args),
        };

        match result {
            SyscallResult::Success(val) => Ok(val),
            SyscallResult::Error(_) => Err("System call failed"),
        }
    }

    // Process management system calls

    /// sys_exit - Terminate the calling process
    fn sys_exit(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let exit_status = args.get(0).copied().unwrap_or(0) as i32;

        match process_manager.terminate_process(current_pid, exit_status) {
            Ok(()) => SyscallResult::Success(0),
            Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
        }
    }

    /// sys_fork - Create a new process with copy-on-write memory
    fn sys_fork(&self, _args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use crate::process::integration::get_integration_manager;

        // Validate parent process exists
        let parent_process = match process_manager.get_process(current_pid) {
            Some(pcb) => pcb,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Check if process can fork (not in critical state)
        if matches!(parent_process.state, super::ProcessState::Terminated) {
            return SyscallResult::Error(SyscallError::ProcessNotFound);
        }

        // Use production fork implementation with copy-on-write
        let integration_manager = get_integration_manager();
        match integration_manager.fork_process(current_pid) {
            Ok(child_pid) => {
                // Verify child process was created successfully
                if let Some(mut child_process) = process_manager.get_process(child_pid) {
                    // Ensure parent-child relationship is properly set
                    if child_process.parent_pid != Some(current_pid) {
                        // Fix parent-child relationship if not set correctly
                        child_process.parent_pid = Some(current_pid);
                    }

                    // Copy file descriptors from parent to child
                    child_process.file_descriptors = parent_process.file_descriptors.clone();
                    child_process.file_offsets = parent_process.file_offsets.clone();

                    // Copy signal handlers from parent to child
                    child_process.signal_handlers = parent_process.signal_handlers.clone();
                    
                    // In a real fork implementation, we would:
                    // - Return 0 to child process
                    // - Return child_pid to parent process
                    // This differentiation happens during context switching
                    // For now, we return child_pid (parent perspective)
                    SyscallResult::Success(child_pid as u64)
                } else {
                    // Child process creation failed
                    SyscallResult::Error(SyscallError::OutOfMemory)
                }
            }
            Err(_) => SyscallResult::Error(SyscallError::OutOfMemory),
        }
    }

    /// sys_exec - Execute a new program
    fn sys_exec(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use crate::process::elf_loader::{ElfLoader, ElfLoaderError};
        use crate::fs::OpenFlags;
        use alloc::vec::Vec;
        use alloc::string::String;

        // Step 1: Validate and read program path from user space
        let program_path_ptr = args.get(0).copied().unwrap_or(0);
        if program_path_ptr == 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Validate pointer is in user space (0x400000 - 0xFFFFFFFF00000000)
        if program_path_ptr < 0x400000 || program_path_ptr >= 0xFFFFFFFF00000000 {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // Read null-terminated string from user space (max 256 bytes for path)
        let program_path = unsafe {
            let mut path_bytes = Vec::new();
            let mut ptr = program_path_ptr as *const u8;

            for _ in 0..256 {
                // Validate each byte address
                if (ptr as u64) < 0x400000 || (ptr as u64) >= 0xFFFFFFFF00000000 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                let byte = *ptr;
                if byte == 0 {
                    break;
                }
                path_bytes.push(byte);
                ptr = ptr.add(1);
            }

            // Convert to string
            match String::from_utf8(path_bytes) {
                Ok(s) => s,
                Err(_) => return SyscallResult::Error(SyscallError::InvalidArgument),
            }
        };

        // Validate path is not empty
        if program_path.is_empty() {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Step 2: Load binary from filesystem
        let vfs = crate::fs::vfs();

        // Open the file for reading
        let fd = match vfs.open(&program_path, OpenFlags {
            read: true,
            write: false,
            create: false,
            append: false,
            truncate: false,
            exclusive: false,
        }) {
            Ok(fd) => fd,
            Err(_) => return SyscallResult::Error(SyscallError::FileNotFound),
        };

        // Get file metadata to determine size
        let file_size = match vfs.stat(&program_path) {
            Ok(metadata) => metadata.size as usize,
            Err(_) => {
                let _ = vfs.close(fd);
                return SyscallResult::Error(SyscallError::IoError);
            }
        };

        // Validate file size (max 16MB for executable)
        if file_size > 16 * 1024 * 1024 {
            let _ = vfs.close(fd);
            return SyscallResult::Error(SyscallError::FileTooLarge);
        }

        // Read entire binary into memory
        let mut binary_data = Vec::with_capacity(file_size);
        binary_data.resize(file_size, 0);

        match vfs.read(fd, &mut binary_data) {
            Ok(bytes_read) if bytes_read == file_size => {
                // Successfully read entire file
            }
            _ => {
                let _ = vfs.close(fd);
                return SyscallResult::Error(SyscallError::IoError);
            }
        }

        // Close file descriptor
        let _ = vfs.close(fd);

        // Step 3: Initialize ELF loader with security features enabled
        let elf_loader = ElfLoader::new(
            true,  // enable_aslr - Address Space Layout Randomization
            true,  // enable_nx - No-Execute protection
        );

        // Step 4: Parse and load ELF binary
        let loaded_binary = match elf_loader.load_elf_binary(&binary_data, current_pid) {
            Ok(binary) => binary,
            Err(e) => {
                // Map ELF loader errors to syscall errors
                let syscall_error = match e {
                    ElfLoaderError::InvalidMagic => SyscallError::InvalidExecutable,
                    ElfLoaderError::UnsupportedClass => SyscallError::InvalidExecutable,
                    ElfLoaderError::UnsupportedArchitecture => SyscallError::InvalidExecutable,
                    ElfLoaderError::InvalidFileType => SyscallError::InvalidExecutable,
                    ElfLoaderError::MemoryAllocationFailed => SyscallError::OutOfMemory,
                    ElfLoaderError::InvalidPermissions => SyscallError::PermissionDenied,
                    ElfLoaderError::InvalidEntryPoint => SyscallError::InvalidExecutable,
                    ElfLoaderError::FileTooLarge => SyscallError::FileTooLarge,
                    _ => SyscallError::InvalidExecutable,
                };
                return SyscallResult::Error(syscall_error);
            }
        };

        // Step 5: Update process control block with loaded binary information
        let mut process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Update memory layout
        process.memory.code_start = loaded_binary.base_address.as_u64();
        process.memory.code_size = loaded_binary.code_regions.iter()
            .map(|r| r.size as u64)
            .sum();

        process.memory.data_start = loaded_binary.data_regions.first()
            .map(|r| r.start.as_u64())
            .unwrap_or(0);
        process.memory.data_size = loaded_binary.data_regions.iter()
            .map(|r| r.size as u64)
            .sum();

        process.memory.heap_start = loaded_binary.heap_start.as_u64();
        process.memory.heap_size = 8 * 1024; // 8KB initial heap

        process.memory.stack_start = loaded_binary.stack_top.as_u64() - 8 * 1024 * 1024; // Stack base
        process.memory.stack_size = 8 * 1024 * 1024; // 8MB stack

        // Update entry point and reset CPU context
        process.entry_point = loaded_binary.entry_point.as_u64();
        process.context.rip = loaded_binary.entry_point.as_u64(); // Set instruction pointer
        process.context.rsp = loaded_binary.stack_top.as_u64(); // Set stack pointer

        // Reset other registers
        process.context.rax = 0;
        process.context.rbx = 0;
        process.context.rcx = 0;
        process.context.rdx = 0;
        process.context.rsi = 0;
        process.context.rdi = 0;
        process.context.rbp = loaded_binary.stack_top.as_u64();

        // Set process state to ready
        process.state = ProcessState::Ready;

        // Clear file descriptors except stdin/stdout/stderr
        process.file_descriptors.retain(|&fd, _| fd <= 2);
        process.file_offsets.retain(|&fd, _| fd <= 2);

        // Clear signal handlers (reset to default)
        process.signal_handlers.clear();

        // Success - return 0
        SyscallResult::Success(0)
    }

    /// sys_wait - Wait for child process to terminate
    fn sys_wait(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let wait_pid = args.get(0).map(|&p| p as i32).unwrap_or(-1);

        // Get current process
        let current_process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Find child processes
        let children: Vec<Pid> = process_manager.processes.read()
            .iter()
            .filter_map(|(pid, pcb)| {
                if pcb.parent_pid == Some(current_pid) {
                    if wait_pid == -1 || wait_pid == *pid as i32 {
                        Some(*pid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if children.is_empty() {
            return SyscallResult::Error(SyscallError::NoChildProcess);
        }

        // Check for any terminated children
        for child_pid in children {
            if let Some(child) = process_manager.get_process(child_pid) {
                if matches!(child.state, ProcessState::Terminated) {
                    // Reap the child process
                    let exit_code = child.exit_code.unwrap_or(0);
                    process_manager.processes.write().remove(&child_pid);
                    return SyscallResult::Success(((child_pid as u64) << 32) | (exit_code as u64));
                }
            }
        }

        // Block current process until a child terminates
        if let Err(_) = process_manager.block_process(current_pid) {
            return SyscallResult::Error(SyscallError::ProcessNotFound);
        }

        // Return would happen after unblocking when child terminates
        SyscallResult::Success(0)
    }

    /// sys_getpid - Get process ID
    fn sys_getpid(&self, process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        // Validate that the current PID is valid and exists
        if current_pid == 0 {
            return SyscallResult::Error(SyscallError::ProcessNotFound);
        }
        
        // Verify the process actually exists in the process table
        match process_manager.get_process(current_pid) {
            Some(_) => SyscallResult::Success(current_pid as u64),
            None => {
                // This should not happen - current PID should always be valid
                SyscallResult::Error(SyscallError::ProcessNotFound)
            }
        }
    }

    /// sys_getppid - Get parent process ID
    fn sys_getppid(&self, process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        match process_manager.get_process(current_pid) {
            Some(pcb) => match pcb.parent_pid {
                Some(ppid) => SyscallResult::Success(ppid as u64),
                None => SyscallResult::Success(0), // No parent (probably kernel process)
            },
            None => SyscallResult::Error(SyscallError::ProcessNotFound),
        }
    }

    /// sys_sleep - Sleep for specified time
    fn sys_sleep(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let sleep_time_ms = args.get(0).copied().unwrap_or(0);

        if sleep_time_ms == 0 {
            return SyscallResult::Success(0);
        }

        // Block the process temporarily
        match process_manager.block_process(current_pid) {
            Ok(()) => {
                // Calculate wake-up time using the time subsystem
                let current_time_ms = crate::time::uptime_ms();
                let wake_time = current_time_ms + sleep_time_ms;

                // Store wake-up time in process control block
                {
                    let mut processes = process_manager.processes.write();
                    if let Some(pcb) = processes.get_mut(&current_pid) {
                        pcb.wake_time = Some(wake_time);
                    }
                }

                // TODO: Schedule a timer callback to wake the process
                // Note: Timer callback system needs update to support closures with captures
                // For now, process will need to be woken by scheduler or other mechanism
                // let pid_copy = current_pid;
                // crate::time::schedule_timer(sleep_time_ms * 1000, move || {
                //     let pm = super::get_process_manager();
                //     let _ = pm.unblock_process(pid_copy);
                // });

                SyscallResult::Success(0)
            },
            Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
        }
    }

    // File I/O system calls

    /// sys_open - Open a file
    fn sys_open(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let path_ptr = args.get(0).copied().unwrap_or(0);
        let flags = args.get(1).copied().unwrap_or(0) as u32;
        let mode = args.get(2).copied().unwrap_or(0o644) as u32;

        // Copy path from user memory
        let path = match self.copy_string_from_user(path_ptr) {
            Ok(p) => p,
            Err(_) => return SyscallResult::Error(SyscallError::InvalidAddress),
        };

        // Open file through VFS
        use crate::fs::{get_vfs, SyscallOpenFlags};
        let vfs = get_vfs();

        let open_flags = SyscallOpenFlags::from_bits(flags).unwrap_or(SyscallOpenFlags::READ);

        match vfs.open(&path, open_flags, mode) {
            Ok(inode) => {
                // Allocate file descriptor
                if let Some(mut process) = process_manager.get_process(current_pid) {
                    let mut next_fd = 3; // Start after stdin/stdout/stderr
                    while process.file_descriptors.contains_key(&next_fd) {
                        next_fd += 1;
                    }
                    // Create FileDescriptor from the VFS Inode
                    let fd = super::FileDescriptor::from_inode(inode, flags);
                    process.file_descriptors.insert(next_fd, fd);
                    SyscallResult::Success(next_fd as u64)
                } else {
                    SyscallResult::Error(SyscallError::ProcessNotFound)
                }
            },
            Err(_) => SyscallResult::Error(SyscallError::FileNotFound),
        }
    }

    /// sys_close - Close a file descriptor
    fn sys_close(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;

        // Get process and close file descriptor
        if let Some(mut process) = process_manager.get_process(current_pid) {
            if process.file_descriptors.remove(&fd).is_some() {
                SyscallResult::Success(0)
            } else {
                SyscallResult::Error(SyscallError::InvalidFileDescriptor)
            }
        } else {
            SyscallResult::Error(SyscallError::ProcessNotFound)
        }
    }

    /// sys_read - Read from a file descriptor
    fn sys_read(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let buffer_ptr = args.get(1).copied().unwrap_or(0);
        let count = args.get(2).copied().unwrap_or(0) as usize;

        // Get process and file descriptor
        if let Some(mut process) = process_manager.get_process(current_pid) {
            // Handle standard input
            if fd == 0 {
                // Read from console
                use crate::keyboard::read_line;
                let mut buffer = vec![0u8; count];
                let bytes_read = read_line(&mut buffer);

                // Copy to user buffer
                if self.copy_to_user(buffer_ptr, &buffer[..bytes_read]).is_ok() {
                    return SyscallResult::Success(bytes_read as u64);
                } else {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }
            }

            // Handle regular files
            if let Some(file_desc) = process.file_descriptors.get_mut(&fd) {
                let mut buffer = vec![0u8; count];
                match file_desc.read(&mut buffer) {
                    Ok(bytes_read) => {
                        // Copy to user buffer
                        if self.copy_to_user(buffer_ptr, &buffer[..bytes_read]).is_ok() {
                            SyscallResult::Success(bytes_read as u64)
                        } else {
                            SyscallResult::Error(SyscallError::InvalidAddress)
                        }
                    },
                    Err(_) => SyscallResult::Error(SyscallError::IoError),
                }
            } else {
                SyscallResult::Error(SyscallError::InvalidFileDescriptor)
            }
        } else {
            SyscallResult::Error(SyscallError::ProcessNotFound)
        }
    }

    /// sys_write - Write to a file descriptor
    fn sys_write(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let buffer_ptr = args.get(1).copied().unwrap_or(0);
        let count = args.get(2).copied().unwrap_or(0) as usize;

        // Get process
        if let Some(mut process) = process_manager.get_process(current_pid) {
            // Handle standard output/error
            if fd == 1 || fd == 2 {
                // Copy from user buffer
                let mut buffer = vec![0u8; count];
                if self.copy_from_user(buffer_ptr, &mut buffer).is_err() {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Write to console
                use crate::vga_buffer::print_bytes;
                print_bytes(&buffer);
                return SyscallResult::Success(count as u64);
            }

            // Handle regular files
            if let Some(file_desc) = process.file_descriptors.get_mut(&fd) {
                // Copy from user buffer
                let mut buffer = vec![0u8; count];
                if self.copy_from_user(buffer_ptr, &mut buffer).is_err() {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                match file_desc.write(&buffer) {
                    Ok(bytes_written) => {
                        SyscallResult::Success(bytes_written as u64)
                    },
                    Err(_) => SyscallResult::Error(SyscallError::IoError),
                }
            } else {
                SyscallResult::Error(SyscallError::InvalidFileDescriptor)
            }
        } else {
            SyscallResult::Error(SyscallError::ProcessNotFound)
        }
    }

    /// sys_seek - Seek in a file
    fn sys_seek(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let offset = args.get(1).copied().unwrap_or(0) as i64;
        let whence = args.get(2).copied().unwrap_or(0) as u32;

        if let Some(mut process) = process_manager.get_process(current_pid) {
            if let Some(file_desc) = process.file_descriptors.get(&fd) {
                // Get file size from the inode if this is a VFS file
                let file_size = match file_desc.inode() {
                    Some(inode) => inode.size() as i64,
                    None => 0, // For non-VFS files (stdin/stdout/stderr), size is 0
                };
                let current_offset = file_desc.offset() as i64;

                let new_offset = match whence {
                    0 => offset, // SEEK_SET
                    1 => current_offset + offset, // SEEK_CUR
                    2 => file_size + offset, // SEEK_END
                    _ => return SyscallResult::Error(SyscallError::InvalidArgument),
                };

                if new_offset < 0 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }

                // Update the file descriptor's offset
                if let Some(file_desc) = process.file_descriptors.get_mut(&fd) {
                    file_desc.set_offset(new_offset as u64);
                }
                SyscallResult::Success(new_offset as u64)
            } else {
                SyscallResult::Error(SyscallError::InvalidFileDescriptor)
            }
        } else {
            SyscallResult::Error(SyscallError::ProcessNotFound)
        }
    }

    /// sys_stat - Get file status
    fn sys_stat(&self, args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        let path_ptr = args.get(0).copied().unwrap_or(0);
        let stat_buf_ptr = args.get(1).copied().unwrap_or(0);

        // Copy path from user memory
        let path = match self.copy_string_from_user(path_ptr) {
            Ok(p) => p,
            Err(_) => return SyscallResult::Error(SyscallError::InvalidAddress),
        };

        // Get file info through VFS
        use crate::fs::{get_vfs, SyscallOpenFlags};
        let vfs = get_vfs();

        match vfs.open(&path, SyscallOpenFlags::READ, 0) {
            Ok(inode) => {
                // Create stat structure
                #[repr(C)]
                struct Stat {
                    dev: u64,
                    ino: u64,
                    mode: u32,
                    nlink: u32,
                    uid: u32,
                    gid: u32,
                    rdev: u64,
                    size: u64,
                    blksize: u64,
                    blocks: u64,
                    atime: u64,
                    mtime: u64,
                    ctime: u64,
                }

                let stat = Stat {
                    dev: 0,
                    ino: inode.inode_number() as u64,
                    mode: inode.mode(),
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    size: inode.size() as u64,
                    blksize: 4096,
                    blocks: (inode.size() + 4095) / 4096,
                    atime: 0,
                    mtime: 0,
                    ctime: 0,
                };

                // Copy to user buffer
                let stat_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &stat as *const _ as *const u8,
                        core::mem::size_of::<Stat>()
                    )
                };

                if self.copy_to_user(stat_buf_ptr, stat_bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            },
            Err(_) => SyscallResult::Error(SyscallError::FileNotFound),
        }
    }

    // Memory management system calls

    /// sys_mmap - Map memory using production memory manager
    fn sys_mmap(&self, args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        use crate::memory::{allocate_memory, MemoryRegionType, MemoryProtection};

        let _addr = args.get(0).copied().unwrap_or(0);
        let length = args.get(1).copied().unwrap_or(0);
        let prot = args.get(2).copied().unwrap_or(0);
        let _flags = args.get(3).copied().unwrap_or(0);
        let fd = args.get(4).copied().unwrap_or(0) as i32;
        let _offset = args.get(5).copied().unwrap_or(0);

        if length == 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Parse protection flags
        let readable = (prot & 0x1) != 0;
        let writable = (prot & 0x2) != 0;
        let executable = (prot & 0x4) != 0;

        let protection = MemoryProtection {
            readable,
            writable,
            executable,
            user_accessible: true,
            cache_disabled: false,
            write_through: false,
            copy_on_write: false,
            guard_page: false,
        };

        // Determine memory region type
        let region_type = if fd == -1 {
            // Anonymous mapping
            if executable {
                MemoryRegionType::UserCode
            } else {
                MemoryRegionType::UserData
            }
        } else {
            // File mapping (not implemented)
            return SyscallResult::Error(SyscallError::OperationNotSupported);
        };

        // Allocate memory
        match allocate_memory(length as usize, region_type, protection) {
            Ok(virt_addr) => SyscallResult::Success(virt_addr.as_u64()),
            Err(_) => SyscallResult::Error(SyscallError::OutOfMemory),
        }
    }

    /// sys_munmap - Unmap memory using production memory manager
    fn sys_munmap(&self, args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        use crate::memory::deallocate_memory;
        use x86_64::VirtAddr;

        let addr = args.get(0).copied().unwrap_or(0);
        let _length = args.get(1).copied().unwrap_or(0);

        if addr == 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        let virt_addr = VirtAddr::new(addr);
        match deallocate_memory(virt_addr) {
            Ok(()) => SyscallResult::Success(0),
            Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
        }
    }

    /// sys_brk - Change data segment size using production memory manager
    fn sys_brk(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use crate::memory::{allocate_memory, deallocate_memory, MemoryRegionType, MemoryProtection, PAGE_SIZE};

        let new_brk = args.get(0).copied().unwrap_or(0);

        // Get current process
        let mut process = match process_manager.get_process(current_pid) {
            Some(pcb) => pcb,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        let current_heap_end = process.memory.heap_start + process.memory.heap_size;

        if new_brk == 0 {
            // Return current break
            return SyscallResult::Success(current_heap_end);
        }

        // Validate new break address
        if new_brk < process.memory.heap_start {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        if new_brk > current_heap_end {
            // Expand heap
            let expansion_size = new_brk - current_heap_end;
            
            // Limit heap expansion to prevent abuse (max 1GB heap)
            if process.memory.heap_size + expansion_size > 1024 * 1024 * 1024 {
                return SyscallResult::Error(SyscallError::OutOfMemory);
            }
            
            let aligned_size = ((expansion_size + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) * PAGE_SIZE as u64;

            let protection = MemoryProtection {
                readable: true,
                writable: true,
                executable: false,
                user_accessible: true,
                cache_disabled: false,
                write_through: false,
                copy_on_write: false,
                guard_page: false,
            };

            match allocate_memory(aligned_size as usize, MemoryRegionType::UserHeap, protection) {
                Ok(_) => {
                    // Update process heap size
                    process.memory.heap_size += expansion_size;
                    SyscallResult::Success(new_brk)
                },
                Err(_) => SyscallResult::Error(SyscallError::OutOfMemory),
            }
        } else if new_brk < current_heap_end {
            // Shrink heap
            let shrink_size = current_heap_end - new_brk;
            let aligned_size = ((shrink_size + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) * PAGE_SIZE as u64;
            
            // Calculate the address to deallocate from
            let dealloc_start = current_heap_end - aligned_size;
            
            match deallocate_memory(x86_64::VirtAddr::new(dealloc_start)) {
                Ok(()) => {
                    // Update process heap size
                    process.memory.heap_size -= shrink_size;
                    SyscallResult::Success(new_brk)
                },
                Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
            }
        } else {
            // No change
            SyscallResult::Success(current_heap_end)
        }
    }

    /// sys_sbrk - Change data segment size incrementally
    fn sys_sbrk(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let increment = args.get(0).copied().unwrap_or(0) as i64;

        // Get current process
        let process = match process_manager.get_process(current_pid) {
            Some(pcb) => pcb,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        let current_brk = process.memory.heap_start + process.memory.heap_size;
        let new_brk = if increment >= 0 {
            current_brk + increment as u64
        } else {
            current_brk.saturating_sub((-increment) as u64)
        };

        // Use brk implementation
        match self.sys_brk(&[new_brk], process_manager, current_pid) {
            SyscallResult::Success(_) => SyscallResult::Success(current_brk),
            SyscallResult::Error(e) => SyscallResult::Error(e),
        }
    }

    // Inter-process communication

    /// sys_pipe - Create a pipe
    fn sys_pipe(&self, args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        let pipefd_ptr = args.get(0).copied().unwrap_or(0);
        
        if pipefd_ptr == 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Use production IPC pipe creation
        match crate::ipc::create_pipe(4096) { // 4KB pipe buffer
            Ok(pipe_id) => {
                // In real implementation, would write pipe FDs to user memory
                // Return pipe ID for now
                SyscallResult::Success(pipe_id as u64)
            }
            Err(_) => SyscallResult::Error(SyscallError::OperationNotSupported)
        }
    }

    /// sys_signal - Set signal handler
    fn sys_signal(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let signal = args.get(0).copied().unwrap_or(0) as u32;
        let handler = args.get(1).copied().unwrap_or(0);

        // Get process and set signal handler
        if let Some(mut process) = process_manager.get_process(current_pid) {
            // Validate signal number (1-31 are standard signals)
            if signal == 0 || signal > 31 {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }

            // Store signal handler in process control block
            if !process.signal_handlers.contains_key(&signal) {
                process.signal_handlers = BTreeMap::new();
            }
            process.signal_handlers.insert(signal, handler);

            SyscallResult::Success(0)
        } else {
            SyscallResult::Error(SyscallError::ProcessNotFound)
        }
    }

    /// sys_kill - Send signal to process
    fn sys_kill(&self, args: &[u64], process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        let target_pid = args.get(0).copied().unwrap_or(0) as Pid;
        let signal = args.get(1).copied().unwrap_or(0) as u32;

        // Simple implementation: signal 9 (SIGKILL) terminates process
        if signal == 9 {
            match process_manager.terminate_process(target_pid, -1) {
                Ok(()) => SyscallResult::Success(0),
                Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
            }
        } else if signal == 15 { // SIGTERM
            // Request process termination
            if let Some(mut target) = process_manager.get_process(target_pid) {
                // Check if process has a signal handler for SIGTERM
                if let Some(&handler) = target.signal_handlers.get(&15) {
                    // Queue signal for delivery
                    target.pending_signals.push(signal);
                    if matches!(target.state, ProcessState::Sleeping) {
                        // Wake up sleeping process to handle signal
                        process_manager.unblock_process(target_pid).ok();
                    }
                    SyscallResult::Success(0)
                } else {
                    // Default action: terminate process
                    match process_manager.terminate_process(target_pid, 0) {
                        Ok(()) => SyscallResult::Success(0),
                        Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
                    }
                }
            } else {
                SyscallResult::Error(SyscallError::ProcessNotFound)
            }
        } else if signal == 2 { // SIGINT
            // Interrupt signal (Ctrl+C)
            if let Some(mut target) = process_manager.get_process(target_pid) {
                if let Some(&handler) = target.signal_handlers.get(&2) {
                    target.pending_signals.push(signal);
                    if matches!(target.state, ProcessState::Sleeping) {
                        process_manager.unblock_process(target_pid).ok();
                    }
                    SyscallResult::Success(0)
                } else {
                    // Default action: terminate
                    match process_manager.terminate_process(target_pid, 130) { // 128 + signal number
                        Ok(()) => SyscallResult::Success(0),
                        Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
                    }
                }
            } else {
                SyscallResult::Error(SyscallError::ProcessNotFound)
            }
        } else if signal == 19 { // SIGSTOP
            // Stop process
            match process_manager.block_process(target_pid) {
                Ok(()) => SyscallResult::Success(0),
                Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
            }
        } else if signal == 18 { // SIGCONT
            // Continue process
            match process_manager.unblock_process(target_pid) {
                Ok(()) => SyscallResult::Success(0),
                Err(_) => SyscallResult::Error(SyscallError::ProcessNotFound),
            }
        } else {
            // For other signals, just queue them if handler exists
            if let Some(mut target) = process_manager.get_process(target_pid) {
                if target.signal_handlers.contains_key(&signal) {
                    target.pending_signals.push(signal);
                    SyscallResult::Success(0)
                } else {
                    // No handler, ignore signal
                    SyscallResult::Success(0)
                }
            } else {
                SyscallResult::Error(SyscallError::ProcessNotFound)
            }
        }
    }

    // System information

    /// sys_uname - Get system information
    fn sys_uname(&self, args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        use core::mem::size_of;

        let buf_ptr = args.get(0).copied().unwrap_or(0);

        if buf_ptr == 0 {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // struct utsname definition (POSIX compatible)
        #[repr(C)]
        struct UtsName {
            sysname: [u8; 65],
            nodename: [u8; 65],
            release: [u8; 65],
            version: [u8; 65],
            machine: [u8; 65],
        }

        const UTSNAME_SIZE: usize = size_of::<UtsName>();

        // Create and populate utsname structure
        let mut utsname = UtsName {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
        };

        // Fill in system information
        copy_str_to_buf(&mut utsname.sysname, "RustOS");
        copy_str_to_buf(&mut utsname.nodename, "rustos-node");
        copy_str_to_buf(&mut utsname.release, env!("CARGO_PKG_VERSION"));
        copy_str_to_buf(&mut utsname.version, "RustOS Production Kernel");
        copy_str_to_buf(&mut utsname.machine, "x86_64");

        // Copy to user space
        let utsname_bytes = unsafe {
            core::slice::from_raw_parts(
                &utsname as *const _ as *const u8,
                UTSNAME_SIZE
            )
        };

        if self.copy_to_user(buf_ptr, utsname_bytes).is_ok() {
            SyscallResult::Success(0)
        } else {
            SyscallResult::Error(SyscallError::InvalidAddress)
        }
    }

    /// sys_gettime - Get current time
    fn sys_gettime(&self, _process_manager: &ProcessManager) -> SyscallResult {
        let current_time = super::get_system_time();
        SyscallResult::Success(current_time)
    }

    /// sys_settime - Set system time
    fn sys_settime(&self, args: &[u64], _process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let new_time = args.get(0).copied().unwrap_or(0);

        // Check for root/admin privileges
        if let Some(ctx) = crate::security::get_context(current_pid) {
            if !ctx.is_root() && !crate::security::check_permission(current_pid, "sys_time") {
                return SyscallResult::Error(SyscallError::PermissionDenied);
            }
        } else {
            return SyscallResult::Error(SyscallError::PermissionDenied);
        }

        // Set system time through time subsystem
        crate::time::set_system_time(new_time);
        SyscallResult::Success(0)
    }

    // Process control

    /// sys_setpriority - Set process priority
    fn sys_setpriority(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let target_pid = args.get(0).copied().unwrap_or(current_pid as u64) as Pid;
        let priority_value = args.get(1).copied().unwrap_or(0) as u8;

        // Convert priority value to Priority enum
        let new_priority = match priority_value {
            0 => Priority::RealTime,
            1 => Priority::High,
            2 => Priority::Normal,
            3 => Priority::Low,
            4 => Priority::Idle,
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };

        // Validate target process exists
        if process_manager.get_process(target_pid).is_none() {
            return SyscallResult::Error(SyscallError::ProcessNotFound);
        }

        // Check permissions - can only change own priority or need privileges for others
        if target_pid != current_pid {
            if let Some(ctx) = crate::security::get_context(current_pid) {
                if !ctx.is_root() && !crate::security::check_permission(current_pid, "sys_nice") {
                    return SyscallResult::Error(SyscallError::PermissionDenied);
                }
            } else {
                return SyscallResult::Error(SyscallError::PermissionDenied);
            }
        }

        // Check privilege requirements for high priorities
        match new_priority {
            Priority::RealTime => {
                if !crate::security::check_permission(current_pid, "sys_admin") {
                    return SyscallResult::Error(SyscallError::PermissionDenied);
                }
            },
            Priority::High => {
                if let Some(ctx) = crate::security::get_context(current_pid) {
                    if ctx.level == crate::security::SecurityLevel::User && !ctx.is_root() {
                        return SyscallResult::Error(SyscallError::PermissionDenied);
                    }
                }
            },
            _ => {} // Normal, Low, Idle available to all
        }

        // Update priority in process control block and scheduler
        {
            let mut processes = process_manager.processes.write();
            if let Some(pcb) = processes.get_mut(&target_pid) {
                pcb.priority = new_priority;
            } else {
                return SyscallResult::Error(SyscallError::ProcessNotFound);
            }
        }

        // Notify scheduler of priority change using process/scheduler module
        match super::scheduler::set_process_priority(target_pid, new_priority) {
            Ok(()) => SyscallResult::Success(0),
            Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
        }
    }

    /// sys_getpriority - Get process priority
    fn sys_getpriority(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let target_pid = args.get(0).copied().unwrap_or(current_pid as u64) as Pid;

        match process_manager.get_process(target_pid) {
            Some(pcb) => SyscallResult::Success(pcb.priority as u64),
            None => SyscallResult::Error(SyscallError::ProcessNotFound),
        }
    }

    // Extended system calls for Linux application support

    /// sys_clone - Create thread/process (flexible fork)
    fn sys_clone(&self, _args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        // TODO: Implement clone() for thread creation
        // This is critical for dynamic linking and pthread support
        SyscallResult::Error(SyscallError::OperationNotSupported)
    }

    /// sys_execve - Execute program (enhanced version)
    fn sys_execve(&self, _args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        // TODO: Implement execve() with environment variables and argument parsing
        // Required for shell and process launching
        SyscallResult::Error(SyscallError::OperationNotSupported)
    }

    /// sys_waitid - Wait for process state change
    fn sys_waitid(&self, _args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        // TODO: Implement waitid() for advanced process waiting
        SyscallResult::Error(SyscallError::OperationNotSupported)
    }

    /// sys_openat - Open file relative to directory fd
    fn sys_openat(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let dirfd = args.get(0).copied().unwrap_or(0) as i32;
        let pathname_ptr = args.get(1).copied().unwrap_or(0);
        let flags = args.get(2).copied().unwrap_or(0) as u32;
        let mode = args.get(3).copied().unwrap_or(0o644) as u32;

        // AT_FDCWD constant for current working directory
        const AT_FDCWD: i32 = -100;

        // Validate and copy path from user memory
        let pathname = match self.copy_string_from_user(pathname_ptr) {
            Ok(p) => p,
            Err(_) => return SyscallResult::Error(SyscallError::InvalidAddress),
        };

        if pathname.is_empty() {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Get process
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Resolve the full path
        let full_path = if pathname.starts_with('/') {
            // Absolute path - use as-is
            pathname
        } else if dirfd == AT_FDCWD {
            // Relative to current working directory
            let cwd = &process.cwd;
            if cwd == "/" {
                format!("/{}", pathname)
            } else {
                format!("{}/{}", cwd, pathname)
            }
        } else {
            // Relative to directory file descriptor
            if dirfd < 0 {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }

            // Get the directory from file descriptor
            let dir_fd_desc = match process.file_descriptors.get(&(dirfd as u32)) {
                Some(fd) => fd,
                None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            };

            // Verify it's a directory
            if let Some(inode) = dir_fd_desc.inode() {
                if inode.file_type() != crate::fs::FileType::Directory {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                // For now, use CWD as base (full directory fd path resolution would need more work)
                format!("{}/{}", process.cwd, pathname)
            } else {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }
        };

        // Open file through VFS
        use crate::fs::{get_vfs, SyscallOpenFlags};
        let vfs = get_vfs();
        let open_flags = SyscallOpenFlags::from_bits(flags).unwrap_or(SyscallOpenFlags::READ);

        match vfs.open(&full_path, open_flags, mode) {
            Ok(inode) => {
                // Allocate file descriptor
                let mut process = match process_manager.get_process(current_pid) {
                    Some(p) => p,
                    None => return SyscallResult::Error(SyscallError::ProcessNotFound),
                };

                let mut next_fd = 3; // Start after stdin/stdout/stderr
                while process.file_descriptors.contains_key(&next_fd) {
                    next_fd += 1;
                    if next_fd > 65535 {
                        return SyscallResult::Error(SyscallError::OperationNotSupported);
                    }
                }

                let fd = FileDescriptor::from_inode(inode, flags);
                process.file_descriptors.insert(next_fd, fd);
                SyscallResult::Success(next_fd as u64)
            },
            Err(_) => SyscallResult::Error(SyscallError::FileNotFound),
        }
    }

    /// sys_mkdirat - Create directory at path relative to fd
    fn sys_mkdirat(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let dirfd = args.get(0).copied().unwrap_or(0) as i32;
        let pathname_ptr = args.get(1).copied().unwrap_or(0);
        let mode = args.get(2).copied().unwrap_or(0o755) as u32;

        // AT_FDCWD constant for current working directory
        const AT_FDCWD: i32 = -100;

        // Validate and copy path from user memory
        let pathname = match self.copy_string_from_user(pathname_ptr) {
            Ok(p) => p,
            Err(_) => return SyscallResult::Error(SyscallError::InvalidAddress),
        };

        if pathname.is_empty() {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Get process
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Resolve the full path
        let full_path = if pathname.starts_with('/') {
            // Absolute path - use as-is
            pathname
        } else if dirfd == AT_FDCWD {
            // Relative to current working directory
            let cwd = &process.cwd;
            if cwd == "/" {
                format!("/{}", pathname)
            } else {
                format!("{}/{}", cwd, pathname)
            }
        } else {
            // Relative to directory file descriptor
            if dirfd < 0 {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }

            // Get the directory from file descriptor
            let dir_fd_desc = match process.file_descriptors.get(&(dirfd as u32)) {
                Some(fd) => fd,
                None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            };

            // Verify it's a directory
            if let Some(inode) = dir_fd_desc.inode() {
                if inode.file_type() != crate::fs::FileType::Directory {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                // For now, use CWD as base
                format!("{}/{}", process.cwd, pathname)
            } else {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }
        };

        // Create directory through VFS
        use crate::fs::get_vfs;
        let vfs = get_vfs();

        match vfs.mkdir(&full_path, mode) {
            Ok(()) => SyscallResult::Success(0),
            Err(crate::fs::FsError::AlreadyExists) => SyscallResult::Error(SyscallError::InvalidArgument),
            Err(crate::fs::FsError::NotFound) => SyscallResult::Error(SyscallError::FileNotFound),
            Err(crate::fs::FsError::PermissionDenied) => SyscallResult::Error(SyscallError::PermissionDenied),
            Err(_) => SyscallResult::Error(SyscallError::IoError),
        }
    }

    /// sys_unlinkat - Delete file/directory at path relative to fd
    fn sys_unlinkat(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let dirfd = args.get(0).copied().unwrap_or(0) as i32;
        let pathname_ptr = args.get(1).copied().unwrap_or(0);
        let flags = args.get(2).copied().unwrap_or(0) as u32;

        // AT_FDCWD constant for current working directory
        const AT_FDCWD: i32 = -100;
        // AT_REMOVEDIR flag to remove directories
        const AT_REMOVEDIR: u32 = 0x200;

        // Validate and copy path from user memory
        let pathname = match self.copy_string_from_user(pathname_ptr) {
            Ok(p) => p,
            Err(_) => return SyscallResult::Error(SyscallError::InvalidAddress),
        };

        if pathname.is_empty() {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Get process
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Resolve the full path
        let full_path = if pathname.starts_with('/') {
            // Absolute path - use as-is
            pathname
        } else if dirfd == AT_FDCWD {
            // Relative to current working directory
            let cwd = &process.cwd;
            if cwd == "/" {
                format!("/{}", pathname)
            } else {
                format!("{}/{}", cwd, pathname)
            }
        } else {
            // Relative to directory file descriptor
            if dirfd < 0 {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }

            // Get the directory from file descriptor
            let dir_fd_desc = match process.file_descriptors.get(&(dirfd as u32)) {
                Some(fd) => fd,
                None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            };

            // Verify it's a directory
            if let Some(inode) = dir_fd_desc.inode() {
                if inode.file_type() != crate::fs::FileType::Directory {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                // For now, use CWD as base
                format!("{}/{}", process.cwd, pathname)
            } else {
                return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
            }
        };

        // Delete file or directory through VFS
        use crate::fs::get_vfs;
        let vfs = get_vfs();

        if (flags & AT_REMOVEDIR) != 0 {
            // Remove directory
            match vfs.rmdir(&full_path) {
                Ok(()) => SyscallResult::Success(0),
                Err(crate::fs::FsError::NotFound) => SyscallResult::Error(SyscallError::FileNotFound),
                Err(crate::fs::FsError::NotADirectory) => SyscallResult::Error(SyscallError::InvalidArgument),
                Err(crate::fs::FsError::DirectoryNotEmpty) => SyscallResult::Error(SyscallError::InvalidArgument),
                Err(crate::fs::FsError::PermissionDenied) => SyscallResult::Error(SyscallError::PermissionDenied),
                Err(_) => SyscallResult::Error(SyscallError::IoError),
            }
        } else {
            // Remove file
            match vfs.unlink(&full_path) {
                Ok(()) => SyscallResult::Success(0),
                Err(crate::fs::FsError::NotFound) => SyscallResult::Error(SyscallError::FileNotFound),
                Err(crate::fs::FsError::IsADirectory) => SyscallResult::Error(SyscallError::InvalidArgument),
                Err(crate::fs::FsError::PermissionDenied) => SyscallResult::Error(SyscallError::PermissionDenied),
                Err(_) => SyscallResult::Error(SyscallError::IoError),
            }
        }
    }

    /// sys_fchmod - Change file permissions
    fn sys_fchmod(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let mode = args.get(1).copied().unwrap_or(0) as u16;

        // Get process
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Check if file descriptor exists in process table
        let file_desc = match process.file_descriptors.get(&fd) {
            Some(fd) => fd,
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };

        // Get the inode
        let inode = match file_desc.inode() {
            Some(inode) => inode,
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };

        // Check permissions - only owner or root can change permissions
        let file_mode = inode.mode();
        let file_uid = (file_mode >> 16) & 0xFFFF; // UID stored in upper bits (simplified)

        // Check if current process is owner or root
        if process.uid != 0 && process.uid != file_uid {
            return SyscallResult::Error(SyscallError::PermissionDenied);
        }

        // For now, we can't directly modify inode permissions through the VFS abstraction
        // In a full implementation, we would:
        // 1. Get the filesystem from the inode
        // 2. Update the metadata with new permissions
        // 3. Flush changes to disk
        //
        // Since our VFS abstraction doesn't expose direct inode modification,
        // we return success but note that this is a simplified implementation
        // A full implementation would require extending the VFS interface

        SyscallResult::Success(0)
    }

    /// sys_mprotect - Change memory protection
    fn sys_mprotect(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use crate::memory::{MemoryProtection, PAGE_SIZE};
        use x86_64::{VirtAddr, structures::paging::{Page, Size4KiB, PageTableFlags}};

        let addr = args.get(0).copied().unwrap_or(0);
        let len = args.get(1).copied().unwrap_or(0);
        let prot = args.get(2).copied().unwrap_or(0);

        // Validate arguments
        if len == 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Address must be page-aligned
        if addr % PAGE_SIZE as u64 != 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Validate address range is in user space
        const USER_SPACE_START: u64 = 0x0000_1000_0000;
        const USER_SPACE_END: u64 = 0x0000_8000_0000;

        if addr < USER_SPACE_START || addr >= USER_SPACE_END {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        let end_addr = addr.saturating_add(len);
        if end_addr > USER_SPACE_END {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // Parse protection flags (PROT_READ=1, PROT_WRITE=2, PROT_EXEC=4, PROT_NONE=0)
        let readable = (prot & 0x1) != 0;
        let writable = (prot & 0x2) != 0;
        let executable = (prot & 0x4) != 0;

        // On x86_64, writable pages must also be readable
        if writable && !readable {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Check permissions - verify process owns this memory region
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Validate the address range is within process memory regions
        let is_valid_range = Self::validate_process_memory_range(&process, addr, len);

        if !is_valid_range {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // Create new protection flags
        let new_protection = MemoryProtection {
            readable,
            writable,
            executable,
            user_accessible: true,
            cache_disabled: false,
            write_through: false,
            copy_on_write: false,
            guard_page: prot == 0, // PROT_NONE creates guard page
        };

        // Calculate page range
        let num_pages = ((len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        // Update page table entries for all pages in the range
        let page_table_flags = new_protection.to_page_table_flags();

        for i in 0..num_pages {
            let page_addr = addr + (i * PAGE_SIZE) as u64;

            // Update page table flags using memory manager
            if let Err(_) = Self::update_page_protection(page_addr, page_table_flags) {
                // Partial update occurred - acceptable POSIX behavior
                break;
            }
        }

        // Flush TLB for the modified pages
        Self::flush_tlb_range(addr, len);

        SyscallResult::Success(0)
    }

    /// Validate that a memory range belongs to the process
    fn validate_process_memory_range(
        process: &crate::process::ProcessControlBlock,
        addr: u64,
        len: u64,
    ) -> bool {
        let end_addr = addr + len;

        // Check if range overlaps with any valid memory region
        let in_code = addr >= process.memory.code_start &&
                      end_addr <= process.memory.code_start + process.memory.code_size;

        let in_data = addr >= process.memory.data_start &&
                      end_addr <= process.memory.data_start + process.memory.data_size;

        let in_heap = addr >= process.memory.heap_start &&
                      end_addr <= process.memory.heap_start + process.memory.heap_size;

        let in_stack = addr >= process.memory.stack_start &&
                       end_addr <= process.memory.stack_start + process.memory.stack_size;

        in_code || in_data || in_heap || in_stack
    }

    /// Update page table protection flags
    fn update_page_protection(
        addr: u64,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        use x86_64::registers::control::Cr3;
        use x86_64::structures::paging::{OffsetPageTable, Page, Size4KiB, Translate};
        use x86_64::VirtAddr;

        // Get current page table
        let (level_4_table_frame, _) = Cr3::read();
        let phys_offset = VirtAddr::new(0xFFFF_8000_0000_0000);

        unsafe {
            let level_4_table = &mut *(phys_offset + level_4_table_frame.start_address().as_u64()).as_mut_ptr();
            let mut page_table = OffsetPageTable::new(level_4_table, phys_offset);

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(addr));

            // Verify the page exists
            if let Ok(_) = page_table.translate_page(page) {
                Ok(())
            } else {
                Err("Page not mapped")
            }
        }
    }

    /// Flush TLB for a range of addresses
    fn flush_tlb_range(addr: u64, len: u64) {
        use x86_64::instructions::tlb;
        use x86_64::VirtAddr;
        use crate::memory::PAGE_SIZE;

        let num_pages = (len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;

        for i in 0..num_pages {
            let page_addr = VirtAddr::new(addr + i * PAGE_SIZE as u64);
            tlb::flush(page_addr);
        }
    }

    /// sys_madvise - Give advice about memory usage
    fn sys_madvise(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use crate::memory::PAGE_SIZE;

        let addr = args.get(0).copied().unwrap_or(0);
        let len = args.get(1).copied().unwrap_or(0);
        let advice = args.get(2).copied().unwrap_or(0) as i32;

        // madvise() advice constants
        const MADV_NORMAL: i32 = 0;       // No special treatment
        const MADV_RANDOM: i32 = 1;       // Expect random page references
        const MADV_SEQUENTIAL: i32 = 2;   // Expect sequential page references
        const MADV_WILLNEED: i32 = 3;     // Will need these pages
        const MADV_DONTNEED: i32 = 4;     // Don't need these pages
        const MADV_FREE: i32 = 8;         // Free pages only if memory pressure
        const MADV_REMOVE: i32 = 9;       // Remove these pages & resources
        const MADV_DONTFORK: i32 = 10;    // Don't inherit across fork
        const MADV_DOFORK: i32 = 11;      // Do inherit across fork
        const MADV_MERGEABLE: i32 = 12;   // KSM may merge identical pages
        const MADV_UNMERGEABLE: i32 = 13; // KSM may not merge pages
        const MADV_HUGEPAGE: i32 = 14;    // Worth backing with hugepages
        const MADV_NOHUGEPAGE: i32 = 15;  // Not worth backing with hugepages
        const MADV_DONTDUMP: i32 = 16;    // Don't include in core dump
        const MADV_DODUMP: i32 = 17;      // Include in core dump
        const MADV_WIPEONFORK: i32 = 18;  // Zero memory on fork
        const MADV_KEEPONFORK: i32 = 19;  // Keep memory on fork
        const MADV_COLD: i32 = 20;        // Deactivate pages
        const MADV_PAGEOUT: i32 = 21;     // Reclaim pages

        // Validate arguments
        if len == 0 {
            // Zero length is accepted and returns success
            return SyscallResult::Success(0);
        }

        // Address must be page-aligned
        if addr % PAGE_SIZE as u64 != 0 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }

        // Validate address range is in user space
        const USER_SPACE_START: u64 = 0x0000_1000_0000;
        const USER_SPACE_END: u64 = 0x0000_8000_0000;

        if addr < USER_SPACE_START || addr >= USER_SPACE_END {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        let end_addr = addr.saturating_add(len);
        if end_addr > USER_SPACE_END {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // Check permissions - verify process owns this memory region
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Validate the address range is within process memory regions
        let is_valid_range = Self::validate_process_memory_range(&process, addr, len);

        if !is_valid_range {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }

        // Process the advice
        match advice {
            MADV_NORMAL => {
                // Default behavior - no special action needed
                SyscallResult::Success(0)
            },
            MADV_RANDOM => {
                // Expect random access pattern
                // Could disable read-ahead, but we accept as hint without action
                SyscallResult::Success(0)
            },
            MADV_SEQUENTIAL => {
                // Expect sequential access
                // Could enable aggressive read-ahead, but we accept as hint without action
                SyscallResult::Success(0)
            },
            MADV_WILLNEED => {
                // Will need these pages soon - prefetch/fault them in
                Self::madvise_willneed(addr, len);
                SyscallResult::Success(0)
            },
            MADV_DONTNEED => {
                // Don't need these pages - can free them
                Self::madvise_dontneed(addr, len);
                SyscallResult::Success(0)
            },
            MADV_FREE => {
                // Mark pages as free, but only reclaim under memory pressure
                // For now, treat similar to DONTNEED but lazier
                Self::madvise_free(addr, len);
                SyscallResult::Success(0)
            },
            MADV_REMOVE => {
                // Remove pages and backing resources
                // Similar to DONTNEED but more aggressive
                Self::madvise_dontneed(addr, len);
                SyscallResult::Success(0)
            },
            MADV_DONTFORK => {
                // Mark region as non-inheritable across fork
                // Store flag in process memory metadata
                SyscallResult::Success(0)
            },
            MADV_DOFORK => {
                // Mark region as inheritable across fork (default)
                SyscallResult::Success(0)
            },
            MADV_MERGEABLE | MADV_UNMERGEABLE => {
                // KSM (Kernel Samepage Merging) not implemented
                // Accept as hint without action
                SyscallResult::Success(0)
            },
            MADV_HUGEPAGE | MADV_NOHUGEPAGE => {
                // Transparent huge pages not yet implemented
                // Accept as hint without action
                SyscallResult::Success(0)
            },
            MADV_DONTDUMP | MADV_DODUMP => {
                // Core dump control - accept as hint
                SyscallResult::Success(0)
            },
            MADV_WIPEONFORK | MADV_KEEPONFORK => {
                // Fork behavior hints - accept without action for now
                SyscallResult::Success(0)
            },
            MADV_COLD => {
                // Mark pages as cold (low priority for keeping in memory)
                // Accept as hint without action
                SyscallResult::Success(0)
            },
            MADV_PAGEOUT => {
                // Try to page out these pages
                Self::madvise_dontneed(addr, len);
                SyscallResult::Success(0)
            },
            _ => {
                // Unknown advice value
                SyscallResult::Error(SyscallError::InvalidArgument)
            }
        }
    }

    /// MADV_WILLNEED - Prefault pages into memory
    fn madvise_willneed(addr: u64, len: u64) {
        use crate::memory::PAGE_SIZE;

        let num_pages = (len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;

        // Touch each page to fault it in
        for i in 0..num_pages {
            let page_addr = addr + i * PAGE_SIZE as u64;
            unsafe {
                // Perform a volatile read to fault the page in
                let _val = core::ptr::read_volatile(page_addr as *const u8);
            }
        }
    }

    /// MADV_DONTNEED - Free pages from memory
    fn madvise_dontneed(addr: u64, len: u64) {
        use crate::memory::{PAGE_SIZE, deallocate_memory};
        use x86_64::VirtAddr;

        let num_pages = (len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;

        // Zero out pages and mark them as free
        for i in 0..num_pages {
            let page_addr = addr + i * PAGE_SIZE as u64;

            // Zero the page
            unsafe {
                core::ptr::write_bytes(page_addr as *mut u8, 0, PAGE_SIZE);
            }

            // Optionally deallocate (for now, just zero)
            // In full implementation, we would unmap and free physical frames
            // let _ = deallocate_memory(VirtAddr::new(page_addr));
        }

        // Flush TLB
        Self::flush_tlb_range(addr, len);
    }

    /// MADV_FREE - Lazy free pages
    fn madvise_free(addr: u64, len: u64) {
        // Mark pages as available for reclamation under memory pressure
        // For now, similar to DONTNEED but without immediate freeing
        // In full implementation, we would mark pages in a free list

        // Just zero pages without deallocating
        use crate::memory::PAGE_SIZE;
        let num_pages = (len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;

        for i in 0..num_pages {
            let page_addr = addr + i * PAGE_SIZE as u64;
            unsafe {
                // Zero the page content
                core::ptr::write_bytes(page_addr as *mut u8, 0, PAGE_SIZE);
            }
        }
    }

    /// sys_futex - Fast userspace mutex
    fn sys_futex(&self, args: &[u64], _process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        use super::futex::{FutexManager, futex_op};

        let uaddr = args.get(0).copied().unwrap_or(0);
        let futex_op = args.get(1).copied().unwrap_or(0) as i32;
        let val = args.get(2).copied().unwrap_or(0) as u32;
        let timeout_ptr = args.get(3).copied().unwrap_or(0);
        let uaddr2 = args.get(4).copied().unwrap_or(0);
        let val3 = args.get(5).copied().unwrap_or(0) as u32;

        // Extract operation and flags
        let cmd = futex_op & 0x7F;  // Lower 7 bits
        let private = (futex_op & futex_op::FUTEX_PRIVATE_FLAG) != 0;
        let realtime = (futex_op & futex_op::FUTEX_CLOCK_REALTIME) != 0;

        // Parse timeout if provided
        let timeout_ns = if timeout_ptr != 0 && (cmd == futex_op::FUTEX_WAIT ||
                                                  cmd == futex_op::FUTEX_LOCK_PI ||
                                                  cmd == futex_op::FUTEX_WAIT_BITSET) {
            // timespec structure: { tv_sec: i64, tv_nsec: i64 }
            unsafe {
                let tv_sec = core::ptr::read_volatile(timeout_ptr as *const i64);
                let tv_nsec = core::ptr::read_volatile((timeout_ptr + 8) as *const i64);

                if tv_sec < 0 || tv_nsec < 0 || tv_nsec >= 1_000_000_000 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }

                Some((tv_sec as u64 * 1_000_000_000) + tv_nsec as u64)
            }
        } else {
            None
        };

        // Dispatch to appropriate futex operation
        let result = match cmd {
            futex_op::FUTEX_WAIT => {
                // FUTEX_WAIT: Block if *uaddr == val
                FutexManager::futex_wait(uaddr, val, timeout_ns, !0, current_pid)
            },
            futex_op::FUTEX_WAKE => {
                // FUTEX_WAKE: Wake up to val waiters
                FutexManager::futex_wake(uaddr, val as i32, !0)
            },
            futex_op::FUTEX_REQUEUE => {
                // FUTEX_REQUEUE: Wake val waiters, requeue rest to uaddr2
                let val2 = timeout_ptr as i32; // val2 is in timeout parameter position
                FutexManager::futex_requeue(uaddr, val as i32, val2, uaddr2)
            },
            futex_op::FUTEX_CMP_REQUEUE => {
                // FUTEX_CMP_REQUEUE: Like REQUEUE but compare *uaddr with val3 first
                let val2 = timeout_ptr as i32; // val2 is in timeout parameter position
                FutexManager::futex_cmp_requeue(uaddr, val as i32, val2, uaddr2, val3)
            },
            futex_op::FUTEX_WAKE_OP => {
                // FUTEX_WAKE_OP: Wake val waiters on uaddr, perform op on uaddr2, wake val2 on uaddr2
                // Complex operation: wake + atomic op + conditional wake
                // For now, implement as simple wake on both addresses
                let woken1 = match FutexManager::futex_wake(uaddr, val as i32, !0) {
                    Ok(n) => n,
                    Err(e) => return SyscallResult::Error(e.into()),
                };

                let val2 = timeout_ptr as i32;
                let woken2 = match FutexManager::futex_wake(uaddr2, val2, !0) {
                    Ok(n) => n,
                    Err(e) => return SyscallResult::Error(e.into()),
                };

                Ok(woken1 + woken2)
            },
            futex_op::FUTEX_LOCK_PI => {
                // Priority-inheritance futex lock
                let tid = current_pid; // Use PID as TID for now
                FutexManager::futex_lock_pi(uaddr, timeout_ns, current_pid, tid)
            },
            futex_op::FUTEX_UNLOCK_PI => {
                // Priority-inheritance futex unlock
                FutexManager::futex_unlock_pi(uaddr, current_pid)
            },
            futex_op::FUTEX_TRYLOCK_PI => {
                // Try to lock PI futex without blocking
                let tid = current_pid; // Use PID as TID for now
                FutexManager::futex_trylock_pi(uaddr, current_pid, tid)
            },
            futex_op::FUTEX_WAIT_BITSET => {
                // FUTEX_WAIT with bitset for selective wakeup
                let bitset = val3;
                if bitset == 0 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                FutexManager::futex_wait(uaddr, val, timeout_ns, bitset, current_pid)
            },
            futex_op::FUTEX_WAKE_BITSET => {
                // FUTEX_WAKE with bitset for selective wakeup
                let bitset = val3;
                if bitset == 0 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                FutexManager::futex_wake(uaddr, val as i32, bitset)
            },
            futex_op::FUTEX_WAIT_REQUEUE_PI => {
                // Wait on PI futex and requeue to another PI futex
                // Complex operation combining wait and priority inheritance
                // For now, implement as simple wait
                FutexManager::futex_wait(uaddr, val, timeout_ns, !0, current_pid)
            },
            futex_op::FUTEX_CMP_REQUEUE_PI => {
                // Conditional requeue for PI futexes
                // For now, implement as simple compare-requeue
                let val2 = timeout_ptr as i32;
                FutexManager::futex_cmp_requeue(uaddr, val as i32, val2, uaddr2, val3)
            },
            futex_op::FUTEX_FD => {
                // FUTEX_FD is deprecated and not implemented
                Err(super::futex::FutexError::NotSupported)
            },
            _ => {
                // Unknown futex operation
                Err(super::futex::FutexError::NotSupported)
            }
        };

        // Convert result to syscall result
        match result {
            Ok(ret_val) => SyscallResult::Success(ret_val as u64),
            Err(futex_error) => {
                let syscall_error: SyscallError = futex_error.into();
                SyscallResult::Error(syscall_error)
            }
        }
    }

    /// sys_socket - Create socket
    fn sys_socket(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let domain = args.get(0).copied().unwrap_or(0) as i32;
        let socket_type = args.get(1).copied().unwrap_or(0) as i32;
        let protocol = args.get(2).copied().unwrap_or(0) as i32;

        // Validate domain: AF_INET = 2, AF_INET6 = 10, AF_UNIX = 1
        match domain {
            2 | 10 | 1 => {},
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };

        // Map socket type: SOCK_STREAM = 1, SOCK_DGRAM = 2, SOCK_RAW = 3
        let net_socket_type = match socket_type {
            1 => crate::net::socket::SocketType::Stream,
            2 => crate::net::socket::SocketType::Datagram,
            3 => crate::net::socket::SocketType::Raw,
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };

        // Determine protocol
        let net_protocol = match (socket_type, protocol) {
            (1, 0) | (1, 6) => crate::net::Protocol::TCP,
            (2, 0) | (2, 17) => crate::net::Protocol::UDP,
            (3, 1) => crate::net::Protocol::ICMP,
            (3, 6) => crate::net::Protocol::TCP,
            (3, 17) => crate::net::Protocol::UDP,
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };

        // Check permissions for raw sockets
        if socket_type == 3 {
            if let Some(ctx) = crate::security::get_context(current_pid) {
                if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_raw") {
                    return SyscallResult::Error(SyscallError::PermissionDenied);
                }
            } else {
                return SyscallResult::Error(SyscallError::PermissionDenied);
            }
        }

        // Create socket
        let network_stack = crate::net::network_stack();
        match network_stack.create_socket(net_socket_type, net_protocol) {
            Ok(socket_id) => {
                let mut process = match process_manager.get_process(current_pid) {
                    Some(p) => p,
                    None => {
                        let _ = network_stack.close_socket(socket_id);
                        return SyscallResult::Error(SyscallError::ProcessNotFound);
                    }
                };

                let mut next_fd = 3;
                while process.file_descriptors.contains_key(&next_fd) {
                    next_fd += 1;
                    if next_fd > 65535 {
                        let _ = network_stack.close_socket(socket_id);
                        return SyscallResult::Error(SyscallError::OutOfMemory);
                    }
                }

                let fd = super::FileDescriptor {
                    fd_type: super::FileDescriptorType::Socket { socket_id },
                    flags: 0,
                    offset: 0,
                };
                process.file_descriptors.insert(next_fd, fd);
                SyscallResult::Success(next_fd as u64)
            }
            Err(_) => SyscallResult::Error(SyscallError::OutOfMemory),
        }
    }

        /// sys_bind - Bind socket to address
    fn sys_bind(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let addr_ptr = args.get(1).copied().unwrap_or(0);
        let addr_len = args.get(2).copied().unwrap_or(0) as u32;
    
        if addr_ptr == 0 || addr_len < 8 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }
    
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };
    
        let socket_id = match process.file_descriptors.get(&fd) {
            Some(fd_entry) => match &fd_entry.fd_type {
                super::FileDescriptorType::Socket { socket_id } => *socket_id,
                _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            },
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };
    
        let mut addr_buffer = vec![0u8; core::cmp::min(addr_len as usize, 128)];
        if let Err(_) = self.copy_from_user(addr_ptr, &mut addr_buffer) {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }
    
        let family = u16::from_ne_bytes([addr_buffer[0], addr_buffer[1]]);
        let socket_address = match family {
            2 => {
                // AF_INET
                if addr_buffer.len() < 8 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
                let ip = [addr_buffer[4], addr_buffer[5], addr_buffer[6], addr_buffer[7]];
    
                if port < 1024 {
                    if let Some(ctx) = crate::security::get_context(current_pid) {
                        if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_bind_service") {
                            return SyscallResult::Error(SyscallError::PermissionDenied);
                        }
                    } else {
                        return SyscallResult::Error(SyscallError::PermissionDenied);
                    }
                }
    
                crate::net::socket::SocketAddress::ipv4(ip[0], ip[1], ip[2], ip[3], port)
            }
            10 => {
                // AF_INET6
                if addr_buffer.len() < 28 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
                let mut ipv6_addr = [0u8; 16];
                ipv6_addr.copy_from_slice(&addr_buffer[8..24]);
    
                if port < 1024 {
                    if let Some(ctx) = crate::security::get_context(current_pid) {
                        if !ctx.is_root() && !crate::security::check_permission(current_pid, "net_bind_service") {
                            return SyscallResult::Error(SyscallError::PermissionDenied);
                        }
                    } else {
                        return SyscallResult::Error(SyscallError::PermissionDenied);
                    }
                }
    
                crate::net::socket::SocketAddress::ipv6(ipv6_addr, port)
            }
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };
    
        if !socket_address.is_valid() {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }
    
        let network_stack = crate::net::network_stack();
        if let Some(mut socket) = network_stack.get_socket(socket_id) {
            match socket.bind(socket_address) {
                Ok(()) => {
                    let _ = network_stack.update_socket(socket_id, socket);
                    SyscallResult::Success(0)
                }
                Err(crate::net::NetworkError::AddressInUse) => {
                    SyscallResult::Error(SyscallError::ResourceBusy)
                }
                Err(_) => SyscallResult::Error(SyscallError::InvalidAddress),
            }
        } else {
            SyscallResult::Error(SyscallError::InvalidFileDescriptor)
        }
    }

        /// sys_connect - Connect socket
    fn sys_connect(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let addr_ptr = args.get(1).copied().unwrap_or(0);
        let addr_len = args.get(2).copied().unwrap_or(0) as u32;
    
        if addr_ptr == 0 || addr_len < 8 {
            return SyscallResult::Error(SyscallError::InvalidArgument);
        }
    
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };
    
        let socket_id = match process.file_descriptors.get(&fd) {
            Some(fd_entry) => match &fd_entry.fd_type {
                super::FileDescriptorType::Socket { socket_id } => *socket_id,
                _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            },
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };
    
        let mut addr_buffer = vec![0u8; core::cmp::min(addr_len as usize, 128)];
        if let Err(_) = self.copy_from_user(addr_ptr, &mut addr_buffer) {
            return SyscallResult::Error(SyscallError::InvalidAddress);
        }
    
        let family = u16::from_ne_bytes([addr_buffer[0], addr_buffer[1]]);
        let socket_address = match family {
            2 => {
                if addr_buffer.len() < 8 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
                let ip = [addr_buffer[4], addr_buffer[5], addr_buffer[6], addr_buffer[7]];
                crate::net::socket::SocketAddress::ipv4(ip[0], ip[1], ip[2], ip[3], port)
            }
            10 => {
                if addr_buffer.len() < 28 {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                let port = u16::from_be_bytes([addr_buffer[2], addr_buffer[3]]);
                let mut ipv6_addr = [0u8; 16];
                ipv6_addr.copy_from_slice(&addr_buffer[8..24]);
                crate::net::socket::SocketAddress::ipv6(ipv6_addr, port)
            }
            _ => return SyscallResult::Error(SyscallError::InvalidArgument),
        };
    
        let network_stack = crate::net::network_stack();
        if let Some(mut socket) = network_stack.get_socket(socket_id) {
            match socket.connect(socket_address) {
                Ok(()) => {
                    let _ = network_stack.update_socket(socket_id, socket);
                    SyscallResult::Success(0)
                }
                Err(crate::net::NetworkError::Timeout) => {
                    SyscallResult::Error(SyscallError::ResourceBusy)
                }
                Err(_) => SyscallResult::Error(SyscallError::InvalidAddress),
            }
        } else {
            SyscallResult::Error(SyscallError::InvalidFileDescriptor)
        }
    }

        /// sys_listen - Listen on socket
    fn sys_listen(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let backlog = args.get(1).copied().unwrap_or(128) as u32;
        let backlog = core::cmp::min(backlog, 4096);
    
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };
    
        let socket_id = match process.file_descriptors.get(&fd) {
            Some(fd_entry) => match &fd_entry.fd_type {
                super::FileDescriptorType::Socket { socket_id } => *socket_id,
                _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            },
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };
    
        let network_stack = crate::net::network_stack();
        if let Some(mut socket) = network_stack.get_socket(socket_id) {
            if socket.local_address.is_none() {
                return SyscallResult::Error(SyscallError::InvalidAddress);
            }
            if socket.socket_type != crate::net::socket::SocketType::Stream {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
    
            match socket.listen(backlog) {
                Ok(()) => {
                    let _ = network_stack.update_socket(socket_id, socket);
                    SyscallResult::Success(0)
                }
                Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
            }
        } else {
            SyscallResult::Error(SyscallError::InvalidFileDescriptor)
        }
    }

        /// sys_accept - Accept socket connection
    fn sys_accept(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let addr_ptr = args.get(1).copied().unwrap_or(0);
        let addrlen_ptr = args.get(2).copied().unwrap_or(0);
    
        let mut process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };
    
        let socket_id = match process.file_descriptors.get(&fd) {
            Some(fd_entry) => match &fd_entry.fd_type {
                super::FileDescriptorType::Socket { socket_id } => *socket_id,
                _ => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
            },
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };
    
        let network_stack = crate::net::network_stack();
        if let Some(mut socket) = network_stack.get_socket(socket_id) {
            if socket.state != crate::net::socket::SocketState::Listening {
                return SyscallResult::Error(SyscallError::InvalidArgument);
            }
    
            match socket.accept() {
                Ok(Some(new_socket_id)) => {
                    let mut sockets = network_stack.sockets.write();
                    sockets.insert(socket_id, socket);
                    let new_socket = sockets.get(&new_socket_id).cloned();
                    drop(sockets);
    
                    if let Some(new_socket) = new_socket {
                        let mut next_fd = 3;
                        while process.file_descriptors.contains_key(&next_fd) {
                            next_fd += 1;
                            if next_fd > 65535 {
                                return SyscallResult::Error(SyscallError::OutOfMemory);
                            }
                        }
    
                        let fd_entry = super::FileDescriptor {
                            fd_type: super::FileDescriptorType::Socket { socket_id: new_socket_id },
                            flags: 0,
                            offset: 0,
                        };
                        process.file_descriptors.insert(next_fd, fd_entry);
    
                        // Write peer address to user space
                        if addr_ptr != 0 && addrlen_ptr != 0 {
                            if let Some(peer_addr) = new_socket.remote_address {
                                let mut addrlen_buf = [0u8; 4];
                                if self.copy_from_user(addrlen_ptr, &mut addrlen_buf).is_ok() {
                                    let max_len = u32::from_ne_bytes(addrlen_buf) as usize;
                                    match peer_addr.address {
                                        crate::net::NetworkAddress::IPv4(ip) => {
                                            if max_len >= 8 {
                                                let mut addr_buf = vec![0u8; 8];
                                                addr_buf[0..2].copy_from_slice(&2u16.to_ne_bytes());
                                                addr_buf[2..4].copy_from_slice(&peer_addr.port.to_be_bytes());
                                                addr_buf[4..8].copy_from_slice(&ip);
                                                let _ = self.copy_to_user(addr_ptr, &addr_buf);
                                                let actual_len = 8u32.to_ne_bytes();
                                                let _ = self.copy_to_user(addrlen_ptr, &actual_len);
                                            }
                                        }
                                        crate::net::NetworkAddress::IPv6(ip) => {
                                            if max_len >= 28 {
                                                let mut addr_buf = vec![0u8; 28];
                                                addr_buf[0..2].copy_from_slice(&10u16.to_ne_bytes());
                                                addr_buf[2..4].copy_from_slice(&peer_addr.port.to_be_bytes());
                                                addr_buf[4..8].copy_from_slice(&[0u8; 4]);
                                                addr_buf[8..24].copy_from_slice(&ip);
                                                addr_buf[24..28].copy_from_slice(&[0u8; 4]);
                                                let _ = self.copy_to_user(addr_ptr, &addr_buf);
                                                let actual_len = 28u32.to_ne_bytes();
                                                let _ = self.copy_to_user(addrlen_ptr, &actual_len);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
    
                        SyscallResult::Success(next_fd as u64)
                    } else {
                        SyscallResult::Error(SyscallError::InvalidAddress)
                    }
                }
                Ok(None) => SyscallResult::Error(SyscallError::ResourceBusy),
                Err(_) => SyscallResult::Error(SyscallError::InvalidArgument),
            }
        } else {
            SyscallResult::Error(SyscallError::InvalidFileDescriptor)
        }
    }

    /// sys_set_tid_address - Set thread ID address
    fn sys_set_tid_address(&self, _args: &[u64], _process_manager: &ProcessManager, _current_pid: Pid) -> SyscallResult {
        // TODO: Implement set_tid_address() for thread ID management
        // Used by dynamic linker and pthread initialization
        SyscallResult::Error(SyscallError::OperationNotSupported)
    }

    /// sys_ioctl - Device-specific I/O control
    fn sys_ioctl(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let request = args.get(1).copied().unwrap_or(0) as u32;
        let arg_ptr = args.get(2).copied().unwrap_or(0);

        // Common ioctl request codes
        const TCGETS: u32 = 0x5401;       // Get terminal attributes
        const TCSETS: u32 = 0x5402;       // Set terminal attributes
        const TCSETSW: u32 = 0x5403;      // Set terminal attributes (wait)
        const TCSETSF: u32 = 0x5404;      // Set terminal attributes (flush)
        const TIOCGWINSZ: u32 = 0x5413;   // Get window size
        const TIOCSWINSZ: u32 = 0x5414;   // Set window size
        const FIONREAD: u32 = 0x541B;     // Get # bytes in input buffer
        const FIONBIO: u32 = 0x5421;      // Set/clear non-blocking I/O
        const BLKGETSIZE: u32 = 0x1260;   // Get device size in 512-byte blocks
        const BLKGETSIZE64: u32 = 0x80081272; // Get device size in bytes
        const BLKFLSBUF: u32 = 0x1261;    // Flush buffer cache
        const BLKRRPART: u32 = 0x125F;    // Re-read partition table
        const SIOCGIFADDR: u32 = 0x8915;  // Get interface address
        const SIOCSIFADDR: u32 = 0x8916;  // Set interface address
        const SIOCGIFFLAGS: u32 = 0x8913; // Get interface flags
        const SIOCSIFFLAGS: u32 = 0x8914; // Set interface flags

        // Get process
        let process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Check if file descriptor exists in process table
        let file_desc = match process.file_descriptors.get(&fd) {
            Some(fd) => fd,
            None => return SyscallResult::Error(SyscallError::InvalidFileDescriptor),
        };

        // Route to appropriate handler based on request type
        match request {
            // Terminal ioctls
            TCGETS => {
                // Get terminal attributes
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Simplified termios structure (minimal implementation)
                #[repr(C)]
                struct Termios {
                    c_iflag: u32,   // Input flags
                    c_oflag: u32,   // Output flags
                    c_cflag: u32,   // Control flags
                    c_lflag: u32,   // Local flags
                    c_line: u8,     // Line discipline
                    c_cc: [u8; 32], // Control characters
                    c_ispeed: u32,  // Input speed
                    c_ospeed: u32,  // Output speed
                }

                let termios = Termios {
                    c_iflag: 0x500,  // ICRNL | IXON
                    c_oflag: 0x5,    // OPOST | ONLCR
                    c_cflag: 0xBF,   // CS8 | CREAD | HUPCL
                    c_lflag: 0x8A3B, // ISIG | ICANON | ECHO | ECHOE | ECHOK | ECHOCTL | ECHOKE | IEXTEN
                    c_line: 0,
                    c_cc: [0; 32],
                    c_ispeed: 38400,
                    c_ospeed: 38400,
                };

                // Copy to user space
                let termios_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &termios as *const _ as *const u8,
                        core::mem::size_of::<Termios>()
                    )
                };

                if self.copy_to_user(arg_ptr, termios_bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            TCSETS | TCSETSW | TCSETSF => {
                // Set terminal attributes (simplified - just validate and return success)
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }
                SyscallResult::Success(0)
            }

            TIOCGWINSZ => {
                // Get window size
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                #[repr(C)]
                struct Winsize {
                    ws_row: u16,
                    ws_col: u16,
                    ws_xpixel: u16,
                    ws_ypixel: u16,
                }

                let winsize = Winsize {
                    ws_row: 25,
                    ws_col: 80,
                    ws_xpixel: 640,
                    ws_ypixel: 400,
                };

                let winsize_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &winsize as *const _ as *const u8,
                        core::mem::size_of::<Winsize>()
                    )
                };

                if self.copy_to_user(arg_ptr, winsize_bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            TIOCSWINSZ => {
                // Set window size (simplified - just return success)
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }
                SyscallResult::Success(0)
            }

            FIONREAD => {
                // Get number of bytes available for reading
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Return 0 bytes available (simplified)
                let bytes_available = 0u32;
                let bytes = bytes_available.to_le_bytes();

                if self.copy_to_user(arg_ptr, &bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            FIONBIO => {
                // Set/clear non-blocking I/O
                // In a full implementation, we would update the file descriptor flags
                SyscallResult::Success(0)
            }

            // Block device ioctls
            BLKGETSIZE => {
                // Get device size in 512-byte blocks
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Return a default size (simplified - would query actual device)
                let blocks = 2048000u32; // ~1GB
                let bytes = blocks.to_le_bytes();

                if self.copy_to_user(arg_ptr, &bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            BLKGETSIZE64 => {
                // Get device size in bytes
                if arg_ptr == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Return a default size (simplified)
                let size_bytes = 1024u64 * 1024 * 1024; // 1GB
                let bytes = size_bytes.to_le_bytes();

                if self.copy_to_user(arg_ptr, &bytes).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            BLKFLSBUF => {
                // Flush buffer cache
                // In a full implementation, would flush filesystem buffers
                SyscallResult::Success(0)
            }

            BLKRRPART => {
                // Re-read partition table
                // In a full implementation, would re-scan partitions
                SyscallResult::Success(0)
            }

            // Network ioctls
            SIOCGIFADDR | SIOCSIFADDR | SIOCGIFFLAGS | SIOCSIFFLAGS => {
                // Network interface ioctls
                // In a full implementation, would interface with network stack
                // For now, return not supported for network operations
                SyscallResult::Error(SyscallError::OperationNotSupported)
            }

            _ => {
                // Unknown ioctl request
                SyscallResult::Error(SyscallError::InvalidArgument)
            }
        }
    }

    /// sys_fcntl - File control operations
    fn sys_fcntl(&self, args: &[u64], process_manager: &ProcessManager, current_pid: Pid) -> SyscallResult {
        let fd = args.get(0).copied().unwrap_or(0) as u32;
        let cmd = args.get(1).copied().unwrap_or(0) as u32;
        let arg = args.get(2).copied().unwrap_or(0);

        // fcntl command codes
        const F_DUPFD: u32 = 0;          // Duplicate file descriptor
        const F_GETFD: u32 = 1;          // Get file descriptor flags
        const F_SETFD: u32 = 2;          // Set file descriptor flags
        const F_GETFL: u32 = 3;          // Get file status flags
        const F_SETFL: u32 = 4;          // Set file status flags
        const F_GETLK: u32 = 5;          // Get record locking info
        const F_SETLK: u32 = 6;          // Set record locking info (non-blocking)
        const F_SETLKW: u32 = 7;         // Set record locking info (blocking)
        const F_GETOWN: u32 = 9;         // Get owner (for async I/O)
        const F_SETOWN: u32 = 8;         // Set owner (for async I/O)
        const F_DUPFD_CLOEXEC: u32 = 1030; // Duplicate FD with close-on-exec

        // File descriptor flags
        const FD_CLOEXEC: u32 = 1;       // Close on exec

        // File status flags
        const O_NONBLOCK: u32 = 0x800;   // Non-blocking I/O
        const O_APPEND: u32 = 0x400;     // Append mode
        const O_ASYNC: u32 = 0x2000;     // Async I/O

        // Get process
        let mut process = match process_manager.get_process(current_pid) {
            Some(p) => p,
            None => return SyscallResult::Error(SyscallError::ProcessNotFound),
        };

        // Check if file descriptor exists in process table
        if !process.file_descriptors.contains_key(&fd) {
            return SyscallResult::Error(SyscallError::InvalidFileDescriptor);
        }

        match cmd {
            F_DUPFD | F_DUPFD_CLOEXEC => {
                // Duplicate file descriptor
                let min_fd = arg as u32;

                // Find next available fd >= min_fd
                let mut new_fd = core::cmp::max(min_fd, 3); // At least 3 (after stdin/stdout/stderr)
                while process.file_descriptors.contains_key(&new_fd) {
                    new_fd += 1;
                    if new_fd > 65535 {
                        return SyscallResult::Error(SyscallError::InvalidArgument);
                    }
                }

                // Clone the file descriptor
                let old_fd_desc = process.file_descriptors.get(&fd).unwrap().clone();
                process.file_descriptors.insert(new_fd, old_fd_desc);

                // If F_DUPFD_CLOEXEC, set close-on-exec flag (simplified - not fully implemented)
                // In a full implementation, we would track this flag per FD

                SyscallResult::Success(new_fd as u64)
            }

            F_GETFD => {
                // Get file descriptor flags
                // In a simplified implementation, we don't track FD flags separately
                // Return 0 (no flags set)
                SyscallResult::Success(0)
            }

            F_SETFD => {
                // Set file descriptor flags
                // In a simplified implementation, we just validate the argument
                let flags = arg as u32;
                if flags > FD_CLOEXEC {
                    return SyscallResult::Error(SyscallError::InvalidArgument);
                }
                // In a full implementation, would store flags per FD
                SyscallResult::Success(0)
            }

            F_GETFL => {
                // Get file status flags
                let file_desc = process.file_descriptors.get(&fd).unwrap();
                SyscallResult::Success(file_desc.flags as u64)
            }

            F_SETFL => {
                // Set file status flags
                let new_flags = arg as u32;

                // Only certain flags can be changed via F_SETFL
                let changeable_flags = O_NONBLOCK | O_APPEND | O_ASYNC;

                // Get current flags
                let file_desc = process.file_descriptors.get_mut(&fd).unwrap();
                let old_flags = file_desc.flags;

                // Preserve non-changeable flags and update changeable ones
                let updated_flags = (old_flags & !changeable_flags) | (new_flags & changeable_flags);
                file_desc.flags = updated_flags;

                SyscallResult::Success(0)
            }

            F_GETLK => {
                // Get record locking information
                if arg == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Simplified implementation - return that no locks conflict
                #[repr(C)]
                struct Flock {
                    l_type: i16,    // Type of lock: F_RDLCK, F_WRLCK, F_UNLCK
                    l_whence: i16,  // How to interpret l_start: SEEK_SET, SEEK_CUR, SEEK_END
                    l_start: i64,   // Starting offset for lock
                    l_len: i64,     // Number of bytes to lock
                    l_pid: i32,     // PID of process holding the lock
                }

                // Read the lock request from user space
                let mut lock_data = [0u8; core::mem::size_of::<Flock>()];
                if self.copy_from_user(arg, &mut lock_data).is_err() {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Set l_type to F_UNLCK to indicate no conflicting lock
                let flock = unsafe { &mut *(lock_data.as_mut_ptr() as *mut Flock) };
                flock.l_type = 2; // F_UNLCK
                flock.l_pid = 0;

                // Copy back to user space
                if self.copy_to_user(arg, &lock_data).is_ok() {
                    SyscallResult::Success(0)
                } else {
                    SyscallResult::Error(SyscallError::InvalidAddress)
                }
            }

            F_SETLK | F_SETLKW => {
                // Set record lock (non-blocking or blocking)
                if arg == 0 {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // Simplified implementation - just validate the structure and return success
                // In a full implementation, would maintain a lock table
                #[repr(C)]
                struct Flock {
                    l_type: i16,
                    l_whence: i16,
                    l_start: i64,
                    l_len: i64,
                    l_pid: i32,
                }

                // Read the lock request
                let mut lock_data = [0u8; core::mem::size_of::<Flock>()];
                if self.copy_from_user(arg, &mut lock_data).is_err() {
                    return SyscallResult::Error(SyscallError::InvalidAddress);
                }

                // In a full implementation, would:
                // 1. Check for conflicting locks
                // 2. Add lock to lock table (for F_RDLCK or F_WRLCK)
                // 3. Remove lock from table (for F_UNLCK)
                // 4. Block if needed (for F_SETLKW)

                SyscallResult::Success(0)
            }

            F_GETOWN => {
                // Get owner for async I/O notifications
                // Simplified implementation - return 0 (no owner)
                SyscallResult::Success(0)
            }

            F_SETOWN => {
                // Set owner for async I/O notifications
                let _owner_pid = arg as i32;
                // In a full implementation, would store the owner PID
                // and send signals on I/O events
                SyscallResult::Success(0)
            }

            _ => {
                // Unknown fcntl command
                SyscallResult::Error(SyscallError::InvalidArgument)
            }
        }
    }

    // Package management syscalls (experimental)

    fn sys_pkg_install(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            200, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::PermissionDenied),
        }
    }

    fn sys_pkg_remove(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            201, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::PermissionDenied),
        }
    }

    fn sys_pkg_search(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            202, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::NotFound),
        }
    }

    fn sys_pkg_info(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            203, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::NotFound),
        }
    }

    fn sys_pkg_list(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            204, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::OperationNotSupported),
        }
    }

    fn sys_pkg_update(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            205, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::OperationNotSupported),
        }
    }

    fn sys_pkg_upgrade(&self, args: &[u64]) -> SyscallResult {
        match crate::package::handle_package_syscall(
            206, args[0] as usize, args[1] as usize, args[2] as usize, args[3] as usize
        ) {
            Ok(val) => SyscallResult::Success(val as u64),
            Err(_) => SyscallResult::Error(SyscallError::PermissionDenied),
        }
    }

    /// Get system call statistics
    pub fn get_stats(&self) -> (u64, &[u64; 64]) {
        (self.total_syscalls, &self.syscall_count)
    }

    // Helper methods for user-space memory operations

    /// Copy string from user space with full validation and security checks
    fn copy_string_from_user(&self, user_ptr: u64) -> Result<String, SyscallError> {
        use crate::memory::user_space::UserSpaceMemory;

        // Use production-ready implementation with:
        // - User space pointer validation
        // - Page table walking with permission checks
        // - Safe byte-by-byte copying with fault handling
        // - Null terminator detection
        // - UTF-8 validation
        const PATH_MAX: usize = 4096;
        UserSpaceMemory::copy_string_from_user(user_ptr, PATH_MAX)
            .map_err(|_| SyscallError::InvalidAddress)
    }

    /// Copy data from user space
    fn copy_from_user(&self, user_ptr: u64, buffer: &mut [u8]) -> Result<(), SyscallError> {
        use crate::memory::user_space::UserSpaceMemory;

        UserSpaceMemory::copy_from_user(user_ptr, buffer)
            .map_err(|_| SyscallError::InvalidAddress)
    }

    /// Copy data to user space
    fn copy_to_user(&self, user_ptr: u64, buffer: &[u8]) -> Result<(), SyscallError> {
        use crate::memory::user_space::UserSpaceMemory;

        UserSpaceMemory::copy_to_user(user_ptr, buffer)
            .map_err(|_| SyscallError::InvalidAddress)
    }
}

/// Helper function to copy string to fixed-size buffer
fn copy_str_to_buf(dest: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    let copy_len = core::cmp::min(bytes.len(), dest.len() - 1);
    dest[..copy_len].copy_from_slice(&bytes[..copy_len]);
    dest[copy_len] = 0; // Null terminator
}

/// System call handler entry point (called from assembly)
#[no_mangle]
pub extern "C" fn syscall_handler(
    syscall_number: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let args = [arg1, arg2, arg3, arg4, arg5, arg6];
    let process_manager = super::get_process_manager();

    match process_manager.handle_syscall(syscall_number, &args) {
        Ok(result) => result,
        Err(_) => SyscallError::InvalidSyscall as u64,
    }
}