//! Syscall Context Switching
//!
//! This module provides low-level context switching for syscalls, handling
//! the transition between user mode (Ring 3) and kernel mode (Ring 0).
//!
//! # Architecture
//!
//! When a user program makes a syscall (via INT 0x80 or SYSCALL instruction),
//! the CPU automatically:
//! 1. Switches from CPL=3 to CPL=0
//! 2. Loads kernel stack from TSS.RSP0
//! 3. Pushes user state (SS, RSP, RFLAGS, CS, RIP) to kernel stack
//! 4. Jumps to syscall handler
//!
//! This module handles:
//! - Saving complete user register state
//! - Providing safe access to syscall arguments
//! - Restoring user state on return
//! - Handling syscall errors
//!
//! # Safety
//!
//! All context switching operations are highly sensitive and must maintain:
//! - Stack pointer validity
//! - Segment selector correctness
//! - Interrupt enable state
//! - Register preservation

use core::arch::asm;
use x86_64::VirtAddr;

/// Complete user mode register state
///
/// This structure captures all general-purpose registers and segment
/// registers that need to be preserved across syscalls.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserContext {
    // General purpose registers (callee-saved + argument registers)
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Instruction pointer and flags
    pub rip: u64,
    pub rflags: u64,

    // Segment selectors
    pub cs: u16,
    pub ss: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,

    // FS and GS bases (for thread-local storage)
    pub fs_base: u64,
    pub gs_base: u64,
}

impl UserContext {
    /// Create a new empty user context
    pub const fn new() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0,
            rflags: 0x202, // Interrupts enabled, reserved bit 1
            cs: 0, ss: 0, ds: 0, es: 0, fs: 0, gs: 0,
            fs_base: 0, gs_base: 0,
        }
    }

    /// Create a user context for starting a new program
    ///
    /// # Arguments
    /// * `entry_point` - Program entry point
    /// * `stack_pointer` - Top of user stack
    pub fn for_new_program(entry_point: VirtAddr, stack_pointer: VirtAddr) -> Self {
        let user_cs = crate::gdt::get_user_code_selector();
        let user_ds = crate::gdt::get_user_data_selector();

        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            rsp: stack_pointer.as_u64(),
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: entry_point.as_u64(),
            rflags: 0x202, // IF=1, IOPL=0
            cs: user_cs.0,
            ss: user_ds.0,
            ds: user_ds.0,
            es: user_ds.0,
            fs: user_ds.0,
            gs: user_ds.0,
            fs_base: 0,
            gs_base: 0,
        }
    }

    /// Save current CPU register state
    ///
    /// # Safety
    /// This function reads CPU registers and should only be called in kernel mode
    pub unsafe fn save_current() -> Self {
        let mut ctx = Self::new();

        asm!(
            "mov {rax}, rax",
            "mov {rbx}, rbx",
            "mov {rcx}, rcx",
            "mov {rdx}, rdx",
            "mov {rsi}, rsi",
            "mov {rdi}, rdi",
            "mov {rbp}, rbp",
            "mov {rsp}, rsp",
            "mov {r8}, r8",
            "mov {r9}, r9",
            "mov {r10}, r10",
            "mov {r11}, r11",
            "mov {r12}, r12",
            "mov {r13}, r13",
            "mov {r14}, r14",
            "mov {r15}, r15",
            rax = out(reg) ctx.rax,
            rbx = out(reg) ctx.rbx,
            rcx = out(reg) ctx.rcx,
            rdx = out(reg) ctx.rdx,
            rsi = out(reg) ctx.rsi,
            rdi = out(reg) ctx.rdi,
            rbp = out(reg) ctx.rbp,
            rsp = out(reg) ctx.rsp,
            r8 = out(reg) ctx.r8,
            r9 = out(reg) ctx.r9,
            r10 = out(reg) ctx.r10,
            r11 = out(reg) ctx.r11,
            r12 = out(reg) ctx.r12,
            r13 = out(reg) ctx.r13,
            r14 = out(reg) ctx.r14,
            r15 = out(reg) ctx.r15,
            options(nostack, preserves_flags)
        );

        // Read segment selectors
        asm!(
            "mov {0:x}, cs",
            "mov {1:x}, ss",
            "mov {2:x}, ds",
            "mov {3:x}, es",
            "mov {4:x}, fs",
            "mov {5:x}, gs",
            out(reg) ctx.cs,
            out(reg) ctx.ss,
            out(reg) ctx.ds,
            out(reg) ctx.es,
            out(reg) ctx.fs,
            out(reg) ctx.gs,
            options(nostack, preserves_flags)
        );

        // Read RFLAGS
        asm!(
            "pushfq",
            "pop {rflags}",
            rflags = out(reg) ctx.rflags,
            options(preserves_flags)
        );

        ctx
    }

    /// Restore this context to the CPU
    ///
    /// # Safety
    /// This function modifies CPU state and must only be called when
    /// switching back to the context this was saved from
    pub unsafe fn restore(&self) {
        // Restore general-purpose registers
        asm!(
            "mov rax, {rax}",
            "mov rbx, {rbx}",
            "mov rcx, {rcx}",
            "mov rdx, {rdx}",
            "mov rsi, {rsi}",
            "mov rdi, {rdi}",
            "mov rbp, {rbp}",
            "mov r8, {r8}",
            "mov r9, {r9}",
            "mov r10, {r10}",
            "mov r11, {r11}",
            "mov r12, {r12}",
            "mov r13, {r13}",
            "mov r14, {r14}",
            "mov r15, {r15}",
            rax = in(reg) self.rax,
            rbx = in(reg) self.rbx,
            rcx = in(reg) self.rcx,
            rdx = in(reg) self.rdx,
            rsi = in(reg) self.rsi,
            rdi = in(reg) self.rdi,
            rbp = in(reg) self.rbp,
            r8 = in(reg) self.r8,
            r9 = in(reg) self.r9,
            r10 = in(reg) self.r10,
            r11 = in(reg) self.r11,
            r12 = in(reg) self.r12,
            r13 = in(reg) self.r13,
            r14 = in(reg) self.r14,
            r15 = in(reg) self.r15,
            options(nostack, preserves_flags)
        );

        // Restore data segment selectors
        asm!(
            "mov ds, {ds:x}",
            "mov es, {es:x}",
            "mov fs, {fs:x}",
            "mov gs, {gs:x}",
            ds = in(reg) self.ds,
            es = in(reg) self.es,
            fs = in(reg) self.fs,
            gs = in(reg) self.gs,
            options(nostack, preserves_flags)
        );
    }
}

impl Default for UserContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Syscall arguments extracted from registers
///
/// Following the System V AMD64 ABI for syscalls:
/// - RAX: syscall number
/// - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub syscall_num: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
    pub arg6: u64,
}

impl SyscallArgs {
    /// Extract syscall arguments from user context
    pub fn from_context(ctx: &UserContext) -> Self {
        Self {
            syscall_num: ctx.rax,
            arg1: ctx.rdi,
            arg2: ctx.rsi,
            arg3: ctx.rdx,
            arg4: ctx.r10, // Note: R10 is used instead of RCX for syscalls
            arg5: ctx.r8,
            arg6: ctx.r9,
        }
    }

    /// Extract from current CPU state (during syscall handler)
    ///
    /// # Safety
    /// Must be called from syscall handler with user registers intact
    pub unsafe fn extract_from_registers() -> Self {
        let mut syscall_num: u64;
        let mut arg1: u64;
        let mut arg2: u64;
        let mut arg3: u64;
        let mut arg4: u64;
        let mut arg5: u64;
        let mut arg6: u64;

        asm!(
            "mov {syscall_num}, rax",
            "mov {arg1}, rdi",
            "mov {arg2}, rsi",
            "mov {arg3}, rdx",
            "mov {arg4}, r10",
            "mov {arg5}, r8",
            "mov {arg6}, r9",
            syscall_num = out(reg) syscall_num,
            arg1 = out(reg) arg1,
            arg2 = out(reg) arg2,
            arg3 = out(reg) arg3,
            arg4 = out(reg) arg4,
            arg5 = out(reg) arg5,
            arg6 = out(reg) arg6,
            options(nostack, preserves_flags)
        );

        Self {
            syscall_num,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
            arg6,
        }
    }
}

/// Syscall handler entry point
///
/// This is called by the interrupt handler (INT 0x80) and manages the
/// complete syscall lifecycle:
/// 1. Save user context
/// 2. Extract syscall arguments
/// 3. Validate arguments
/// 4. Execute syscall handler
/// 5. Return result
/// 6. Restore user context
///
/// # Safety
/// This function must only be called from the INT 0x80 interrupt handler
pub unsafe fn handle_syscall_entry() -> ! {
    // Extract syscall arguments from registers
    let args = SyscallArgs::extract_from_registers();

    // Validate we came from user mode
    if !crate::usermode::in_user_mode() {
        // Syscall from kernel mode is invalid
        crate::serial_println!("WARNING: Syscall from kernel mode!");
        return_syscall_error(-38); // -ENOSYS
    }

    // Get current process
    let process_manager = crate::process::get_process_manager();
    let current_pid = process_manager.current_process();

    if current_pid == 0 {
        // No current process - this should not happen
        crate::serial_println!("ERROR: Syscall with no current process!");
        return_syscall_error(-1); // -EPERM
    }

    // Dispatch syscall
    let result = crate::syscall_handler::dispatch_syscall(
        args.syscall_num,
        args.arg1,
        args.arg2,
        args.arg3,
        args.arg4,
        args.arg5,
        args.arg6,
    );

    // Return result to user mode
    return_to_usermode_with_result(result);
}

/// Return from syscall with result value
///
/// # Safety
/// This function must only be called at the end of syscall handling
unsafe fn return_to_usermode_with_result(result: i64) -> ! {
    // Set RAX to syscall result
    asm!(
        "mov rax, {result}",
        result = in(reg) result,
        options(nostack, preserves_flags)
    );

    // The iretq instruction will restore user mode state
    // The interrupt handler pushed: SS, RSP, RFLAGS, CS, RIP
    // iretq will pop them and return to user mode
    asm!(
        "iretq",
        options(noreturn)
    );
}

/// Return syscall error
///
/// # Safety
/// This function must only be called during syscall error handling
unsafe fn return_syscall_error(error: i64) -> ! {
    return_to_usermode_with_result(error);
}

/// Save user context before executing syscall
///
/// This should be called at the beginning of syscall handling to preserve
/// user state that might be modified during syscall execution.
pub fn save_user_context_for_syscall(pid: crate::process::Pid) -> Result<(), &'static str> {
    unsafe {
        let ctx = UserContext::save_current();

        // Store context in process control block
        let process_manager = crate::process::get_process_manager();
        if let Some(mut pcb) = process_manager.get_process(pid) {
            // Update context with current state
            pcb.context.rax = ctx.rax;
            pcb.context.rbx = ctx.rbx;
            pcb.context.rcx = ctx.rcx;
            pcb.context.rdx = ctx.rdx;
            pcb.context.rsi = ctx.rsi;
            pcb.context.rdi = ctx.rdi;
            pcb.context.rbp = ctx.rbp;
            pcb.context.rsp = ctx.rsp;
            pcb.context.r8 = ctx.r8;
            pcb.context.r9 = ctx.r9;
            pcb.context.r10 = ctx.r10;
            pcb.context.r11 = ctx.r11;
            pcb.context.r12 = ctx.r12;
            pcb.context.r13 = ctx.r13;
            pcb.context.r14 = ctx.r14;
            pcb.context.r15 = ctx.r15;
            pcb.context.rip = ctx.rip;
            pcb.context.rflags = ctx.rflags;

            Ok(())
        } else {
            Err("Process not found")
        }
    }
}

/// Restore user context after syscall completion
pub fn restore_user_context_after_syscall(pid: crate::process::Pid) -> Result<(), &'static str> {
    let process_manager = crate::process::get_process_manager();

    if let Some(pcb) = process_manager.get_process(pid) {
        let ctx = UserContext {
            rax: pcb.context.rax,
            rbx: pcb.context.rbx,
            rcx: pcb.context.rcx,
            rdx: pcb.context.rdx,
            rsi: pcb.context.rsi,
            rdi: pcb.context.rdi,
            rbp: pcb.context.rbp,
            rsp: pcb.context.rsp,
            r8: pcb.context.r8,
            r9: pcb.context.r9,
            r10: pcb.context.r10,
            r11: pcb.context.r11,
            r12: pcb.context.r12,
            r13: pcb.context.r13,
            r14: pcb.context.r14,
            r15: pcb.context.r15,
            rip: pcb.context.rip,
            rflags: pcb.context.rflags,
            cs: pcb.context.cs,
            ss: pcb.context.ss,
            ds: pcb.context.ds,
            es: pcb.context.es,
            fs: pcb.context.fs,
            gs: pcb.context.gs,
            fs_base: 0,
            gs_base: 0,
        };

        unsafe {
            ctx.restore();
        }

        Ok(())
    } else {
        Err("Process not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_creation() {
        let ctx = UserContext::new();
        assert_eq!(ctx.rax, 0);
        assert_eq!(ctx.rsp, 0);
        assert_eq!(ctx.rflags, 0x202);
    }

    #[test]
    fn test_user_context_for_new_program() {
        let entry = VirtAddr::new(0x400000);
        let stack = VirtAddr::new(0x7FFFFFFF0);

        let ctx = UserContext::for_new_program(entry, stack);

        assert_eq!(ctx.rip, 0x400000);
        assert_eq!(ctx.rsp, 0x7FFFFFFF0);
        assert_eq!(ctx.rflags, 0x202);
    }

    #[test]
    fn test_syscall_args_size() {
        use core::mem::size_of;

        // Verify struct sizes for ABI compatibility
        assert_eq!(size_of::<SyscallArgs>(), 7 * 8); // 7 u64 fields
    }
}
