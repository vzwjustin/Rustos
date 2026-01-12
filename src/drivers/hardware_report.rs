//! Hardware Detection Report Module
//!
//! This module provides comprehensive reporting of detected hardware peripherals,
//! including PS/2 devices, input capabilities, and system configuration.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::{ps2_controller, ps2_mouse, input_manager};

/// Hardware detection report for a single device
#[derive(Debug, Clone)]
pub struct DeviceReport {
    pub device_name: String,
    pub device_type: String,
    pub status: DeviceStatus,
    pub details: Vec<String>,
    pub capabilities: Vec<String>,
}

/// Device status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Detected,
    Active,
    NotFound,
    Error,
}

impl DeviceReport {
    pub fn new(name: String, device_type: String) -> Self {
        Self {
            device_name: name,
            device_type,
            status: DeviceStatus::NotFound,
            details: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: DeviceStatus) -> Self {
        self.status = status;
        self
    }

    pub fn add_detail(&mut self, detail: String) {
        self.details.push(detail);
    }

    pub fn add_capability(&mut self, capability: String) {
        self.capabilities.push(capability);
    }
}

/// Complete hardware report for all peripherals
#[derive(Debug, Clone)]
pub struct HardwareReport {
    pub ps2_controller: DeviceReport,
    pub keyboard: DeviceReport,
    pub mouse: DeviceReport,
    pub input_manager: DeviceReport,
}

impl HardwareReport {
    pub fn new() -> Self {
        Self {
            ps2_controller: DeviceReport::new(
                "PS/2 Controller".into(),
                "System Controller".into()
            ),
            keyboard: DeviceReport::new(
                "PS/2 Keyboard".into(),
                "Input Device".into()
            ),
            mouse: DeviceReport::new(
                "PS/2 Mouse".into(),
                "Pointing Device".into()
            ),
            input_manager: DeviceReport::new(
                "Input Manager".into(),
                "System Service".into()
            ),
        }
    }

    /// Generate a complete hardware report by querying all subsystems
    pub fn generate() -> Self {
        let mut report = Self::new();

        // Check PS/2 Controller
        if ps2_controller::is_initialized() {
            report.ps2_controller.status = DeviceStatus::Active;
            report.ps2_controller.add_detail("8042 PS/2 Controller".into());

            if let Some((port1_avail, port1_dev, port2_avail, port2_dev)) = ps2_controller::get_device_info() {
                if port1_avail {
                    report.ps2_controller.add_detail(format!(
                        "Port 1: {} ({})",
                        if port1_avail { "Available" } else { "Not Available" },
                        device_type_name(port1_dev)
                    ));
                    report.ps2_controller.add_capability("Keyboard Support".into());
                }

                if port2_avail {
                    report.ps2_controller.add_detail(format!(
                        "Port 2: {} ({})",
                        if port2_avail { "Available" } else { "Not Available" },
                        device_type_name(port2_dev)
                    ));
                    report.ps2_controller.add_capability("Mouse Support".into());
                }
            }
        } else {
            report.ps2_controller.status = DeviceStatus::NotFound;
            report.ps2_controller.add_detail("Controller not initialized".into());
        }

        // Check Keyboard
        if crate::keyboard::is_initialized() {
            report.keyboard.status = DeviceStatus::Active;
            report.keyboard.add_detail("PS/2 Scancode Set 1".into());

            let stats = crate::keyboard::get_extended_statistics();
            report.keyboard.add_detail(format!(
                "Keypresses: {}, Releases: {}",
                stats.basic_stats.total_keypresses,
                stats.basic_stats.total_releases
            ));

            report.keyboard.add_capability("Full QWERTY Layout".into());
            report.keyboard.add_capability("Special Keys (F1-F12, Arrows, etc.)".into());
            report.keyboard.add_capability("Modifier Keys (Shift, Ctrl, Alt)".into());

            if stats.caps_lock_enabled {
                report.keyboard.add_detail("Caps Lock: ON".into());
            }
            if stats.num_lock_enabled {
                report.keyboard.add_detail("Num Lock: ON".into());
            }
        } else {
            report.keyboard.status = DeviceStatus::NotFound;
        }

        // Check Mouse
        if ps2_mouse::is_initialized() {
            report.mouse.status = DeviceStatus::Active;

            if let Some(protocol) = ps2_mouse::get_protocol() {
                let protocol_name = match protocol {
                    ps2_mouse::MouseProtocol::Standard => "Standard PS/2 (3-byte)",
                    ps2_mouse::MouseProtocol::IntelliMouse => "IntelliMouse (4-byte, Scroll Wheel)",
                    ps2_mouse::MouseProtocol::IntelliMouseExplorer => "IntelliMouse Explorer (5-button)",
                };
                report.mouse.add_detail(format!("Protocol: {}", protocol_name));

                // Add capabilities based on protocol
                report.mouse.add_capability("Left Button".into());
                report.mouse.add_capability("Right Button".into());
                report.mouse.add_capability("Middle Button".into());

                if protocol != ps2_mouse::MouseProtocol::Standard {
                    report.mouse.add_capability("Scroll Wheel".into());
                }

                if protocol == ps2_mouse::MouseProtocol::IntelliMouseExplorer {
                    report.mouse.add_capability("4th Button".into());
                    report.mouse.add_capability("5th Button".into());
                }
            }

            if let Some((packets_rx, packets_dropped)) = ps2_mouse::get_statistics() {
                report.mouse.add_detail(format!(
                    "Packets: {} received, {} dropped",
                    packets_rx,
                    packets_dropped
                ));
            }
        } else {
            report.mouse.status = DeviceStatus::NotFound;
            report.mouse.add_detail("Mouse driver not initialized".into());
        }

        // Check Input Manager
        if input_manager::is_initialized() {
            report.input_manager.status = DeviceStatus::Active;

            let (cursor_x, cursor_y) = input_manager::get_cursor_position();
            report.input_manager.add_detail(format!("Cursor Position: ({}, {})", cursor_x, cursor_y));

            let (events_queued, events_dropped) = input_manager::get_statistics();
            report.input_manager.add_detail(format!(
                "Events: {} processed, {} dropped",
                events_queued,
                events_dropped
            ));

            report.input_manager.add_capability("Unified Event Queue".into());
            report.input_manager.add_capability("Keyboard Event Routing".into());
            report.input_manager.add_capability("Mouse Event Routing".into());
            report.input_manager.add_capability("Cursor Position Tracking".into());
            report.input_manager.add_capability("Button State Management".into());
        } else {
            report.input_manager.status = DeviceStatus::NotFound;
        }

        report
    }

    /// Get a summary count of detected, active, and failed devices
    pub fn get_summary(&self) -> (usize, usize, usize) {
        let devices = [
            &self.ps2_controller,
            &self.keyboard,
            &self.mouse,
            &self.input_manager,
        ];

        let mut detected = 0;
        let mut active = 0;
        let mut failed = 0;

        for device in &devices {
            match device.status {
                DeviceStatus::Detected => detected += 1,
                DeviceStatus::Active => active += 1,
                DeviceStatus::Error | DeviceStatus::NotFound => failed += 1,
            }
        }

        (detected, active, failed)
    }

    /// Print a formatted report to the console
    pub fn print(&self) {
        crate::println!();
        crate::println!("========================================");
        crate::println!("     HARDWARE PERIPHERAL REPORT");
        crate::println!("========================================");
        crate::println!();

        let (detected, active, failed) = self.get_summary();
        crate::println!("Summary: {} active, {} detected, {} not found", active, detected, failed);
        crate::println!();

        // Print PS/2 Controller
        self.print_device(&self.ps2_controller);
        crate::println!();

        // Print Keyboard
        self.print_device(&self.keyboard);
        crate::println!();

        // Print Mouse
        self.print_device(&self.mouse);
        crate::println!();

        // Print Input Manager
        self.print_device(&self.input_manager);
        crate::println!();

        crate::println!("========================================");
    }

    /// Print a single device report
    fn print_device(&self, device: &DeviceReport) {
        let status_symbol = match device.status {
            DeviceStatus::Active => "[✓]",
            DeviceStatus::Detected => "[•]",
            DeviceStatus::NotFound => "[✗]",
            DeviceStatus::Error => "[!]",
        };

        crate::println!("{} {} ({})", status_symbol, device.device_name, device.device_type);

        if !device.details.is_empty() {
            crate::println!("    Details:");
            for detail in &device.details {
                crate::println!("      - {}", detail);
            }
        }

        if !device.capabilities.is_empty() {
            crate::println!("    Capabilities:");
            for capability in &device.capabilities {
                crate::println!("      + {}", capability);
            }
        }
    }

    /// Get a compact one-line summary
    pub fn get_compact_summary(&self) -> String {
        let (_, active, failed) = self.get_summary();
        format!("{} peripherals active, {} not detected", active, failed)
    }
}

/// Helper function to convert device type enum to readable name
fn device_type_name(device_type: ps2_controller::Ps2DeviceType) -> &'static str {
    match device_type {
        ps2_controller::Ps2DeviceType::Keyboard => "Keyboard",
        ps2_controller::Ps2DeviceType::StandardMouse => "Standard Mouse",
        ps2_controller::Ps2DeviceType::MouseWithScrollWheel => "Mouse with Scroll Wheel",
        ps2_controller::Ps2DeviceType::Mouse5Button => "5-Button Mouse",
        ps2_controller::Ps2DeviceType::Unknown => "Unknown Device",
    }
}

/// Print a quick peripheral status check
pub fn print_quick_status() {
    let report = HardwareReport::generate();
    crate::println!();
    crate::println!("Peripheral Status: {}", report.get_compact_summary());
    crate::println!();
}

/// Check if all critical peripherals are operational
pub fn check_critical_peripherals() -> Result<(), &'static str> {
    if !ps2_controller::is_initialized() {
        return Err("PS/2 controller not initialized");
    }

    if !crate::keyboard::is_initialized() {
        return Err("Keyboard not initialized");
    }

    if !input_manager::is_initialized() {
        return Err("Input manager not initialized");
    }

    Ok(())
}

/// Get detailed mouse information for debugging
pub fn get_mouse_debug_info() -> String {
    if !ps2_mouse::is_initialized() {
        return "Mouse: Not initialized".into();
    }

    let mut info = String::from("Mouse Debug Info:\n");

    if let Some(protocol) = ps2_mouse::get_protocol() {
        info.push_str(&format!("  Protocol: {:?}\n", protocol));
    }

    if let Some((rx, dropped)) = ps2_mouse::get_statistics() {
        info.push_str(&format!("  Packets: {} received, {} dropped\n", rx, dropped));
        let success_rate = if rx > 0 {
            (rx as f32 / (rx + dropped) as f32) * 100.0
        } else {
            0.0
        };
        info.push_str(&format!("  Success Rate: {:.2}%\n", success_rate));
    }

    if let Some(buttons) = ps2_mouse::get_button_state() {
        info.push_str(&format!(
            "  Buttons: L={} R={} M={} B4={} B5={}\n",
            if buttons.left { "✓" } else { "✗" },
            if buttons.right { "✓" } else { "✗" },
            if buttons.middle { "✓" } else { "✗" },
            if buttons.button4 { "✓" } else { "✗" },
            if buttons.button5 { "✓" } else { "✗" },
        ));
    }

    info
}

/// Get detailed input manager information
pub fn get_input_manager_debug_info() -> String {
    if !input_manager::is_initialized() {
        return "Input Manager: Not initialized".into();
    }

    let mut info = String::from("Input Manager Debug Info:\n");

    let (x, y) = input_manager::get_cursor_position();
    info.push_str(&format!("  Cursor: ({}, {})\n", x, y));

    let (queued, dropped) = input_manager::get_statistics();
    info.push_str(&format!("  Events: {} queued, {} dropped\n", queued, dropped));

    let queue_count = input_manager::get_event_count();
    info.push_str(&format!("  Queue: {} events pending\n", queue_count));

    let buttons = input_manager::get_button_states();
    info.push_str(&format!(
        "  Buttons: L={} R={} M={}\n",
        if buttons.left { "Down" } else { "Up" },
        if buttons.right { "Down" } else { "Up" },
        if buttons.middle { "Down" } else { "Up" },
    ));

    info
}
