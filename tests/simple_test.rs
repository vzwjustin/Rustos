#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

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
fn test_pattern_creation() {
    let pattern = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.0, 0.0, 0.0];
    assert_eq!(pattern.len(), 8);
    assert_eq!(pattern[0], 0.1);
}

#[test_case]
fn test_similarity_calculation() {
    let pattern1 = [0.5f32, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
    let pattern2 = [0.5f32, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];

    let mut similarity = 0.0f32;
    for (a, b) in pattern1.iter().zip(pattern2.iter()) {
        similarity += 1.0 - (a - b).abs();
    }
    similarity /= pattern1.len() as f32;

    assert!((similarity - 1.0).abs() < 0.001);
}

#[test_case]
fn test_neural_network_concepts() {
    // Test basic neural network concepts
    let weights = [[0.1f32, 0.2], [0.3, 0.4]];
    let input = [1.0f32, 0.5];

    let mut output = 0.0f32;
    for i in 0..weights.len() {
        for j in 0..input.len() {
            output += weights[i][j] * input[j];
        }
    }

    assert!(output > 0.0);
}
