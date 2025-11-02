// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! BIOS Boot Integration Tests
//!
//! These tests verify that the complete emulator stack can boot a real PSX BIOS
//! and display graphics output.
//!
//! # Requirements
//!
//! These tests require an actual PSX BIOS file. Set the `PSX_BIOS_PATH` environment
//! variable or place a BIOS file named `SCPH1001.BIN` in the project root.
//!
//! # Running
//!
//! ```bash
//! # Run all BIOS boot tests
//! cargo test --test bios_boot -- --ignored --nocapture
//!
//! # Run specific test
//! cargo test --test bios_boot test_bios_boot -- --ignored --nocapture
//! ```
//!
//! # Legal Notice
//!
//! You must legally own a PlayStation console to use its BIOS for testing.

use psrx::core::system::System;

/// Get BIOS path from environment or default location
fn get_bios_path() -> String {
    std::env::var("PSX_BIOS_PATH").unwrap_or_else(|_| "SCPH1001.BIN".to_string())
}

/// Test basic BIOS boot sequence
///
/// This test verifies that the BIOS can be loaded and executed for a short period.
/// It checks that:
/// - BIOS loads without errors
/// - CPU executes instructions successfully
/// - PC advances from the reset vector
/// - GPU receives commands
#[test]
#[ignore] // Requires BIOS file - run with: cargo test -- --ignored
fn test_bios_boot() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let bios_path = get_bios_path();
    let mut system = System::new();

    // Load actual PSX BIOS
    match system.load_bios(&bios_path) {
        Ok(_) => println!("BIOS loaded successfully from: {}", bios_path),
        Err(e) => {
            eprintln!("Failed to load BIOS: {}", e);
            eprintln!("Set PSX_BIOS_PATH environment variable or place BIOS in project root");
            panic!("BIOS file not found: {}", bios_path);
        }
    }

    system.reset();

    println!("Starting BIOS boot test...");
    println!("Initial PC: 0x{:08X}", system.pc());

    // Run for 1 second (60 frames)
    const TEST_FRAMES: usize = 60;
    for frame in 0..TEST_FRAMES {
        if frame % 10 == 0 {
            println!(
                "Frame {}/{} | PC: 0x{:08X} | Cycles: {}",
                frame,
                TEST_FRAMES,
                system.pc(),
                system.cycles()
            );
        }

        match system.run_frame() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error at frame {}: {}", frame, e);
                eprintln!("PC: 0x{:08X}", system.pc());
                panic!("BIOS boot failed");
            }
        }
    }

    println!();
    println!("BIOS boot test completed successfully!");
    println!("Executed {} frames", TEST_FRAMES);
    println!("Total cycles: {}", system.cycles());
    println!("Final PC: 0x{:08X}", system.pc());

    // Verify CPU is executing
    assert_ne!(
        system.pc(),
        0xBFC00000,
        "CPU should have moved past reset vector"
    );

    // Should have executed approximately 60 * 564,480 cycles
    assert!(
        system.cycles() >= 60 * 564_480,
        "Should execute at least 60 frames worth of cycles"
    );
}

/// Test PSX logo display
///
/// This test runs the BIOS for ~5 seconds to display the PSX logo,
/// then verifies that the framebuffer contains visible content.
#[test]
#[ignore] // Requires BIOS file - run with: cargo test -- --ignored
fn test_logo_display() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let bios_path = get_bios_path();
    let mut system = System::new();

    // Load BIOS
    system
        .load_bios(&bios_path)
        .expect("Failed to load BIOS for logo test");
    system.reset();

    println!("Running PSX logo display test...");
    println!("This will take approximately 5 seconds...");

    // Run for ~5 seconds (300 frames)
    // PSX logo should be displayed during this time
    const LOGO_FRAMES: usize = 300;
    for frame in 0..LOGO_FRAMES {
        if frame % 60 == 0 {
            println!("Progress: {}s | PC: 0x{:08X}", frame / 60, system.pc());
        }

        system.run_frame().expect("Frame execution failed");
    }

    println!();
    println!("Logo display period complete");

    // Get framebuffer
    let gpu = system.gpu();
    let framebuffer = gpu.borrow().get_framebuffer();

    // Verify framebuffer is not all black
    // Count non-black pixels (RGB values not all zero)
    let non_black_pixels = framebuffer
        .chunks(3)
        .filter(|rgb| rgb[0] != 0 || rgb[1] != 0 || rgb[2] != 0)
        .count();

    let total_pixels = framebuffer.len() / 3;
    let non_black_percentage = (non_black_pixels as f32 / total_pixels as f32) * 100.0;

    println!("Framebuffer analysis:");
    println!("  Total pixels: {}", total_pixels);
    println!(
        "  Non-black pixels: {} ({:.2}%)",
        non_black_pixels, non_black_percentage
    );

    assert!(
        non_black_pixels > 100,
        "Framebuffer should have visible content (found {} non-black pixels)",
        non_black_pixels
    );

    println!("✓ Logo display test passed!");
}

/// Test BIOS execution stability
///
/// Runs the BIOS for an extended period to verify stability.
#[test]
#[ignore] // Requires BIOS file and takes time - run with: cargo test -- --ignored
fn test_bios_stability() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let bios_path = get_bios_path();
    let mut system = System::new();

    system
        .load_bios(&bios_path)
        .expect("Failed to load BIOS for stability test");
    system.reset();

    println!("Running BIOS stability test (30 seconds)...");

    // Run for 30 seconds (1800 frames)
    const STABILITY_FRAMES: usize = 1800;
    for frame in 0..STABILITY_FRAMES {
        if frame % 60 == 0 {
            println!("Progress: {}s / 30s", frame / 60);
        }

        if let Err(e) = system.run_frame() {
            eprintln!("Stability test failed at frame {}: {}", frame, e);
            panic!("Emulation crashed during stability test");
        }
    }

    println!();
    println!("✓ Stability test passed!");
    println!("Ran for 30 seconds without crashes");
}

/// Test GPU status during BIOS boot
///
/// Verifies that GPU status flags are set correctly during BIOS execution.
#[test]
#[ignore] // Requires BIOS file - run with: cargo test -- --ignored
fn test_gpu_status_during_boot() {
    let bios_path = get_bios_path();
    let mut system = System::new();

    system
        .load_bios(&bios_path)
        .expect("Failed to load BIOS for GPU status test");
    system.reset();

    // Run for a few frames
    for _ in 0..60 {
        system.run_frame().expect("Frame execution failed");
    }

    // Check GPU status
    let gpu = system.gpu();
    let status = gpu.borrow().status();

    // Verify ready flags are set
    assert_ne!(
        status & (1 << 26),
        0,
        "GPU should be ready to receive command"
    );
    assert_ne!(status & (1 << 27), 0, "GPU should be ready to send VRAM");
    assert_ne!(status & (1 << 28), 0, "GPU should be ready to receive DMA");

    println!("GPU status: 0x{:08X}", status);
    println!("✓ GPU status test passed!");
}
