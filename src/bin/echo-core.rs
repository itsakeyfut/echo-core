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

use echo_core::core::error::Result;
use echo_core::core::system::System;
use log::{error, info};
use std::env;

fn main() -> Result<()> {
    // Initialize logger with default level INFO
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("echo-core v{}", env!("CARGO_PKG_VERSION"));
    info!("PlayStation emulator");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <bios_file>", args[0]);
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  <bios_file>  Path to PlayStation BIOS file (e.g., SCPH1001.BIN)");
        eprintln!();
        eprintln!("Environment variables:");
        eprintln!("  RUST_LOG     Set log level (trace, debug, info, warn, error)");
        std::process::exit(1);
    }

    let bios_path = &args[1];
    info!("Loading BIOS from: {}", bios_path);

    // Create and initialize system
    let mut system = System::new();

    // Load BIOS
    if let Err(e) = system.load_bios(bios_path) {
        error!("Failed to load BIOS: {}", e);
        return Err(e);
    }

    info!("BIOS loaded successfully");

    // Reset system to start execution
    info!("Starting emulation...");
    system.reset();

    // Run for 100,000 instructions
    const TOTAL_INSTRUCTIONS: usize = 100_000;
    const LOG_INTERVAL: usize = 10_000;

    for i in 0..TOTAL_INSTRUCTIONS {
        // Log progress periodically
        if i % LOG_INTERVAL == 0 && i > 0 {
            info!(
                "Progress: {}/{} instructions | PC: 0x{:08X} | Cycles: {}",
                i,
                TOTAL_INSTRUCTIONS,
                system.pc(),
                system.cycles()
            );
        }

        // Execute one instruction
        if let Err(e) = system.step() {
            error!("Error at PC=0x{:08X}: {}", system.pc(), e);
            error!("Instruction count: {}", i);
            system.cpu().dump_registers();
            return Err(e);
        }
    }

    // Final status
    info!("Emulation completed successfully!");
    info!("Total instructions: {}", TOTAL_INSTRUCTIONS);
    info!("Total cycles: {}", system.cycles());
    info!("Final PC: 0x{:08X}", system.pc());

    Ok(())
}
