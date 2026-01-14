//! Enhanced Debugging Utilities for RustOS
//!
//! This module provides comprehensive debugging tools including:
//! - Enhanced assertion macros with detailed error reporting
//! - Memory dump utilities for inspecting raw memory
//! - Register dump utilities for CPU state inspection
//! - Function entry/exit tracing macros
//! - Debug helpers for common debugging patterns

use core::fmt;
use alloc::string::String;
use alloc::format;

/// Debug assertion with custom message
#[macro_export]
macro_rules! debug_assert_msg {
    ($cond:expr, $msg:expr) => {
        #[cfg(debug_assertions)]
        {
            if !($cond) {
                log_error!("DEBUG", "Assertion failed: {} at {}:{}", $msg, file!(), line!());
                panic!("Assertion failed: {}", $msg);
            }
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            if !($cond) {
                let msg = format!($fmt, $($arg)*);
                log_error!("DEBUG", "Assertion failed: {} at {}:{}", msg, file!(), line!());
                panic!("Assertion failed: {}", msg);
            }
        }
    };
}

/// Trace function entry with arguments
#[macro_export]
macro_rules! trace_entry {
    ($module:expr) => {
        log_trace!($module, "→ Entering {} at {}:{}",
            core::any::type_name::<Self>(), file!(), line!());
    };
    ($module:expr, $fmt:expr, $($arg:tt)*) => {
        log_trace!($module, "→ Entering {} with args: {} at {}:{}",
            core::any::type_name::<Self>(),
            format!($fmt, $($arg)*),
            file!(), line!());
    };
}

/// Trace function exit with return value
#[macro_export]
macro_rules! trace_exit {
    ($module:expr) => {
        log_trace!($module, "← Exiting {} at {}:{}",
            core::any::type_name::<Self>(), file!(), line!());
    };
    ($module:expr, $fmt:expr, $($arg:tt)*) => {
        log_trace!($module, "← Exiting {} with result: {} at {}:{}",
            core::any::type_name::<Self>(),
            format!($fmt, $($arg)*),
            file!(), line!());
    };
}

/// Debug log with automatic module detection
#[macro_export]
macro_rules! debug {
    ($fmt:expr) => {
        log_debug!(module_path!(), $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        log_debug!(module_path!(), $fmt, $($arg)*);
    };
}

/// Info log with automatic module detection
#[macro_export]
macro_rules! info {
    ($fmt:expr) => {
        log_info!(module_path!(), $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        log_info!(module_path!(), $fmt, $($arg)*);
    };
}

/// Warning log with automatic module detection
#[macro_export]
macro_rules! warn {
    ($fmt:expr) => {
        log_warn!(module_path!(), $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        log_warn!(module_path!(), $fmt, $($arg)*);
    };
}

/// Error log with automatic module detection
#[macro_export]
macro_rules! error {
    ($fmt:expr) => {
        log_error!(module_path!(), $fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        log_error!(module_path!(), $fmt, $($arg)*);
    };
}

/// Memory dump utility
pub struct MemoryDumper;

impl MemoryDumper {
    /// Dump memory region in hexdump format
    pub fn dump(addr: usize, len: usize, label: &str) {
        use crate::serial_println;

        crate::serial_println!("=== Memory Dump: {} ===", label);
        crate::serial_println!("Address: 0x{:016x}, Length: {} bytes", addr, len);

        let ptr = addr as *const u8;
        let mut offset = 0;

        while offset < len {
            crate::serial_print!("{:08x}  ", offset);

            // Hex bytes
            for i in 0..16 {
                if offset + i < len {
                    unsafe {
                        crate::serial_print!("{:02x} ", *ptr.add(offset + i));
                    }
                } else {
                    crate::serial_print!("   ");
                }

                if i == 7 {
                    crate::serial_print!(" ");
                }
            }

            crate::serial_print!(" |");

            // ASCII representation
            for i in 0..16 {
                if offset + i < len {
                    unsafe {
                        let byte = *ptr.add(offset + i);
                        if byte >= 32 && byte <= 126 {
                            crate::serial_print!("{}", byte as char);
                        } else {
                            crate::serial_print!(".");
                        }
                    }
                }
            }

            crate::serial_println!("|");
            offset += 16;
        }

        crate::serial_println!("=== End Memory Dump ===");
    }

    /// Dump stack trace (simplified)
    pub fn dump_stack(label: &str) {
        use crate::serial_println;

        crate::serial_println!("=== Stack Trace: {} ===", label);

        let rbp: usize;
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }

        crate::serial_println!("RBP: 0x{:016x}", rbp);

        // Walk stack frames (simplified - could be enhanced with debug symbols)
        let mut frame_ptr = rbp as *const usize;
        let mut depth = 0;
        const MAX_ADDR: usize = 0x7fffffffffff;

        while depth < 16 && !frame_ptr.is_null() {
            unsafe {
                let addr = frame_ptr as usize;
                if addr > 0x1000 && addr < MAX_ADDR {
                    let return_addr = *frame_ptr.add(1);
                    crate::serial_println!("  Frame {}: RIP = 0x{:016x}", depth, return_addr);
                    frame_ptr = *frame_ptr as *const usize;
                    depth += 1;
                } else {
                    break;
                }
            }
        }

        crate::serial_println!("=== End Stack Trace ===");
    }
}

/// CPU Register dumper
pub struct RegisterDumper;

impl RegisterDumper {
    /// Dump general purpose registers
    pub fn dump_gpr(label: &str) {
        use crate::serial_println;

        crate::serial_println!("=== CPU Registers: {} ===", label);

        let rax: u64;
        let rbx: u64;
        let rcx: u64;
        let rdx: u64;
        let rsi: u64;
        let rdi: u64;
        let rbp: u64;
        let rsp: u64;

        unsafe {
            core::arch::asm!(
                "mov {}, rax",
                "mov {}, rbx",
                "mov {}, rcx",
                "mov {}, rdx",
                "mov {}, rsi",
                "mov {}, rdi",
                "mov {}, rbp",
                "mov {}, rsp",
                out(reg) rax,
                out(reg) rbx,
                out(reg) rcx,
                out(reg) rdx,
                out(reg) rsi,
                out(reg) rdi,
                out(reg) rbp,
                out(reg) rsp,
            );
        }

        crate::serial_println!("RAX: 0x{:016x}  RBX: 0x{:016x}", rax, rbx);
        crate::serial_println!("RCX: 0x{:016x}  RDX: 0x{:016x}", rcx, rdx);
        crate::serial_println!("RSI: 0x{:016x}  RDI: 0x{:016x}", rsi, rdi);
        crate::serial_println!("RBP: 0x{:016x}  RSP: 0x{:016x}", rbp, rsp);

        crate::serial_println!("=== End CPU Registers ===");
    }

    /// Dump control registers
    pub fn dump_control_regs(label: &str) {
        use crate::serial_println;

        crate::serial_println!("=== Control Registers: {} ===", label);

        let cr0: u64;
        let cr2: u64;
        let cr3: u64;
        let cr4: u64;

        unsafe {
            core::arch::asm!(
                "mov {}, cr0",
                "mov {}, cr2",
                "mov {}, cr3",
                "mov {}, cr4",
                out(reg) cr0,
                out(reg) cr2,
                out(reg) cr3,
                out(reg) cr4,
            );
        }

        crate::serial_println!("CR0: 0x{:016x}  CR2: 0x{:016x}", cr0, cr2);
        crate::serial_println!("CR3: 0x{:016x}  CR4: 0x{:016x}", cr3, cr4);

        crate::serial_println!("=== End Control Registers ===");
    }
}

/// Performance measurement helper
pub struct PerfMeasure {
    start_tsc: u64,
    label: &'static str,
}

impl PerfMeasure {
    /// Start performance measurement
    pub fn start(label: &'static str) -> Self {
        let start_tsc = unsafe {
            let mut low: u32;
            let mut high: u32;
            core::arch::asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
            );
            ((high as u64) << 32) | (low as u64)
        };

        Self { start_tsc, label }
    }
}

impl Drop for PerfMeasure {
    fn drop(&mut self) {
        let end_tsc = unsafe {
            let mut low: u32;
            let mut high: u32;
            core::arch::asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
            );
            ((high as u64) << 32) | (low as u64)
        };

        let cycles = end_tsc - self.start_tsc;
        crate::log_debug!("PERF", "{}: {} cycles", self.label, cycles);
    }
}

/// Debug breakpoint helper
#[macro_export]
macro_rules! debug_breakpoint {
    () => {
        #[cfg(debug_assertions)]
        {
            log_warn!("DEBUG", "Breakpoint at {}:{}", file!(), line!());
            unsafe { core::arch::asm!("int3"); }
        }
    };
}

/// Conditional debug breakpoint
#[macro_export]
macro_rules! debug_breakpoint_if {
    ($cond:expr) => {
        #[cfg(debug_assertions)]
        {
            if $cond {
                log_warn!("DEBUG", "Conditional breakpoint triggered at {}:{}", file!(), line!());
                unsafe { core::arch::asm!("int3"); }
            }
        }
    };
}

/// Debug probe - log variable value and continue
#[macro_export]
macro_rules! debug_probe {
    ($var:expr) => {
        log_debug!("PROBE", "{} = {:?} at {}:{}", stringify!($var), $var, file!(), line!());
    };
}

/// Hexdump macro for easy memory inspection
#[macro_export]
macro_rules! hexdump {
    ($addr:expr, $len:expr, $label:expr) => {
        $crate::debug_utils::MemoryDumper::dump($addr as usize, $len, $label);
    };
}

/// Stack trace macro
#[macro_export]
macro_rules! stacktrace {
    ($label:expr) => {
        $crate::debug_utils::MemoryDumper::dump_stack($label);
    };
}
