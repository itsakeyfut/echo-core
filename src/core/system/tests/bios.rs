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
