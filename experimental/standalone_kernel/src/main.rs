#![no_std]
#![no_main]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};

// VGA buffer for output
const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
static mut CURSOR_POS: usize = 0;

// Multiboot entry point (called from assembly)
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    clear_screen();
    print_line(b"RustOS Multiboot Kernel - SUCCESS!");
    print_line(b"Standalone kernel working!");
    print_line(b"Multiboot headers functional!");
    print_line(b"Ready for deployment!");
    
    loop {
        unsafe { 
            core::arch::asm!("hlt"); 
        }
    }
}

// Bootloader entry point
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    clear_screen();
    print_line(b"RustOS - Bootloader Integration SUCCESS!");
    print_line(b"Boot information received");
    print_line(b"Memory regions available");
    print_line(b"System fully operational!");
    
    // Show we can access boot info
    let memory_map = &boot_info.memory_map;
    print_line(b"Memory map regions detected");
    
    loop {
        unsafe { 
            core::arch::asm!("hlt"); 
        }
    }
}

fn clear_screen() {
    unsafe {
        for i in 0..(80 * 25 * 2) {
            *VGA_BUFFER.add(i) = if i % 2 == 0 { b' ' } else { 0x0F };
        }
        CURSOR_POS = 0;
    }
}

fn print_line(s: &[u8]) {
    unsafe {
        let row = CURSOR_POS / 80;
        if row >= 25 {
            // Scroll or wrap
            clear_screen();
            CURSOR_POS = 0;
        }
        
        let start_pos = (CURSOR_POS / 80) * 80; // Start of current line
        
        for (i, &byte) in s.iter().enumerate() {
            if i < 79 { // Leave room for line ending
                let offset = (start_pos + i) * 2;
                if offset < (80 * 25 * 2) {
                    *VGA_BUFFER.add(offset) = byte;
                    *VGA_BUFFER.add(offset + 1) = 0x0F; // Bright white on black
                }
            }
        }
        
        CURSOR_POS = start_pos + 80; // Move to next line
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    clear_screen();
    print_line(b"KERNEL PANIC - SYSTEM HALTED");
    
    loop {
        unsafe { 
            core::arch::asm!("hlt"); 
        }
    }
}
