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

//! BIOS boot tests

use super::super::*;

#[test]
#[ignore] // Requires actual BIOS file - run with: cargo test -- --ignored
fn test_bios_boot() {
    // This test requires an actual PSX BIOS file.
    // Place your BIOS file (e.g., SCPH1001.BIN) in the project root or specify the path.
    //
    // To run this test:
    //   cargo test test_bios_boot -- --ignored --nocapture
    //
    // Note: You must legally own a PlayStation console to use its BIOS.

    let bios_path = std::env::var("PSX_BIOS_PATH").unwrap_or_else(|_| "SCPH1001.BIN".to_string());

    let mut system = System::new();

    // Load actual PSX BIOS
    match system.load_bios(&bios_path) {
        Ok(_) => println!("BIOS loaded successfully from: {}", bios_path),
        Err(e) => {
            println!("Failed to load BIOS: {}", e);
            println!("Set PSX_BIOS_PATH environment variable or place BIOS in project root");
            panic!("BIOS file not found");
        }
    }

    system.reset();

    println!("Starting BIOS execution test...");
    println!("Initial PC: 0x{:08X}", system.pc());

    // Execute first 10,000 instructions
    const TEST_INSTRUCTIONS: usize = 10_000;
    for i in 0..TEST_INSTRUCTIONS {
        if i % 1000 == 0 && i > 0 {
            println!(
                "Progress: {}/{} | PC: 0x{:08X} | Cycles: {}",
                i,
                TEST_INSTRUCTIONS,
                system.pc(),
                system.cycles()
            );
        }

        match system.step() {
            Ok(_) => {}
            Err(e) => {
                println!("Error at PC=0x{:08X}: {}", system.pc(), e);
                println!("Instruction count: {}", i);
                system.cpu().dump_registers();
                panic!("BIOS boot failed");
            }
        }
    }

    // If we got here, BIOS is executing successfully
    println!();
    println!("BIOS boot test completed successfully!");
    println!("Executed {} instructions", TEST_INSTRUCTIONS);
    println!("Total cycles: {}", system.cycles());
    println!("Final PC: 0x{:08X}", system.pc());

    // Basic sanity checks
    assert!(system.cycles() >= TEST_INSTRUCTIONS as u64);
    // PC should have moved from initial BIOS entry point
    assert_ne!(system.pc(), 0xBFC00000);
}

#[test]
#[ignore] // Requires BIOS file and CD-ROM image - run with: cargo test -- --ignored
fn test_bios_boot_with_cdrom() {
    // This is the CRITICAL TEST from Issue #135
    // Verifies that the timing event system fixes the infinite loop bug

    let bios_path =
        std::env::var("PSX_BIOS_PATH").unwrap_or_else(|_| "bios/SCPH1001.bin".to_string());
    let cdrom_path = "assets/usa/Saga Frontier/SaGa Frontier.cue";

    let mut system = System::new();

    // Load BIOS
    match system.load_bios(&bios_path) {
        Ok(_) => println!("BIOS loaded successfully from: {}", bios_path),
        Err(e) => {
            println!("Failed to load BIOS: {}", e);
            println!("Set PSX_BIOS_PATH environment variable or place BIOS in project root");
            panic!("BIOS file not found");
        }
    }

    // Insert CD-ROM (optional - test still valid if disc not found)
    match system.cdrom().borrow_mut().load_disc(cdrom_path) {
        Ok(_) => println!("CD-ROM loaded successfully from: {}", cdrom_path),
        Err(e) => {
            println!("Warning: Failed to load CD-ROM: {}", e);
            println!("Continuing test without CD-ROM...");
        }
    }

    system.reset();

    println!("Starting BIOS boot test with CD-ROM...");
    println!("This test verifies that the timing event system prevents infinite loops");

    // Execute for 10 seconds of emulated time
    const TEST_DURATION_CYCLES: u64 = 33_868_800 * 10; // 10 seconds at 33.8688 MHz
    let target = system.cycles() + TEST_DURATION_CYCLES;

    let mut last_pc = 0;
    let mut stuck_count = 0;
    const MAX_STUCK_COUNT: usize = 1000;

    while system.cycles() < target {
        let pc = system.pc();

        // Check for infinite loop (PC not changing)
        if pc == last_pc {
            stuck_count += 1;
            if stuck_count > MAX_STUCK_COUNT {
                panic!(
                    "Infinite loop detected at PC=0x{:08X} after {} identical PCs",
                    pc, stuck_count
                );
            }
        } else {
            stuck_count = 0;
        }
        last_pc = pc;

        // Execute one frame
        match system.run_frame() {
            Ok(_) => {}
            Err(e) => {
                println!("BIOS boot failed at PC=0x{:08X}: {}", pc, e);
                system.cpu().dump_registers();
                panic!("BIOS boot failed: {}", e);
            }
        }

        // Log progress every second
        if system.cycles() % 33_868_800 < 564_480 {
            println!(
                "BIOS boot progress: {} seconds, PC=0x{:08X}",
                system.cycles() / 33_868_800,
                pc
            );
        }
    }

    println!();
    println!("BIOS boot test completed successfully!");
    println!("Total cycles: {}", system.cycles());
    println!("Total time: 10 seconds (emulated)");
    println!("No infinite loops detected");

    // If we got here without panicking, the infinite loop is fixed
}

#[test]
#[ignore] // Requires BIOS file
fn test_bios_execution_with_timing_events() {
    // Verify that BIOS executes correctly with timing event system active

    let bios_path =
        std::env::var("PSX_BIOS_PATH").unwrap_or_else(|_| "bios/SCPH1001.bin".to_string());

    let mut system = System::new();

    match system.load_bios(&bios_path) {
        Ok(_) => println!("BIOS loaded successfully"),
        Err(e) => {
            println!("Failed to load BIOS: {}", e);
            panic!("BIOS file not found");
        }
    }

    system.reset();

    // Run for 1 second of emulated time
    const ONE_SECOND_CYCLES: u64 = 33_868_800;
    let target = system.cycles() + ONE_SECOND_CYCLES;

    let mut frame_count = 0;

    while system.cycles() < target {
        match system.run_frame() {
            Ok(_) => {
                frame_count += 1;
            }
            Err(e) => {
                panic!("BIOS execution failed: {}", e);
            }
        }
    }

    println!("Executed {} frames in 1 second", frame_count);

    // At 60 FPS, should have approximately 60 frames
    assert!(
        (50..=70).contains(&frame_count),
        "Frame count should be ~60 for 1 second at 60 FPS, got {}",
        frame_count
    );
}

#[test]
fn test_bios_boot_without_bios_file() {
    // Verify graceful handling when BIOS file is not available
    let mut system = System::new();

    let result = system.load_bios("nonexistent.bin");
    assert!(result.is_err(), "Should fail when BIOS file doesn't exist");
}
