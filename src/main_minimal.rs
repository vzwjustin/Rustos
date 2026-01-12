//! Minimal RustOS Kernel
//!
//! A bare-bones bootable kernel demonstrating core functionality without complex dependencies.
//! This kernel boots, prints to both serial and VGA, then halts cleanly.

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};

// ============================================================================
// Compiler Intrinsics (Required for bare-metal)
// ============================================================================

use core::ffi::c_void;

/// Memory copy implementation
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let dest_bytes = dest as *mut u8;
    let src_bytes = src as *const u8;
    for i in 0..n {
        *dest_bytes.add(i) = *src_bytes.add(i);
    }
    dest
}

/// Memory set implementation
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let bytes = s as *mut u8;
    let byte_val = c as u8;
    for i in 0..n {
        *bytes.add(i) = byte_val;
    }
    s
}

/// Memory compare implementation
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    let bytes1 = s1 as *const u8;
    let bytes2 = s2 as *const u8;
    for i in 0..n {
        let b1 = *bytes1.add(i);
        let b2 = *bytes2.add(i);
        if b1 < b2 {
            return -1;
        } else if b1 > b2 {
            return 1;
        }
    }
    0
}

/// Memory move implementation (handles overlapping regions)
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let dest_bytes = dest as *mut u8;
    let src_bytes = src as *const u8;
    if (dest_bytes as usize) < (src_bytes as usize) {
        // Copy forward
        for i in 0..n {
            *dest_bytes.add(i) = *src_bytes.add(i);
        }
    } else {
        // Copy backward to handle overlap
        for i in (0..n).rev() {
            *dest_bytes.add(i) = *src_bytes.add(i);
        }
    }
    dest
}

// ============================================================================
// Serial Port Driver (COM1)
// ============================================================================

/// Initialize serial port COM1 (0x3F8)
unsafe fn init_serial() {
    let port = 0x3f8; // COM1
    // Disable interrupts
    outb(port + 1, 0x00);
    // Enable DLAB (set baud rate divisor)
    outb(port + 3, 0x80);
    // Set divisor to 3 (38400 baud)
    outb(port + 0, 0x03);
    outb(port + 1, 0x00);
    // 8 bits, no parity, one stop bit
    outb(port + 3, 0x03);
    // Enable FIFO, clear them, with 14-byte threshold
    outb(port + 2, 0xc7);
    // Enable interrupts, set RTS/DSR
    outb(port + 4, 0x0b);
}

/// Write a byte to I/O port
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nostack, preserves_flags)
    );
}

/// Read a byte from I/O port
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nostack, preserves_flags)
    );
    value
}

/// Write a single byte to serial port
unsafe fn serial_write_byte(byte: u8) {
    let port = 0x3f8;
    // Wait for transmit to be ready (bit 5 of line status)
    while (inb(port + 5) & 0x20) == 0 {}
    outb(port, byte);
}

/// Write a string to serial port
unsafe fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        serial_write_byte(byte);
    }
}

// ============================================================================
// VGA Text Buffer (Direct Access)
// ============================================================================

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

/// VGA color codes
#[allow(dead_code)]
#[repr(u8)]
enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// Create VGA color attribute byte
const fn vga_color(fg: VgaColor, bg: VgaColor) -> u8 {
    (bg as u8) << 4 | (fg as u8)
}

/// Clear VGA screen
unsafe fn vga_clear() {
    let color = vga_color(VgaColor::White, VgaColor::Black);
    for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
        *VGA_BUFFER.add(i * 2) = b' ';
        *VGA_BUFFER.add(i * 2 + 1) = color;
    }
}

/// Write string to VGA at specific position
unsafe fn vga_write_at(x: usize, y: usize, s: &str, color: u8) {
    if y >= VGA_HEIGHT {
        return;
    }
    let offset = (y * VGA_WIDTH + x) * 2;
    for (i, byte) in s.bytes().enumerate() {
        if x + i >= VGA_WIDTH {
            break;
        }
        *VGA_BUFFER.add(offset + i * 2) = byte;
        *VGA_BUFFER.add(offset + i * 2 + 1) = color;
    }
}

// ============================================================================
// Kernel Entry Point
// ============================================================================

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    unsafe {
        // Initialize serial port for debugging output
        init_serial();
        serial_write_str("RustOS Minimal Kernel starting...\r\n");

        // Clear VGA screen
        vga_clear();
        serial_write_str("VGA buffer cleared\r\n");

        // Display boot banner
        let color_title = vga_color(VgaColor::Yellow, VgaColor::Blue);
        let color_normal = vga_color(VgaColor::White, VgaColor::Black);
        let color_success = vga_color(VgaColor::LightGreen, VgaColor::Black);
        let color_info = vga_color(VgaColor::LightCyan, VgaColor::Black);

        // Title bar
        vga_write_at(0, 0, "                                RustOS Minimal Kernel                                ", color_title);

        // Main message
        vga_write_at(25, 5, "RustOS Minimal Kernel Alive!", color_success);

        // Status information
        vga_write_at(10, 8, "Status:", color_normal);
        vga_write_at(18, 8, "RUNNING", color_success);

        vga_write_at(10, 10, "Architecture:", color_normal);
        vga_write_at(24, 10, "x86_64", color_info);

        vga_write_at(10, 11, "Boot Method:", color_normal);
        vga_write_at(24, 11, "Multiboot2 via bootloader crate", color_info);

        vga_write_at(10, 13, "Serial Output:", color_normal);
        vga_write_at(25, 13, "COM1 (0x3F8) - 38400 baud", color_success);

        vga_write_at(10, 14, "Video Output:", color_normal);
        vga_write_at(25, 14, "VGA Text Mode (80x25)", color_success);

        vga_write_at(10, 16, "Kernel Features:", color_normal);
        vga_write_at(12, 17, "- Bootloader integration", color_info);
        vga_write_at(12, 18, "- Serial port driver (COM1)", color_info);
        vga_write_at(12, 19, "- VGA text mode output", color_info);
        vga_write_at(12, 20, "- Panic handler with diagnostics", color_info);
        vga_write_at(12, 21, "- CPU halt loop", color_info);

        // Footer
        vga_write_at(20, 23, "Kernel is now idle - entering halt loop", color_normal);

        serial_write_str("RustOS Minimal Kernel Alive!\r\n");
        serial_write_str("Features initialized:\r\n");
        serial_write_str("  - Serial output (COM1)\r\n");
        serial_write_str("  - VGA text mode\r\n");
        serial_write_str("  - Panic handler\r\n");
        serial_write_str("Entering idle loop...\r\n");
    }

    // Halt loop - continuously halt the CPU
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, preserves_flags));
        }
    }
}

// ============================================================================
// Panic Handler
// ============================================================================

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        // Write to serial port
        serial_write_str("\r\n!!! KERNEL PANIC !!!\r\n");

        // Display panic location
        if let Some(location) = info.location() {
            serial_write_str("Location: ");
            serial_write_str(location.file());
            serial_write_str(":");
            // Note: Can't easily format numbers without alloc, so skip line/column
            serial_write_str("\r\n");
        }

        // Display panic message
        serial_write_str("Message: ");
        // Note: Can't easily format PanicMessage without alloc
        serial_write_str("<panic occurred>\r\n");

        // Display on VGA screen
        let color_error = vga_color(VgaColor::White, VgaColor::Red);
        let color_text = vga_color(VgaColor::LightRed, VgaColor::Black);

        vga_write_at(0, 12, "                                  KERNEL PANIC!                                  ", color_error);
        vga_write_at(10, 14, "The kernel has encountered a fatal error and must halt.", color_text);

        if let Some(location) = info.location() {
            vga_write_at(10, 16, "Location: ", color_text);
            vga_write_at(20, 16, location.file(), color_text);
        }

        vga_write_at(10, 18, "System halted. Please reboot.", color_text);

        serial_write_str("System halted.\r\n");
    }

    // Halt forever
    loop {
        unsafe {
            core::arch::asm!("cli", options(nostack, preserves_flags));
            core::arch::asm!("hlt", options(nostack, preserves_flags));
        }
    }
}
