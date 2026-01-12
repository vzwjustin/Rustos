#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};

// Include compiler intrinsics for missing symbols
mod intrinsics;

// Include VGA buffer module for better output
mod vga_buffer;
// Include print module for print! and println! macros
mod print;
// Include basic memory management
mod memory_basic;
// Include full memory management
mod memory;
// Include filesystem
mod fs;
// Include visual boot display
mod boot_display;
// Include enhanced boot UI with progress indicators
mod boot_ui;
// Include keyboard input handler
mod keyboard;
// Include desktop environment
mod simple_desktop;
// Include graphics system
mod graphics;
// Include GPU support
mod gpu;
// Include data structures
mod data_structures;
// Include advanced desktop environment
mod desktop;
// Include serial port driver
mod serial;
// Include time management system
mod time;
// Include GDT (Global Descriptor Table)
mod gdt;
// Include interrupt handling
mod interrupts;
// Include ACPI support
mod acpi;
// Include APIC support
mod apic;
// Include architecture-specific code
mod arch;
// Include SMP (multiprocessor) support
mod smp;
// Include PCI bus support
mod pci;
// Include drivers
mod drivers;
// Include network stack
pub mod net;
// Re-export network module with alternative name for compatibility
pub use net as network;
// Include security
mod security;
// Include IPC
mod ipc;
// Include kernel core
mod kernel;
// Include process management
mod process;
// Include process manager (high-level process APIs)
mod process_manager;
// Include scheduler
mod scheduler;
// Include error handling and recovery system
mod error;
// Include system health monitoring
mod health;
// Include comprehensive logging and debugging
mod logging;
// Include comprehensive testing framework
mod testing;
// Include testing framework core (used by testing module)
mod testing_framework;
// Include I/O optimization and scheduling system
mod io_optimized;
// Include performance monitoring
mod performance;
mod performance_monitor;
// Include experimental package management system
mod package;
// Include Linux API compatibility layer
mod linux_compat;
// Include Linux integration layer
mod linux_integration;
// Include memory manager for virtual memory management
mod memory_manager;
// Include VFS and initramfs for Linux userspace
mod vfs;
mod initramfs;
// Include ELF loader for binary execution
mod elf_loader;
// Include syscall system
mod syscall;
// Include syscall handler for INT 0x80
mod syscall_handler;
// Include fast syscall support (SYSCALL/SYSRET)
mod syscall_fast;
// Include usermode helper module
mod usermode;
// Include usermode testing module
mod usermode_test;

// VGA_WRITER is now used via macros in print module

// Print macros
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

entry_point!(kernel_main);

// Early serial output functions for debugging
unsafe fn init_early_serial() {
    let port = 0x3f8; // COM1
    // Disable interrupts
    outb(port + 1, 0x00);
    // Enable DLAB
    outb(port + 3, 0x80);
    // Set divisor (38400 baud)
    outb(port + 0, 0x03);
    outb(port + 1, 0x00);
    // 8 bits, no parity, one stop bit
    outb(port + 3, 0x03);
    // Enable FIFO
    outb(port + 2, 0xc7);
    // Enable interrupts
    outb(port + 4, 0x0b);
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}

unsafe fn early_serial_write_byte(byte: u8) {
    let port = 0x3f8;
    // Wait for transmit to be ready
    while (inb(port + 5) & 0x20) == 0 {}
    outb(port, byte);
}

unsafe fn early_serial_write_str(s: &str) {
    for byte in s.bytes() {
        early_serial_write_byte(byte);
    }
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize early serial output for debugging
    unsafe {
        init_early_serial();
        early_serial_write_str("RustOS: Kernel entry point reached!\r\n");
    }

    // Write directly to VGA buffer without any initialization to test if kernel is running
    unsafe {
        let vga_buffer = 0xb8000 as *mut u8;
        let message = b"KERNEL STARTED!";
        for (i, &byte) in message.iter().enumerate() {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0x0f; // White on black
        }
        early_serial_write_str("RustOS: VGA buffer initialized\r\n");
    }

    // Initialize VGA buffer for text mode display
    vga_buffer::init();
    unsafe {
        early_serial_write_str("RustOS: VGA buffer system initialized\r\n");
    }

    // Record boot start time (after basic init)
    let boot_start_time = 0u64; // Will use time::uptime_ms() after time init

    // ========================================================================
    // PHASE 1: Boot Splash and Early Initialization
    // ========================================================================
    boot_ui::show_boot_splash();
    boot_ui::boot_delay_medium();

    // ========================================================================
    // PHASE 2: Hardware Detection
    // ========================================================================
    let hardware_result = boot_ui::hardware_detection_progress();

    // ========================================================================
    // PHASE 3: ACPI Initialization
    // ========================================================================
    // Note: bootloader v0.9.33 doesn't provide rsdp_addr or physical_memory_offset
    // We'll use manual ACPI detection and a default physical offset
    let physical_memory_offset = x86_64::VirtAddr::new(0);
    let acpi_result = {
        boot_ui::begin_stage(boot_ui::BootStage::AcpiInit, 1);
        boot_ui::report_warning("ACPI", "Bootloader v0.9.33 doesn't provide RSDP address - using manual detection");
        boot_ui::complete_stage(boot_ui::BootStage::AcpiInit);
        // Try ACPI initialization with manual detection
        boot_ui::acpi_init_progress(None, physical_memory_offset)
    };

    // ========================================================================
    // PHASE 4: PCI Bus Enumeration
    // ========================================================================
    let pci_result = boot_ui::pci_enum_progress();

    // ========================================================================
    // PHASE 5: Memory Management Initialization
    // ========================================================================
    let memory_result = boot_ui::memory_init_progress(&boot_info.memory_map, physical_memory_offset);

    // ========================================================================
    // PHASE 6: Interrupt and System Setup
    // ========================================================================
    boot_ui::begin_stage(boot_ui::BootStage::InterruptInit, 5);

    // Initialize error handling system early
    boot_ui::update_substage(1, "Initializing error handling...");
    error::init_error_handling();
    boot_ui::report_success("Error handling system initialized");

    // Initialize health monitoring system
    boot_ui::update_substage(2, "Starting health monitoring...");
    health::init_health_monitoring();
    boot_ui::report_success("System health monitoring active");

    // Initialize comprehensive logging and debugging
    boot_ui::update_substage(3, "Setting up logging subsystem...");
    logging::init_logging_and_debugging();
    boot_ui::report_success("Logging and debugging ready");

    // Initialize GDT and interrupts
    boot_ui::update_substage(4, "Configuring GDT and IDT...");
    gdt::init();
    interrupts::init();
    boot_ui::report_success("GDT and interrupts configured");

    // Initialize fast syscall support
    boot_ui::update_substage(5, "Setting up syscall interface...");
    if syscall_fast::is_supported() {
        syscall_fast::init();
        boot_ui::report_success("Fast syscall (SYSCALL/SYSRET) enabled");
    } else {
        boot_ui::report_warning("Syscall", "Using INT 0x80 fallback");
    }

    boot_ui::complete_stage(boot_ui::BootStage::InterruptInit);
    boot_ui::boot_delay_short();

    // ========================================================================
    // PHASE 7: Driver Loading
    // ========================================================================
    let driver_result = boot_ui::driver_loading_progress();

    // Initialize time management system (part of driver loading)
    let time_initialized = match time::init() {
        Ok(()) => {
            let stats = time::get_timer_stats();
            log_info!("kernel", "Time system initialized with {:?} timer", stats.active_timer);

            // Initialize system time from RTC
            if let Ok(()) = time::init_system_time_from_rtc() {
                log_info!("kernel", "System time initialized from RTC: {}", time::system_time());
            }
            true
        }
        Err(e) => {
            log_error!("kernel", "Time system initialization failed: {}", e);
            false
        }
    };

    // ========================================================================
    // PHASE 8: File System Mount
    // ========================================================================
    let fs_result = boot_ui::filesystem_mount_progress();

    // Initialize Linux integration layer
    boot_display::show_subsystem_init("Linux Integration Layer", boot_display::SubsystemStatus::Initializing);
    match linux_integration::init() {
        Ok(_) => {
            boot_display::show_subsystem_init("Linux Integration Layer", boot_display::SubsystemStatus::Ready);
            if let Err(e) = crate::kernel::update_subsystem_state("linux_compat", crate::kernel::SubsystemState::Ready) {
                log_warn!("kernel", "Failed to update linux_compat state: {}", e);
            }
            if let Err(e) = crate::kernel::update_subsystem_state("linux_integration", crate::kernel::SubsystemState::Ready) {
                log_warn!("kernel", "Failed to update linux_integration state: {}", e);
            }
        }
        Err(e) => {
            boot_display::show_subsystem_init("Linux Integration Layer", boot_display::SubsystemStatus::Warning);
            log_warn!("kernel", "Linux Integration initialization failed: {}", e);
        }
    }

    // ========================================================================
    // PHASE 9: Graphics Initialization
    // ========================================================================
    let graphics_result = boot_ui::graphics_init_progress();

    // Decide boot mode based on graphics initialization
    let use_graphics_desktop = graphics_result.framebuffer_ready && !graphics_result.fallback_to_text;

    // ========================================================================
    // PHASE 10: Desktop Environment Initialization
    // ========================================================================
    let desktop_result = if use_graphics_desktop {
        boot_ui::desktop_init_progress()
    } else {
        // Skip desktop init for text mode
        boot_ui::begin_stage(boot_ui::BootStage::DesktopInit, 1);
        boot_ui::update_substage(1, "Preparing text-mode desktop...");
        boot_ui::report_warning("Desktop", "Using text-mode interface");
        boot_ui::complete_stage(boot_ui::BootStage::DesktopInit);
        boot_ui::DesktopInitResult::new()
    };

    // ========================================================================
    // Boot Complete Summary
    // ========================================================================
    let boot_time = if time_initialized { time::uptime_ms() } else { 0 };
    boot_ui::boot_complete_summary();
    boot_display::show_boot_complete(boot_time);

    // Show first boot information
    boot_ui::show_first_boot_info(&hardware_result, &memory_result);

    // Brief pause before transitioning to desktop
    boot_ui::boot_delay_medium();

    // ========================================================================
    // Transition to Desktop Environment
    // ========================================================================
    boot_ui::transition_to_desktop();

    unsafe {
        early_serial_write_str("RustOS: Boot sequence complete, entering desktop\r\n");
    }

    // Launch appropriate desktop environment
    if use_graphics_desktop && desktop_result.window_manager_ready {
        println!();
        println!("Launching MODERN GRAPHICS DESKTOP");
        println!("   Resolution: {}x{}", graphics_result.width, graphics_result.height);
        println!("   GPU Acceleration: {}", if graphics_result.gpu_accelerated { "Enabled" } else { "Software" });
        println!();

        // Enter modern desktop main loop
        modern_desktop_main_loop()
    } else {
        // Fall back to text mode desktop
        handle_graphics_fallback();

        println!();
        println!("Launching TEXT MODE DESKTOP");
        println!("   Mode: 80x25 VGA Text");
        println!("   Interface: MS-DOS Style");
        println!();

        // Run demonstrations
        demonstrate_error_handling_and_logging();
        demonstrate_comprehensive_testing();
        demonstrate_package_manager();
        demonstrate_linux_compat();

        simple_desktop::init_desktop();
        desktop_main_loop()
    }
}

/// Handle graphics initialization failure with user options
fn handle_graphics_fallback() {
    let progress = boot_ui::boot_progress();

    if progress.is_safe_mode() {
        boot_display::show_safe_mode_banner();
        return;
    }

    // Show error information
    boot_ui::show_graphics_error("Graphics initialization failed or unsupported hardware");

    println!();
    println!("  Automatically continuing in text mode...");
    boot_ui::boot_delay_medium();
}

/// Demonstrate the new error handling and logging system
fn demonstrate_error_handling_and_logging() {
    println!("🔧 Demonstrating Error Handling and Logging System:");
    
    // Test different log levels
    log_info!("demo", "Testing structured logging system");
    log_debug!("demo", "Debug message with timestamp and location");
    log_warn!("demo", "Warning message example");
    
    // Test performance profiling
    {
        let _timer = logging::profiling::start_measurement("demo_function");
        // Simulate some work
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    } // Timer automatically records when dropped
    
    // Display system diagnostics
    logging::kernel_debug::dump_kernel_state();
    
    // Show health status
    let health_status = health::get_health_status();
    println!("   System Health: {:?}", health_status);
    
    // Validate kernel subsystems
    let validation_result = logging::kernel_debug::validate_kernel_subsystems();
    println!("   Kernel Validation: {}", if validation_result { "PASSED" } else { "FAILED" });
    
    // Show recent logs
    let recent_logs = logging::get_recent_logs();
    println!("   Recent Log Entries: {} stored in memory", recent_logs.len());
    
    println!("✅ Error handling and logging demonstration complete");
    println!();
}

/// Demonstrate the package management system
fn demonstrate_package_manager() {
    println!("📦 Demonstrating Package Management System:");

    // Initialize package manager with Native RustOS package manager
    package::init_package_manager(package::PackageManagerType::Native);
    println!("   ✅ Package manager initialized (Native RustOS mode)");

    // Show supported package formats
    println!("   📋 Supported Package Formats:");
    println!("      • .deb  - Debian/Ubuntu packages (full support)");
    println!("      • .rpm  - Fedora/RHEL packages (validation only)");
    println!("      • .apk  - Alpine Linux packages (validation only)");
    println!("      • .rustos - Native RustOS packages (planned)");

    println!("   🔧 Available Operations:");
    println!("      • Install: syscall(200, name_ptr, name_len)");
    println!("      • Remove: syscall(201, name_ptr, name_len)");
    println!("      • Search: syscall(202, query_ptr, query_len, result_ptr, result_len)");
    println!("      • Info: syscall(203, name_ptr, name_len, result_ptr, result_len)");
    println!("      • List: syscall(204, result_ptr, result_len)");
    println!("      • Update: syscall(205)");
    println!("      • Upgrade: syscall(206, name_ptr, name_len)");

    println!("   📚 Features:");
    println!("      • AR archive parsing (for .deb)");
    println!("      • TAR archive extraction");
    println!("      • GZIP/DEFLATE decompression");
    println!("      • Package metadata parsing");
    println!("      • Dependency tracking");
    println!("      • Package database management");

    println!("   ⚠️  Note: Full installation requires:");
    println!("      • Network stack (for downloads)");
    println!("      • Filesystem support (for file installation)");
    println!("      • Script execution (for postinst/prerm)");

    println!("✅ Package management system demonstration complete");
    println!();
}

/// Demonstrate the Linux compatibility layer
fn demonstrate_linux_compat() {
    println!("🐧 Demonstrating Linux API Compatibility Layer:");

    // Initialize Linux compatibility layer
    linux_compat::init_linux_compat();
    println!("   ✅ Linux compatibility layer initialized");

    // Show supported API categories
    println!("   📋 Supported POSIX/Linux APIs (200+ functions):");
    println!("      • File Operations: fstat, lstat, access, dup, link, chmod, chown, truncate");
    println!("      • Process Control: getuid, setuid, getpgid, setsid, getrusage, prctl");
    println!("      • Time APIs: clock_gettime, nanosleep, timer_create, gettimeofday");
    println!("      • Signal Handling: sigaction, sigprocmask, sigpending, rt_sig*, pause");
    println!("      • Socket Operations: send, recv, setsockopt, poll, epoll, select");
    println!("      • IPC: message queues, semaphores, shared memory, eventfd, timerfd");
    println!("      • Device Control: ioctl, fcntl, flock");
    println!("      • Advanced I/O: pread/pwrite, readv/writev, sendfile, splice, tee");
    println!("      • Extended Attrs: getxattr, setxattr, listxattr, removexattr");
    println!("      • Directory Ops: mkdir, rmdir, getdents64");
    println!("      • Terminal/TTY: tcgetattr, tcsetattr, openpty, isatty, cfsetspeed");
    println!("      • Memory Mgmt: mmap, munmap, mprotect, madvise, mlock, brk, sbrk");
    println!("      • Threading: clone, futex, set_tid_address, robust_list, arch_prctl");
    println!("      • Filesystem: mount, umount, statfs, pivot_root, sync, quotactl");
    println!("      • Resources: getrlimit, setrlimit, prlimit, getpriority, sched_*");
    println!("      • System Info: sysinfo, uname, gethostname, getrandom, syslog");

    // Show statistics
    let stats = linux_compat::get_compat_stats();
    println!("   📊 API Call Statistics:");
    println!("      • File operations: {}", stats.file_ops_count);
    println!("      • Process operations: {}", stats.process_ops_count);
    println!("      • Time operations: {}", stats.time_ops_count);
    println!("      • Signal operations: {}", stats.signal_ops_count);
    println!("      • Socket operations: {}", stats.socket_ops_count);
    println!("      • IPC operations: {}", stats.ipc_ops_count);
    println!("      • Ioctl operations: {}", stats.ioctl_ops_count);
    println!("      • Advanced I/O: {}", stats.advanced_io_count);
    println!("      • TTY operations: {}", stats.tty_ops_count);
    println!("      • Memory operations: {}", stats.memory_ops_count);
    println!("      • Thread operations: {}", stats.thread_ops_count);
    println!("      • Filesystem operations: {}", stats.fs_ops_count);
    println!("      • Resource operations: {}", stats.resource_ops_count);
    println!("      • Sysinfo operations: {}", stats.sysinfo_ops_count);

    println!("   ✨ Linux Compatibility Features:");
    println!("      • POSIX-compliant error codes (errno)");
    println!("      • Linux syscall number compatibility");
    println!("      • struct stat, timespec, sigaction compatibility");
    println!("      • Binary-compatible with Linux applications");

    println!("✅ Linux compatibility layer demonstration complete");
    println!();
}

/// Demonstrate the comprehensive testing system
fn demonstrate_comprehensive_testing() {
    println!("🧪 Demonstrating Comprehensive Testing System:");
    
    // Initialize testing system
    match testing::init_testing_system() {
        Ok(()) => {
            println!("   ✅ Testing framework initialized successfully");
            
            // Run a quick subset of tests for demonstration
            println!("   🔬 Running sample unit tests...");
            let unit_stats = testing::run_test_category("unit");
            println!("      Unit Tests: {}/{} passed", unit_stats.passed, unit_stats.total_tests);
            
            println!("   🔗 Running sample integration tests...");
            let integration_stats = testing::run_test_category("integration");
            println!("      Integration Tests: {}/{} passed", integration_stats.passed, integration_stats.total_tests);
            
            println!("   ⚡ Running sample performance tests...");
            let perf_stats = testing::run_test_category("performance");
            println!("      Performance Tests: {}/{} passed", perf_stats.passed, perf_stats.total_tests);
            
            // Show testing capabilities
            println!("   📊 Available test categories:");
            println!("      • Unit Tests - Core functionality validation");
            println!("      • Integration Tests - System interaction validation");
            println!("      • Stress Tests - High-load system testing");
            println!("      • Performance Tests - Benchmarking and regression detection");
            println!("      • Security Tests - Security vulnerability testing");
            println!("      • Hardware Tests - Real hardware validation");
            
            println!("   🎯 Comprehensive testing ready for production validation");
            
            // Demonstrate production validation capabilities
            println!("   🏭 Production validation features:");
            println!("      • Real hardware configuration testing");
            println!("      • Memory safety validation");
            println!("      • Security audit and vulnerability assessment");
            println!("      • Performance regression detection");
            println!("      • Backward compatibility verification");
            println!("      • System stability under load");
            println!("      • Production readiness scoring");
            
            // Note: Full production validation would be run separately due to time requirements
            println!("   📋 Full production validation available via testing::production_validation::run_production_validation()");
        }
        Err(e) => {
            println!("   ❌ Testing framework initialization failed: {}", e);
        }
    }
    
    println!("✅ Comprehensive testing demonstration complete");
    println!();
}

/// Main desktop loop that handles keyboard input and desktop updates
fn desktop_main_loop() -> ! {
    let mut update_counter: u64 = 0;
    let mut last_time_display = 0u64;

    // Test timer system functionality
    println!("Testing timer system...");
    match time::test_timer_accuracy() {
        Ok(()) => println!("✅ Timer system test completed successfully"),
        Err(e) => println!("❌ Timer system test failed: {}", e),
    }
    
    // Display timer system information
    time::display_time_info();
    
    // Schedule a test timer to demonstrate functionality
    let _timer_id = time::schedule_periodic_timer(5_000_000, || {
        // This callback runs every 5 seconds
        // Note: We can't use println! from interrupt context, but this demonstrates the timer system
    });

    loop {
        // Process keyboard events and forward to desktop
        while let Some(key_event) = keyboard::get_key_event() {
            match key_event {
                keyboard::KeyEvent::CharacterPress(c) => {
                    simple_desktop::with_desktop(|desktop| {
                        desktop.handle_key(c as u8);
                    });
                }
                keyboard::KeyEvent::SpecialPress(special_key) => {
                    // Map special keys to desktop key codes
                    let key_code = match special_key {
                        keyboard::SpecialKey::Escape => 27, // ESC
                        keyboard::SpecialKey::Enter => 13,  // Enter
                        keyboard::SpecialKey::Backspace => 8, // Backspace
                        keyboard::SpecialKey::Tab => 9,     // Tab
                        keyboard::SpecialKey::F1 => 112,   // F1
                        keyboard::SpecialKey::F2 => 113,   // F2
                        keyboard::SpecialKey::F3 => 114,   // F3
                        keyboard::SpecialKey::F4 => 115,   // F4
                        keyboard::SpecialKey::F5 => 116,   // F5
                        _ => continue, // Ignore other special keys for now
                    };

                    simple_desktop::with_desktop(|desktop| {
                        desktop.handle_key(key_code);
                    });
                }
                _ => {
                    // Ignore key releases for now
                }
            }
        }

        // Update desktop periodically (for clock and animations)
        if update_counter.is_multiple_of(1_000_000) {
            simple_desktop::with_desktop(|desktop| {
                desktop.update();
            });
            
            // Display time information every few seconds
            let current_time = time::uptime_ms();
            if current_time > last_time_display + 5000 {
                last_time_display = current_time;
                // Update desktop with current time info
                simple_desktop::with_desktop(|desktop| {
                    // The desktop will show uptime in its status
                });
            }
        }

        update_counter += 1;

        // Halt CPU until next interrupt to save power
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// Modern desktop loop that handles graphics-based desktop
///
/// This is the main event loop for the graphical desktop environment.
/// It handles:
/// - Keyboard input routing to windows
/// - Mouse cursor rendering and movement
/// - Window focus, dragging, and interaction
/// - Periodic desktop updates and rendering
fn modern_desktop_main_loop() -> ! {
    // Desktop state
    let mut update_counter: u64 = 0;
    let mut frame_counter: usize = 0;
    let mut last_render_time: u64 = 0;
    let target_frame_time_ms: u64 = 16; // ~60 FPS target

    // Window interaction state
    let mut _dragging_window: Option<desktop::WindowId> = None;
    let mut _drag_start_x: usize = 0;
    let mut _drag_start_y: usize = 0;
    let mut _window_start_x: usize = 0;
    let mut _window_start_y: usize = 0;

    // Set cursor bounds for input manager
    use drivers::{set_cursor_bounds, get_cursor_position};
    set_cursor_bounds(639, 479); // VGA 640x480

    // Initial render
    desktop::invalidate_desktop();
    desktop::render_desktop();

    // Main event loop
    loop {
        let current_time = time::uptime_ms();

        // ====================================================================
        // Input Processing Phase
        // ====================================================================

        // Process all pending input events from the unified input manager
        while let Some(input_event) = drivers::get_input_event() {
            match input_event {
                drivers::InputEvent::KeyPress(key_event) => {
                    // Handle keyboard press events
                    handle_keyboard_input(key_event);
                }
                drivers::InputEvent::KeyRelease(_key_event) => {
                    // Key release events - could be used for modifier tracking
                }
                drivers::InputEvent::MouseMove { x, y } => {
                    // Real hardware mouse movement
                    desktop::handle_mouse_move(x, y);
                }
                drivers::InputEvent::MouseButtonDown { button, x, y } => {
                    // Convert input manager button to desktop button
                    let desktop_button = match button {
                        drivers::MouseButton::Left => desktop::MouseButton::Left,
                        drivers::MouseButton::Right => desktop::MouseButton::Right,
                        drivers::MouseButton::Middle => desktop::MouseButton::Middle,
                        _ => continue, // Ignore extra buttons for now
                    };
                    desktop::handle_mouse_down(x, y, desktop_button);
                }
                drivers::InputEvent::MouseButtonUp { button, x, y } => {
                    // Convert input manager button to desktop button
                    let desktop_button = match button {
                        drivers::MouseButton::Left => desktop::MouseButton::Left,
                        drivers::MouseButton::Right => desktop::MouseButton::Right,
                        drivers::MouseButton::Middle => desktop::MouseButton::Middle,
                        _ => continue, // Ignore extra buttons for now
                    };
                    desktop::handle_mouse_up(x, y, desktop_button);
                }
                drivers::InputEvent::MouseScroll { delta, x, y } => {
                    // Handle scroll wheel
                    desktop::handle_scroll(x, y, delta as i32);
                }
            }
        }

        // ====================================================================
        // Desktop Update Phase
        // ====================================================================

        // Process pending desktop events
        if update_counter.is_multiple_of(1000) {
            desktop::process_desktop_events();
        }

        // Update desktop state periodically
        if update_counter.is_multiple_of(10_000) {
            desktop::update_desktop();
        }

        // ====================================================================
        // Rendering Phase
        // ====================================================================

        // Render at target frame rate or when needed
        let should_render = desktop::desktop_needs_redraw() ||
                           (current_time >= last_render_time + target_frame_time_ms);

        if should_render {
            // Render the desktop (windows, taskbar, dock)
            desktop::render_desktop();

            // Get current mouse position from input manager
            let (mouse_x, mouse_y) = get_cursor_position();
            let button_state = drivers::input_manager::get_button_states();

            // Render mouse cursor overlay
            render_mouse_cursor(mouse_x, mouse_y, button_state.left);

            // Present the frame
            graphics::framebuffer::present();

            frame_counter += 1;
            last_render_time = current_time;

            // Log frame rate periodically (every 60 frames)
            if frame_counter % 60 == 0 {
                log_debug!("desktop", "Frame {}, uptime {}ms", frame_counter, current_time);
            }
        }

        // ====================================================================
        // System Tasks Phase
        // ====================================================================

        // Periodic system maintenance
        if update_counter.is_multiple_of(1_000_000) {
            // Update system time display (if applicable)
            // Check system health
            // Process deferred operations
        }

        update_counter = update_counter.wrapping_add(1);

        // Halt CPU until next interrupt to save power
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// Handle keyboard input events (unified keyboard handler for modern desktop)
fn handle_keyboard_input(key_event: keyboard::KeyEvent) {
    match key_event {
        keyboard::KeyEvent::CharacterPress(c) => {
            let key_code = c as u8;

            // Forward character input to desktop/window manager
            desktop::handle_key_down(key_code);

            // Log significant keypresses for debugging
            if c == '\x1b' { // ESC
                log_debug!("input", "ESC pressed - could trigger menu");
            }
        }
        keyboard::KeyEvent::SpecialPress(special_key) => {
            // Map special keys to key codes for desktop
            let key_code = match special_key {
                keyboard::SpecialKey::Escape => 27,
                keyboard::SpecialKey::Enter => 13,
                keyboard::SpecialKey::Backspace => 8,
                keyboard::SpecialKey::Tab => 9,
                keyboard::SpecialKey::F1 => 112,  // Help
                keyboard::SpecialKey::F2 => 113,  // Rename
                keyboard::SpecialKey::F3 => 114,  // Search
                keyboard::SpecialKey::F4 => 115,  // Close (Alt+F4)
                keyboard::SpecialKey::F5 => 116,  // Refresh
                keyboard::SpecialKey::F6 => 117,
                keyboard::SpecialKey::F7 => 118,
                keyboard::SpecialKey::F8 => 119,
                keyboard::SpecialKey::F9 => 120,
                keyboard::SpecialKey::F10 => 121,
                keyboard::SpecialKey::F11 => 122, // Fullscreen
                keyboard::SpecialKey::F12 => 123, // Debug console
                keyboard::SpecialKey::Insert => 45,
                keyboard::SpecialKey::Delete => 46,
                keyboard::SpecialKey::Home => 36,
                keyboard::SpecialKey::End => 35,
                keyboard::SpecialKey::PageUp => 33,
                keyboard::SpecialKey::PageDown => 34,
                keyboard::SpecialKey::ArrowUp => 38,
                keyboard::SpecialKey::ArrowDown => 40,
                keyboard::SpecialKey::ArrowLeft => 37,
                keyboard::SpecialKey::ArrowRight => 39,
                _ => return, // Ignore other special keys
            };

            desktop::handle_key_down(key_code);
        }
        _ => {}
    }
}

/// Handle keyboard character input with mouse simulation (legacy - kept for text mode desktop)
fn handle_keyboard_character(c: char, mouse_x: &mut usize, mouse_y: &mut usize,
                              button_left: &mut bool, _button_right: &mut bool) {
    let key_code = c as u8;

    // Mouse simulation keys (WASD or similar)
    match c {
        // WASD for mouse movement
        'w' | 'W' => *mouse_y = mouse_y.saturating_sub(5),
        'a' | 'A' => *mouse_x = mouse_x.saturating_sub(5),
        's' | 'S' => *mouse_y = (*mouse_y + 5).min(479),
        'd' | 'D' => *mouse_x = (*mouse_x + 5).min(639),
        // Space for left click
        ' ' => *button_left = true,
        _ => {
            // Forward to desktop/window manager
            desktop::handle_key_down(key_code);
        }
    }

    // Log significant keypresses for debugging
    if key_code == 27 { // ESC
        log_debug!("input", "ESC pressed - could trigger menu");
    }
}

/// Handle special key presses (function keys, arrows, etc.)
fn handle_special_key(special_key: keyboard::SpecialKey, mouse_x: &mut usize, mouse_y: &mut usize) {
    // Arrow keys for cursor movement
    let move_amount = 10;
    match special_key {
        keyboard::SpecialKey::ArrowUp => {
            *mouse_y = mouse_y.saturating_sub(move_amount);
            desktop::handle_mouse_move(*mouse_x, *mouse_y);
            return;
        }
        keyboard::SpecialKey::ArrowDown => {
            *mouse_y = (*mouse_y + move_amount).min(479);
            desktop::handle_mouse_move(*mouse_x, *mouse_y);
            return;
        }
        keyboard::SpecialKey::ArrowLeft => {
            *mouse_x = mouse_x.saturating_sub(move_amount);
            desktop::handle_mouse_move(*mouse_x, *mouse_y);
            return;
        }
        keyboard::SpecialKey::ArrowRight => {
            *mouse_x = (*mouse_x + move_amount).min(639);
            desktop::handle_mouse_move(*mouse_x, *mouse_y);
            return;
        }
        _ => {}
    }

    let key_code = match special_key {
        keyboard::SpecialKey::Escape => 27,
        keyboard::SpecialKey::Enter => 13,
        keyboard::SpecialKey::Backspace => 8,
        keyboard::SpecialKey::Tab => 9,
        keyboard::SpecialKey::F1 => 112,  // Help
        keyboard::SpecialKey::F2 => 113,  // Rename
        keyboard::SpecialKey::F3 => 114,  // Search
        keyboard::SpecialKey::F4 => 115,  // Close (Alt+F4)
        keyboard::SpecialKey::F5 => 116,  // Refresh
        keyboard::SpecialKey::F6 => 117,
        keyboard::SpecialKey::F7 => 118,
        keyboard::SpecialKey::F8 => 119,
        keyboard::SpecialKey::F9 => 120,
        keyboard::SpecialKey::F10 => 121,
        keyboard::SpecialKey::F11 => 122, // Fullscreen
        keyboard::SpecialKey::F12 => 123, // Debug console
        keyboard::SpecialKey::Insert => 45,
        keyboard::SpecialKey::Delete => 46,
        keyboard::SpecialKey::Home => 36,
        keyboard::SpecialKey::End => 35,
        keyboard::SpecialKey::PageUp => 33,
        keyboard::SpecialKey::PageDown => 34,
        _ => return, // Already handled or ignore
    };

    desktop::handle_key_down(key_code);

    // Handle special window operations
    match special_key {
        keyboard::SpecialKey::F4 => {
            // Close focused window (would need Alt modifier check)
            log_debug!("input", "F4 pressed - close window shortcut");
        }
        keyboard::SpecialKey::F11 => {
            // Toggle fullscreen
            log_debug!("input", "F11 pressed - fullscreen toggle");
        }
        keyboard::SpecialKey::F12 => {
            // Debug console toggle
            log_debug!("input", "F12 pressed - debug console");
        }
        _ => {}
    }
}

/// Render the mouse cursor at the specified position
fn render_mouse_cursor(x: usize, y: usize, pressed: bool) {
    // Get screen dimensions for bounds checking
    let (max_x, max_y) = if let Some((w, h)) = graphics::get_screen_dimensions() {
        (w, h)
    } else {
        return; // No framebuffer available
    };

    // Cursor color based on state
    let cursor_color = if pressed {
        graphics::Color::rgb(255, 200, 0) // Yellow when pressed
    } else {
        graphics::Color::WHITE
    };

    // Cursor shadow for visibility
    let shadow_color = graphics::Color::rgb(0, 0, 0);

    // Simple arrow cursor pattern (12 pixels tall)
    let cursor_pattern: [(usize, usize); 21] = [
        (0, 0),
        (0, 1), (1, 1),
        (0, 2), (1, 2), (2, 2),
        (0, 3), (1, 3), (2, 3), (3, 3),
        (0, 4), (1, 4), (2, 4), (3, 4), (4, 4),
        (0, 5), (1, 5), (2, 5),
        (0, 6), (1, 6), (3, 6),
    ];

    // Draw shadow first (offset by 1 pixel)
    for &(dx, dy) in cursor_pattern.iter() {
        let px = x + dx + 1;
        let py = y + dy + 1;
        if px < max_x && py < max_y {
            graphics::framebuffer::set_pixel(px, py, shadow_color);
        }
    }

    // Draw cursor
    for &(dx, dy) in cursor_pattern.iter() {
        let px = x + dx;
        let py = y + dy;
        if px < max_x && py < max_y {
            graphics::framebuffer::set_pixel(px, py, cursor_color);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // Alternative entry point - use a static dummy boot info
    // Note: This won't have proper memory map, so memory init will be limited
    static DUMMY_BOOT_INFO: BootInfo = unsafe { core::mem::zeroed() };
    kernel_main(&DUMMY_BOOT_INFO)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::error::{KernelError, SystemError, ErrorSeverity, ErrorContext, ERROR_MANAGER};
    
    // Create error context for panic
    let location = if let Some(loc) = info.location() {
        alloc::format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
    } else {
        "unknown location".to_string()
    };
    
    let message = if let Some(msg) = info.message() {
        alloc::format!("{}", msg)
    } else {
        "Kernel panic occurred".to_string()
    };
    
    let error_context = ErrorContext::new(
        KernelError::System(SystemError::InternalError),
        ErrorSeverity::Fatal,
        "panic_handler",
        alloc::format!("KERNEL PANIC: {} at {}", message, location),
    );
    
    // Try to handle the fatal error gracefully
    if let Ok(mut manager) = ERROR_MANAGER.try_lock() {
        let _ = manager.handle_error(error_context);
    } else {
        // Fallback if error manager is not available
        println!();
        println!("🚨 KERNEL PANIC!");
        println!("Message: {}", message);
        println!("Location: {}", location);
        println!("System halted.");
        
        loop {
            unsafe { core::arch::asm!("hlt"); }
        }
    }
    
    // This should never be reached due to handle_error for Fatal errors
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}
