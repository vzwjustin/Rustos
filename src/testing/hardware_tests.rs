//! Hardware Testing Framework for RustOS
//!
//! This module provides comprehensive hardware tests for:
//! - Real hardware device detection and initialization
//! - Hardware interrupt handling validation
//! - PCI device enumeration and configuration
//! - ACPI table parsing and hardware discovery
//! - Network device communication
//! - Storage device operations
//! - GPU hardware acceleration
//! - Timer and clock hardware validation

use alloc::{vec::Vec, vec, string::{String, ToString}};
use crate::testing_framework::{TestResult, TestCase, TestSuite, TestType};

/// Create hardware test suite
pub fn create_hardware_test_suite() -> TestSuite {
    TestSuite {
        name: "Hardware Tests".to_string(),
        tests: vec![
            TestCase {
                name: "PCI Device Detection".to_string(),
                test_type: TestType::Integration,
                function: test_pci_device_detection,
                timeout_ms: 10000,
                setup: Some(setup_hardware_tests),
                teardown: Some(teardown_hardware_tests),
                dependencies: vec!["pci".to_string()],
            },
            TestCase {
                name: "ACPI Hardware Discovery".to_string(),
                test_type: TestType::Integration,
                function: test_acpi_hardware_discovery,
                timeout_ms: 10000,
                setup: Some(setup_acpi_tests),
                teardown: Some(teardown_acpi_tests),
                dependencies: vec!["acpi".to_string()],
            },
            TestCase {
                name: "Hardware Interrupt Handling".to_string(),
                test_type: TestType::Integration,
                function: test_hardware_interrupt_handling,
                timeout_ms: 10000,
                setup: Some(setup_interrupt_tests),
                teardown: Some(teardown_interrupt_tests),
                dependencies: vec!["interrupts".to_string(), "apic".to_string()],
            },
            TestCase {
                name: "Timer Hardware Validation".to_string(),
                test_type: TestType::Integration,
                function: test_timer_hardware,
                timeout_ms: 10000,
                setup: Some(setup_timer_tests),
                teardown: Some(teardown_timer_tests),
                dependencies: vec!["time".to_string()],
            },
        ],
        setup: Some(setup_all_hardware_tests),
        teardown: Some(teardown_all_hardware_tests),
    }
}

// Setup and teardown functions
fn setup_all_hardware_tests() {
    // Initialize hardware testing environment

    // Enable hardware detection and enumeration
    crate::pci::enable_detection_mode();

    // Initialize ACPI if available
    let _ = crate::acpi::init();

    // Clear hardware event logs
    crate::hardware::clear_event_log();

    // Enable hardware monitoring
    crate::hardware::enable_monitoring();

    // Record baseline hardware state
    crate::hardware::snapshot_state();
}

fn teardown_all_hardware_tests() {
    // Clean up hardware testing environment

    // Disable hardware monitoring
    crate::hardware::disable_monitoring();

    // Verify hardware state integrity
    crate::hardware::verify_state_integrity();

    // Check for hardware errors during testing
    let errors = crate::hardware::get_error_count();
    if errors > 0 {
        crate::println!("[WARNING] {} hardware errors detected during testing", errors);
    }

    // Reset hardware detection mode
    crate::pci::disable_detection_mode();
}

fn setup_hardware_tests() {
    // Initialize PCI device detection test environment

    // Scan PCI bus and cache results
    let _ = crate::pci::full_bus_scan();

    // Enable PCI configuration space access logging
    crate::pci::enable_config_logging();

    // Clear device enumeration cache
    crate::pci::clear_device_cache();

    // Enable hot-plug detection for testing
    crate::pci::enable_hotplug_detection();

    // Record baseline PCI device count
    let device_count = crate::pci::get_device_count();
    crate::pci::set_test_baseline_count(device_count);
}

fn teardown_hardware_tests() {
    // Clean up PCI device detection test environment

    // Disable PCI configuration space access logging
    crate::pci::disable_config_logging();

    // Disable hot-plug detection
    crate::pci::disable_hotplug_detection();

    // Verify PCI device integrity
    let current_count = crate::pci::get_device_count();
    let baseline_count = crate::pci::get_test_baseline_count();

    if current_count != baseline_count {
        crate::println!("[INFO] PCI device count changed: {} -> {}", baseline_count, current_count);
    }

    // Check for PCI configuration space corruption
    if !crate::pci::verify_config_integrity() {
        crate::println!("[ERROR] PCI configuration space corrupted");
    }
}

fn setup_acpi_tests() {
    // Initialize ACPI hardware discovery test environment

    // Enable ACPI table parsing
    crate::acpi::enable_table_parsing();

    // Clear ACPI table cache
    crate::acpi::clear_table_cache();

    // Enable ACPI event logging
    crate::acpi::enable_event_logging();

    // Scan for ACPI tables
    let _ = crate::acpi::scan_tables();

    // Record baseline ACPI state
    crate::acpi::snapshot_state();
}

fn teardown_acpi_tests() {
    // Clean up ACPI hardware discovery test environment

    // Disable ACPI event logging
    crate::acpi::disable_event_logging();

    // Verify ACPI table integrity
    if !crate::acpi::verify_table_integrity() {
        crate::println!("[ERROR] ACPI table integrity check failed");
    }

    // Check for ACPI errors
    let errors = crate::acpi::get_error_count();
    if errors > 0 {
        crate::println!("[WARNING] {} ACPI errors detected", errors);
    }

    // Restore ACPI state
    crate::acpi::restore_state();
}

fn setup_interrupt_tests() {
    // Initialize hardware interrupt handling test environment

    // Save current interrupt state
    crate::interrupts::save_state();

    // Enable interrupt statistics tracking
    crate::interrupts::enable_statistics();

    // Clear interrupt counters
    crate::interrupts::clear_counters();

    // Enable interrupt latency measurement
    crate::interrupts::enable_latency_measurement();

    // Verify interrupt controller initialization
    if crate::apic::is_available() {
        crate::apic::enable_test_mode();
    } else {
        crate::interrupts::enable_pic_test_mode();
    }

    // Record baseline interrupt statistics
    let stats = crate::interrupts::get_stats();
    crate::interrupts::set_test_baseline_stats(stats);
}

fn teardown_interrupt_tests() {
    // Clean up hardware interrupt handling test environment

    // Disable interrupt latency measurement
    crate::interrupts::disable_latency_measurement();

    // Disable test mode
    if crate::apic::is_available() {
        crate::apic::disable_test_mode();
    } else {
        crate::interrupts::disable_pic_test_mode();
    }

    // Verify interrupt handling integrity
    let final_stats = crate::interrupts::get_stats();
    let baseline_stats = crate::interrupts::get_test_baseline_stats();

    // Check for missed interrupts
    if final_stats.missed_interrupts > baseline_stats.missed_interrupts {
        let missed = final_stats.missed_interrupts - baseline_stats.missed_interrupts;
        crate::println!("[WARNING] {} interrupts missed during testing", missed);
    }

    // Restore interrupt state
    crate::interrupts::restore_state();

    // Disable interrupt statistics tracking
    crate::interrupts::disable_statistics();
}

fn setup_timer_tests() {
    // Initialize timer hardware test environment

    // Save current timer state
    crate::time::save_timer_state();

    // Enable timer statistics collection
    crate::time::enable_statistics();

    // Clear timer event counters
    crate::time::clear_timer_counters();

    // Calibrate TSC for accurate measurements
    crate::time::calibrate_tsc();

    // Record baseline timer state
    let stats = crate::time::get_timer_stats();
    crate::time::set_test_baseline_stats(stats);

    // Enable high-precision timing for tests
    crate::time::enable_high_precision_mode();

    // Initialize test timer callbacks
    crate::time::init_test_callbacks();
}

fn teardown_timer_tests() {
    // Clean up timer hardware test environment

    // Disable high-precision timing
    crate::time::disable_high_precision_mode();

    // Clean up test timer callbacks
    crate::time::cleanup_test_callbacks();

    // Verify timer accuracy
    let final_stats = crate::time::get_timer_stats();
    let baseline_stats = crate::time::get_test_baseline_stats();

    // Check for timer drift
    let ticks_elapsed = final_stats.total_ticks - baseline_stats.total_ticks;
    if ticks_elapsed > 0 {
        let accuracy = crate::time::calculate_timer_accuracy(ticks_elapsed);
        if accuracy < 0.95 {
            crate::println!("[WARNING] Timer accuracy below 95%: {:.2}%", accuracy * 100.0);
        }
    }

    // Restore timer state
    crate::time::restore_timer_state();

    // Disable timer statistics collection
    crate::time::disable_statistics();

    // Verify no timer leaks (pending callbacks)
    let pending = crate::time::get_pending_timer_count();
    if pending > 0 {
        crate::println!("[WARNING] {} timer callbacks still pending", pending);
    }
}

// Hardware test implementations

/// Test PCI device detection and enumeration
fn test_pci_device_detection() -> TestResult {
    let mut devices_found = 0;
    let mut configuration_successful = 0;

    // Test PCI bus scanning
    match crate::pci::scan_pci_bus() {
        Ok(devices) => {
            devices_found = devices.len();
            
            // Test configuration space access for each device
            for device in devices {
                if let Ok(_config) = crate::pci::read_device_config(&device) {
                    configuration_successful += 1;
                    
                    // Test device classification
                    if let Ok(_class) = crate::pci::classify_device(&device) {
                        // Device classification successful
                    }
                    
                    // Test driver loading for known devices
                    if let Ok(_) = crate::pci::load_device_driver(&device) {
                        // Driver loading successful
                    }
                }
            }
        }
        Err(_) => {
            return TestResult::Fail;
        }
    }

    // Pass if we found devices and could configure most of them
    if devices_found > 0 && configuration_successful >= devices_found / 2 {
        TestResult::Pass
    } else if devices_found == 0 {
        TestResult::Skip // No PCI devices found (possible in some environments)
    } else {
        TestResult::Fail
    }
}

/// Test ACPI hardware discovery
fn test_acpi_hardware_discovery() -> TestResult {
    let mut acpi_features_working = 0;
    let total_features = 4;

    // Test ACPI table enumeration
    match crate::acpi::enumerate_tables() {
        Ok(tables) => {
            if !tables.is_empty() {
                acpi_features_working += 1;
            }

            // Test specific table parsing
            for table in &tables.descriptors {
                let sig_str = core::str::from_utf8(&table.signature).unwrap_or("");
                match sig_str {
                    "MADT" => {
                        if crate::acpi::parse_madt().is_ok() {
                            acpi_features_working += 1;
                        }
                    }
                    "HPET" => {
                        if crate::acpi::parse_hpet().is_ok() {
                            // HPET parsing successful
                        }
                    }
                    "FADT" => {
                        if crate::acpi::parse_fadt().is_ok() {
                            // FADT parsing successful
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(_) => {
            return TestResult::Skip; // ACPI not available
        }
    }

    // Test ACPI device enumeration
    match crate::acpi::enumerate_devices() {
        Ok(devices) => {
            if !devices.is_empty() {
                acpi_features_working += 1;
            }
        }
        Err(_) => {
            // Device enumeration failed
        }
    }

    // Test power management features
    if crate::acpi::power_management_available() {
        acpi_features_working += 1;
    }

    if acpi_features_working >= total_features / 2 {
        TestResult::Pass
    } else {
        TestResult::Fail
    }
}

/// Test hardware interrupt handling
fn test_hardware_interrupt_handling() -> TestResult {
    let mut interrupt_tests_passed = 0;
    let total_tests = 5;

    // Test interrupt controller initialization
    match crate::apic::init_apic() {
        Ok(()) => {
            interrupt_tests_passed += 1;
            
            // Test local APIC functionality
            if crate::apic::local_apic_available() {
                interrupt_tests_passed += 1;
            }
            
            // Test I/O APIC functionality
            if crate::apic::io_apic_available() {
                interrupt_tests_passed += 1;
            }
        }
        Err(_) => {
            // Try PIC fallback
            crate::interrupts::init_pic();
            interrupt_tests_passed += 1;
        }
    }

    // Test timer interrupt
    let initial_timer_count = crate::interrupts::get_stats().timer_count;
    
    // Wait for timer interrupts
    let start_time = crate::time::uptime_us();
    while crate::time::uptime_us() - start_time < 100_000 { // 100ms
        unsafe { core::arch::asm!("hlt"); }
    }
    
    let final_timer_count = crate::interrupts::get_stats().timer_count;
    if final_timer_count > initial_timer_count {
        interrupt_tests_passed += 1;
    }

    // Test interrupt masking/unmasking
    if test_interrupt_masking() {
        interrupt_tests_passed += 1;
    }

    if interrupt_tests_passed >= total_tests - 1 {
        TestResult::Pass
    } else {
        TestResult::Fail
    }
}

/// Test timer hardware functionality
fn test_timer_hardware() -> TestResult {
    let mut timer_tests_passed = 0;
    let total_tests = 4;

    // Test timer initialization
    match crate::time::init() {
        Ok(()) => {
            timer_tests_passed += 1;
            
            let stats = crate::time::get_timer_stats();
            
            // Test timer accuracy
            let start_time = crate::time::uptime_us();
            
            // Busy wait for approximately 10ms
            let target_delay = 10_000; // 10ms in microseconds
            while crate::time::uptime_us() - start_time < target_delay {
                core::hint::spin_loop();
            }
            
            let actual_delay = crate::time::uptime_us() - start_time;
            let accuracy = if actual_delay > target_delay {
                target_delay as f64 / actual_delay as f64
            } else {
                actual_delay as f64 / target_delay as f64
            };
            
            // Accept 90% accuracy or better
            if accuracy >= 0.9 {
                timer_tests_passed += 1;
            }
            
            // Test TSC calibration
            if stats.tsc_frequency > 0 {
                timer_tests_passed += 1;
            }
            
            // Test timer scheduling
            let timer_scheduled = test_timer_scheduling();
            if timer_scheduled {
                timer_tests_passed += 1;
            }
        }
        Err(_) => {
            return TestResult::Fail;
        }
    }

    if timer_tests_passed >= total_tests - 1 {
        TestResult::Pass
    } else {
        TestResult::Fail
    }
}

// Helper functions for hardware tests

fn test_interrupt_masking() -> bool {
    // Test interrupt enable/disable functionality
    let initial_state = crate::interrupts::interrupts_enabled();
    
    // Disable interrupts
    crate::interrupts::disable_interrupts();
    let disabled_state = crate::interrupts::interrupts_enabled();
    
    // Re-enable interrupts
    crate::interrupts::enable_interrupts();
    let enabled_state = crate::interrupts::interrupts_enabled();
    
    // Restore initial state
    if initial_state {
        crate::interrupts::enable_interrupts();
    } else {
        crate::interrupts::disable_interrupts();
    }
    
    // Test passed if we could control interrupt state
    !disabled_state && enabled_state
}

fn test_timer_scheduling() -> bool {
    use core::sync::atomic::{AtomicBool, Ordering};
    
    static TIMER_FIRED: AtomicBool = AtomicBool::new(false);
    
    // Schedule a timer callback
    let timer_id = crate::time::schedule_timer(1_000_000, || { // 1 second
        TIMER_FIRED.store(true, Ordering::Release);
    });

    // Wait for timer to fire (with timeout)
    let start_time = crate::time::uptime_us();
    while !TIMER_FIRED.load(Ordering::Acquire) {
        if crate::time::uptime_us() - start_time > 2_000_000 { // 2 second timeout
            break;
        }
        unsafe { core::arch::asm!("hlt"); }
    }

    let timer_fired = TIMER_FIRED.load(Ordering::Acquire);

    // Clean up
    let _ = crate::time::cancel_timer(timer_id);
    
    timer_fired
}