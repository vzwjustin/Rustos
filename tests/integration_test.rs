#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rustos::ai::inference_engine::{InferenceEngine, InferenceRule};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::serial_println!("[failed]\n");
    rustos::serial_println!("Error: {}\n", info);
    rustos::exit_qemu(rustos::QemuExitCode::Failed);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[test_case]
fn test_inference_engine_basic() {
    let mut engine = InferenceEngine::new();
    assert!(engine.initialize().is_ok());
    assert!(engine.get_rules_count() > 0);
}

#[test_case]
fn test_inference_rule_matching() {
    let rule = InferenceRule::new([0.5f32; 8], 1.0, 1);
    let input = [0.5f32; 8];
    let similarity = rule.matches(&input);

    // Should be very close to 1.0 for identical patterns
    assert!((similarity - 1.0).abs() < 0.001);
}

#[test_case]
fn test_inference_engine_prediction() {
    let mut engine = InferenceEngine::new();
    let _ = engine.initialize();

    // Test ascending pattern
    let ascending_input = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let confidence = engine.infer(&ascending_input).unwrap();

    // Should have some confidence > 0
    assert!(confidence > 0.0);
}
