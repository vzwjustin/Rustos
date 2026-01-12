//! Boot Display Module for RustOS
//!
//! Provides visual boot logo and enhanced display using VGA text mode.
//! This module handles text-mode boot visualization including progress bars,
//! system information panels, and service status displays.

use crate::vga_buffer::{Color, VGA_WRITER};
use crate::{print, println};
use alloc::format;
use alloc::string::String;

/// Display boot logo with ASCII art
pub fn show_boot_logo() {
    // Set colors for the logo
    set_color_temp(Color::LightCyan, Color::Black);

    println!();
    println!("    ██████╗ ██╗   ██╗███████╗████████╗ ██████╗ ███████╗");
    println!("    ██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔═══██╗██╔════╝");
    println!("    ██████╔╝██║   ██║███████╗   ██║   ██║   ██║███████╗");
    println!("    ██╔══██╗██║   ██║╚════██║   ██║   ██║   ██║╚════██║");
    println!("    ██║  ██║╚██████╔╝███████║   ██║   ╚██████╔╝███████║");
    println!("    ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝ ╚══════╝");
    println!();

    // Reset to default colors
    set_color_temp(Color::White, Color::Black);

    // Add subtitle with different color
    set_color_temp(Color::Yellow, Color::Black);
    print_centered("Advanced Rust Operating System");
    set_color_temp(Color::LightGray, Color::Black);
    print_centered("Version 1.0.0 - Enhanced Edition");
    println!();
}

/// Display boot progress bar
pub fn show_boot_progress(step: usize, total: usize, message: &str) {
    let progress = (step * 50) / total;
    let percentage = (step * 100) / total;

    set_color_temp(Color::LightBlue, Color::Black);
    print!("{}  [", message);

    // Draw progress bar
    set_color_temp(Color::LightGreen, Color::Green);
    for i in 0..progress {
        if i < progress {
            print!("█");
        }
    }

    set_color_temp(Color::DarkGray, Color::Black);
    for _ in progress..50 {
        print!("░");
    }

    set_color_temp(Color::LightBlue, Color::Black);
    println!("] {}%", percentage);

    // Reset colors
    set_color_temp(Color::White, Color::Black);
}

/// Show system information panel
pub fn show_system_info() {
    println!();
    draw_box("System Information", 60);

    set_color_temp(Color::LightCyan, Color::Black);
    println!("  ◆ Architecture: x86_64");
    println!("  ◆ Kernel Type: Microkernel");
    println!("  ◆ Memory Model: 64-bit Linear");
    println!("  ◆ Boot Method: Multiboot2");
    println!("  ◆ Graphics: VGA Text Mode");

    set_color_temp(Color::White, Color::Black);
    draw_line(60);
}

/// Show memory information
pub fn show_memory_info(total_mb: usize, usable_mb: usize, regions: usize) {
    println!();
    draw_box("Memory Configuration", 60);

    set_color_temp(Color::LightGreen, Color::Black);
    println!("  ◇ Total Memory:    {} MB", total_mb);
    println!("  ◇ Usable Memory:   {} MB", usable_mb);
    println!("  ◇ Memory Regions:  {}", regions);
    println!("  ◇ Heap Reserved:   100 MB");

    let usage_percent = if total_mb > 0 { (usable_mb * 100) / total_mb } else { 0 };
    println!("  ◇ Memory Usage:    {}%", usage_percent);

    set_color_temp(Color::White, Color::Black);
    draw_line(60);
}

/// Show kernel services status
pub fn show_services_status() {
    println!();
    draw_box("Kernel Services", 60);

    show_service_status("VGA Text Buffer", true);
    show_service_status("Print Subsystem", true);
    show_service_status("Memory Manager", true);
    show_service_status("Interrupt Handler", false);
    show_service_status("Process Scheduler", false);
    show_service_status("Network Stack", false);

    draw_line(60);
}

/// Show desktop environment startup
pub fn show_desktop_startup() {
    println!();
    set_color_temp(Color::Pink, Color::Black);
    print_centered("┌─────────────────────────────────────┐");
    print_centered("│        Starting Desktop...         │");
    print_centered("└─────────────────────────────────────┘");
    println!();

    set_color_temp(Color::White, Color::Black);

    // Show desktop features
    println!("  Desktop Features:");
    set_color_temp(Color::LightCyan, Color::Black);
    println!("    • Window Management System");
    println!("    • Hardware Accelerated Graphics");
    println!("    • Multi-tasking Environment");
    println!("    • File System Integration");
    println!("    • Network Connectivity");

    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Helper function to show service status
fn show_service_status(service: &str, active: bool) {
    print!("  ► {:<20} ", service);

    if active {
        set_color_temp(Color::LightGreen, Color::Black);
        println!("[ACTIVE]");
    } else {
        set_color_temp(Color::Red, Color::Black);
        println!("[INACTIVE]");
    }

    set_color_temp(Color::White, Color::Black);
}

/// Print text centered on screen
fn print_centered(text: &str) {
    let width = 80; // VGA text mode width
    let padding = (width - text.len()) / 2;

    for _ in 0..padding {
        print!(" ");
    }
    println!("{}", text);
}

/// Draw a decorative box with title
fn draw_box(title: &str, width: usize) {
    // Top border
    set_color_temp(Color::LightBlue, Color::Black);
    print!("  ╔");
    for _ in 0..(width-4) {
        print!("═");
    }
    println!("╗");

    // Title line
    let title_padding = ((width - 4) - title.len()) / 2;
    print!("  ║");
    for _ in 0..title_padding {
        print!(" ");
    }
    set_color_temp(Color::Yellow, Color::Black);
    print!("{}", title);
    set_color_temp(Color::LightBlue, Color::Black);
    for _ in 0..title_padding {
        print!(" ");
    }
    if title.len() % 2 == 1 {
        print!(" "); // Extra space for odd titles
    }
    println!("║");

    // Separator
    print!("  ╠");
    for _ in 0..(width-4) {
        print!("═");
    }
    println!("╣");

    set_color_temp(Color::White, Color::Black);
}

/// Draw bottom line for box
fn draw_line(width: usize) {
    set_color_temp(Color::LightBlue, Color::Black);
    print!("  ╚");
    for _ in 0..(width-4) {
        print!("═");
    }
    println!("╝");
    set_color_temp(Color::White, Color::Black);
}

/// Temporarily set VGA colors (helper function)
fn set_color_temp(foreground: Color, background: Color) {
    let mut writer = VGA_WRITER.lock();
    writer.set_color(foreground, background);
}

/// Show welcome message
pub fn show_welcome_message() {
    println!();
    set_color_temp(Color::LightGreen, Color::Black);
    print_centered("┌───────────────────────────────────────────────────────────────┐");
    print_centered("│                    Welcome to RustOS!                        │");
    print_centered("│                                                               │");
    print_centered("│           Your secure, fast, and reliable OS                 │");
    print_centered("│                  Built with Rust 🦀                         │");
    print_centered("└───────────────────────────────────────────────────────────────┘");
    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Add some delay for visual effect
pub fn boot_delay() {
    // Simple delay loop
    for _ in 0..10_000_000 {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

// ============================================================================
// Enhanced Boot Display Functions
// ============================================================================

/// Boot phase enumeration for detailed progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    EarlyInit,
    HardwareProbe,
    MemorySetup,
    DriverInit,
    ServiceStart,
    DesktopLaunch,
}

impl BootPhase {
    /// Get display name for the phase
    pub fn name(&self) -> &'static str {
        match self {
            BootPhase::EarlyInit => "Early Initialization",
            BootPhase::HardwareProbe => "Hardware Detection",
            BootPhase::MemorySetup => "Memory Configuration",
            BootPhase::DriverInit => "Driver Initialization",
            BootPhase::ServiceStart => "Starting Services",
            BootPhase::DesktopLaunch => "Launching Desktop",
        }
    }

    /// Get phase number (1-6)
    pub fn number(&self) -> usize {
        match self {
            BootPhase::EarlyInit => 1,
            BootPhase::HardwareProbe => 2,
            BootPhase::MemorySetup => 3,
            BootPhase::DriverInit => 4,
            BootPhase::ServiceStart => 5,
            BootPhase::DesktopLaunch => 6,
        }
    }
}

/// Display detailed boot progress with substeps
pub fn show_detailed_progress(phase: BootPhase, substep: usize, total_substeps: usize, detail: &str) {
    let phase_progress = (phase.number() - 1) * 100 / 6;
    let substep_progress = if total_substeps > 0 {
        (substep * 100 / 6) / total_substeps
    } else {
        0
    };
    let total_progress = phase_progress + substep_progress;

    // Phase header
    set_color_temp(Color::LightBlue, Color::Black);
    print!("  [{}/6] {} ", phase.number(), phase.name());

    // Substep indicator
    if total_substeps > 0 {
        set_color_temp(Color::DarkGray, Color::Black);
        print!("({}/{}) ", substep, total_substeps);
    }

    // Progress bar
    set_color_temp(Color::DarkGray, Color::Black);
    print!("[");

    let bar_width = 25;
    let filled = (total_progress * bar_width) / 100;

    set_color_temp(Color::LightGreen, Color::Black);
    for _ in 0..filled {
        print!("#");
    }

    set_color_temp(Color::DarkGray, Color::Black);
    for _ in filled..bar_width {
        print!("-");
    }

    print!("] {}%", total_progress);

    set_color_temp(Color::White, Color::Black);
    println!();

    // Detail message
    if !detail.is_empty() {
        set_color_temp(Color::Cyan, Color::Black);
        println!("      -> {}", detail);
        set_color_temp(Color::White, Color::Black);
    }
}

/// Display hardware detection summary
pub fn show_hardware_summary(
    cpu_cores: usize,
    memory_mb: usize,
    storage_count: usize,
    network_count: usize,
    gpu_detected: bool,
) {
    println!();
    draw_box("Hardware Summary", 60);

    set_color_temp(Color::LightCyan, Color::Black);
    println!("  CPU:      {} core(s) detected", cpu_cores);
    println!("  Memory:   {} MB total", memory_mb);
    println!("  Storage:  {} device(s)", storage_count);
    println!("  Network:  {} interface(s)", network_count);
    println!("  Graphics: {}", if gpu_detected { "GPU detected" } else { "VGA only" });

    set_color_temp(Color::White, Color::Black);
    draw_line(60);
}

/// Display driver loading status
pub fn show_driver_status(driver_name: &str, loaded: bool, version: Option<&str>) {
    print!("  ");
    set_color_temp(Color::White, Color::Black);
    print!("{:<24} ", driver_name);

    if loaded {
        set_color_temp(Color::LightGreen, Color::Black);
        print!("[LOADED]");
        if let Some(ver) = version {
            set_color_temp(Color::DarkGray, Color::Black);
            print!(" v{}", ver);
        }
    } else {
        set_color_temp(Color::Red, Color::Black);
        print!("[FAILED]");
    }

    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Display boot error with recovery options
pub fn show_boot_error(error_code: u32, message: &str, recoverable: bool) {
    println!();
    set_color_temp(Color::Red, Color::Black);
    println!("  ╔════════════════════════════════════════════════════════╗");
    println!("  ║             BOOT ERROR ENCOUNTERED                     ║");
    println!("  ╠════════════════════════════════════════════════════════╣");
    set_color_temp(Color::White, Color::Black);
    println!("  ║ Error Code: 0x{:08X}                                  ║", error_code);
    set_color_temp(Color::Yellow, Color::Black);

    // Word-wrap the message
    let words: alloc::vec::Vec<&str> = message.split_whitespace().collect();
    let mut line = String::from("  ║ ");
    for word in words {
        if line.len() + word.len() + 1 > 58 {
            while line.len() < 60 {
                line.push(' ');
            }
            println!("{}║", line);
            line = String::from("  ║ ");
        }
        line.push_str(word);
        line.push(' ');
    }
    while line.len() < 60 {
        line.push(' ');
    }
    println!("{}║", line);

    set_color_temp(Color::Red, Color::Black);
    println!("  ╠════════════════════════════════════════════════════════╣");

    if recoverable {
        set_color_temp(Color::LightGreen, Color::Black);
        println!("  ║ This error is recoverable.                            ║");
        println!("  ║                                                        ║");
        set_color_temp(Color::Cyan, Color::Black);
        println!("  ║ Options:                                               ║");
        println!("  ║   [S] Continue in Safe Mode                            ║");
        println!("  ║   [R] Retry initialization                             ║");
        println!("  ║   [C] Continue anyway (may be unstable)                ║");
    } else {
        set_color_temp(Color::Red, Color::Black);
        println!("  ║ This error is NOT recoverable.                        ║");
        println!("  ║ The system cannot continue booting.                   ║");
        println!("  ║                                                        ║");
        set_color_temp(Color::Yellow, Color::Black);
        println!("  ║ Please check your hardware configuration and          ║");
        println!("  ║ try rebooting the system.                             ║");
    }

    set_color_temp(Color::Red, Color::Black);
    println!("  ╚════════════════════════════════════════════════════════╝");
    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Display safe mode banner
pub fn show_safe_mode_banner() {
    println!();
    set_color_temp(Color::Yellow, Color::Black);
    println!("  ╔════════════════════════════════════════════════════════╗");
    println!("  ║                    SAFE MODE ACTIVE                     ║");
    println!("  ╠════════════════════════════════════════════════════════╣");
    set_color_temp(Color::White, Color::Black);
    println!("  ║ RustOS is running in Safe Mode with limited features.  ║");
    println!("  ║                                                        ║");
    println!("  ║ Disabled features:                                     ║");
    println!("  ║   - Hardware acceleration                              ║");
    println!("  ║   - Advanced graphics modes                            ║");
    println!("  ║   - Non-essential drivers                              ║");
    println!("  ║                                                        ║");
    println!("  ║ To exit Safe Mode, reboot and select normal boot.      ║");
    set_color_temp(Color::Yellow, Color::Black);
    println!("  ╚════════════════════════════════════════════════════════╝");
    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Display boot complete message with timing
pub fn show_boot_complete(boot_time_ms: u64) {
    println!();
    set_color_temp(Color::LightGreen, Color::Black);
    println!("  ╔════════════════════════════════════════════════════════╗");
    println!("  ║              BOOT SEQUENCE COMPLETE                    ║");
    println!("  ╠════════════════════════════════════════════════════════╣");
    set_color_temp(Color::White, Color::Black);

    let seconds = boot_time_ms / 1000;
    let millis = boot_time_ms % 1000;
    println!("  ║ Boot time: {}.{:03} seconds                             ║", seconds, millis);
    println!("  ║                                                        ║");
    println!("  ║ All systems initialized successfully.                  ║");
    println!("  ║ Launching desktop environment...                       ║");

    set_color_temp(Color::LightGreen, Color::Black);
    println!("  ╚════════════════════════════════════════════════════════╝");
    set_color_temp(Color::White, Color::Black);
    println!();
}

/// Display animated loading indicator
pub fn show_loading_animation(frame: usize) {
    let spinner = ['|', '/', '-', '\\'];
    let idx = frame % 4;

    set_color_temp(Color::Cyan, Color::Black);
    print!("\r  Loading {} ", spinner[idx]);
    set_color_temp(Color::White, Color::Black);
}

/// Display subsystem initialization status
pub fn show_subsystem_init(name: &str, status: SubsystemStatus) {
    print!("      ");

    match status {
        SubsystemStatus::Initializing => {
            set_color_temp(Color::Yellow, Color::Black);
            print!("[....] ");
        }
        SubsystemStatus::Ready => {
            set_color_temp(Color::LightGreen, Color::Black);
            print!("[ OK ] ");
        }
        SubsystemStatus::Failed => {
            set_color_temp(Color::Red, Color::Black);
            print!("[FAIL] ");
        }
        SubsystemStatus::Skipped => {
            set_color_temp(Color::DarkGray, Color::Black);
            print!("[SKIP] ");
        }
        SubsystemStatus::Warning => {
            set_color_temp(Color::Yellow, Color::Black);
            print!("[WARN] ");
        }
    }

    set_color_temp(Color::White, Color::Black);
    println!("{}", name);
}

/// Subsystem initialization status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemStatus {
    Initializing,
    Ready,
    Failed,
    Skipped,
    Warning,
}

/// Display graphics mode information
pub fn show_graphics_mode_info(width: usize, height: usize, bpp: usize, accelerated: bool) {
    println!();
    draw_box("Graphics Configuration", 60);

    set_color_temp(Color::LightCyan, Color::Black);
    println!("  Resolution:   {}x{}", width, height);
    println!("  Color Depth:  {} bits per pixel", bpp);
    println!("  Acceleration: {}", if accelerated { "Hardware" } else { "Software" });

    set_color_temp(Color::White, Color::Black);
    draw_line(60);
}

/// Display memory map summary
pub fn show_memory_map_summary(
    total_mb: usize,
    usable_mb: usize,
    kernel_mb: usize,
    heap_mb: usize,
) {
    println!();
    draw_box("Memory Layout", 60);

    set_color_temp(Color::LightCyan, Color::Black);
    println!("  Total Memory:     {} MB", total_mb);
    println!("  Usable Memory:    {} MB", usable_mb);
    println!("  Kernel Reserved:  {} MB", kernel_mb);
    println!("  Heap Allocated:   {} MB", heap_mb);

    // Memory bar visualization
    let used_percent = if total_mb > 0 {
        ((total_mb - usable_mb + kernel_mb + heap_mb) * 100) / total_mb
    } else {
        0
    };

    set_color_temp(Color::White, Color::Black);
    print!("  Usage: [");

    let bar_width = 40;
    let filled = (used_percent * bar_width) / 100;

    set_color_temp(Color::LightGreen, Color::Black);
    for _ in 0..filled.min(bar_width / 2) {
        print!("#");
    }
    set_color_temp(Color::Yellow, Color::Black);
    for _ in (bar_width / 2)..(filled.min(bar_width * 3 / 4)) {
        print!("#");
    }
    set_color_temp(Color::Red, Color::Black);
    for _ in (bar_width * 3 / 4)..filled.min(bar_width) {
        print!("#");
    }
    set_color_temp(Color::DarkGray, Color::Black);
    for _ in filled..bar_width {
        print!("-");
    }

    set_color_temp(Color::White, Color::Black);
    println!("] {}%", used_percent);

    draw_line(60);
}

/// Display kernel version information
pub fn show_kernel_version() {
    println!();
    set_color_temp(Color::LightBlue, Color::Black);
    println!("  ┌─────────────────────────────────────────────────────────┐");
    println!("  │                    RustOS Kernel                        │");
    println!("  ├─────────────────────────────────────────────────────────┤");
    set_color_temp(Color::White, Color::Black);
    println!("  │  Version:     1.0.0                                     │");
    println!("  │  Build:       Release                                   │");
    println!("  │  Architecture: x86_64                                   │");
    println!("  │  Compiler:    Rust (nightly)                            │");
    set_color_temp(Color::LightBlue, Color::Black);
    println!("  └─────────────────────────────────────────────────────────┘");
    set_color_temp(Color::White, Color::Black);
}

/// Display countdown before auto-continuing
pub fn show_countdown(seconds: usize, message: &str) {
    for i in (1..=seconds).rev() {
        set_color_temp(Color::Yellow, Color::Black);
        print!("\r  {} {} ", message, i);
        set_color_temp(Color::White, Color::Black);

        // Delay for approximately 1 second
        for _ in 0..100_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
    println!();
}