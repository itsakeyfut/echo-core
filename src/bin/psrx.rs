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

use clap::Parser;
use log::{error, info};
use psrx::core::error::Result;
use psrx::core::system::System;

/// PlayStation (PSX) emulator
#[derive(Parser)]
#[command(name = "psrx")]
#[command(about = "PlayStation emulator", long_about = None)]
struct Args {
    /// Path to PlayStation BIOS file (e.g., SCPH1001.BIN)
    bios_file: String,

    /// Path to CD-ROM image file (.cue)
    #[arg(short = 'c', long)]
    cdrom: Option<String>,

    /// Number of instructions to execute
    #[arg(short = 'n', long, default_value = "100000")]
    instructions: usize,
}

fn main() -> Result<()> {
    // Initialize logger with default level INFO
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("psrx v{}", env!("CARGO_PKG_VERSION"));
    info!("PlayStation emulator");

    // Parse command line arguments
    let args = Args::parse();

    info!("Loading BIOS from: {}", args.bios_file);

    // Create and initialize system
    let mut system = System::new();

    // Load BIOS
    if let Err(e) = system.load_bios(&args.bios_file) {
        error!("Failed to load BIOS: {}", e);
        return Err(e);
    }

    info!("BIOS loaded successfully");

    // Load CD-ROM image if provided
    if let Some(cdrom_path) = &args.cdrom {
        info!("Loading CD-ROM from: {}", cdrom_path);
        system
            .cdrom()
            .borrow_mut()
            .load_disc(cdrom_path)
            .map_err(|e| {
                error!("Failed to load CD-ROM: {}", e);
                // CdRomError is automatically converted to EmulatorError via #[from]
                psrx::core::error::EmulatorError::CdRom(e)
            })?;
        info!("CD-ROM loaded successfully");
    }

    // Reset system to start execution
    info!("Starting emulation...");
    system.reset();

    // Run for specified number of instructions
    let total_instructions = args.instructions;
    let log_interval = (total_instructions / 10).max(1); // Log ~10 times during execution

    for i in 0..total_instructions {
        // Log progress periodically
        if i % log_interval == 0 && i > 0 {
            info!(
                "Progress: {}/{} instructions | PC: 0x{:08X} | Cycles: {}",
                i,
                total_instructions,
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
    info!("Total instructions: {}", total_instructions);
    info!("Total cycles: {}", system.cycles());
    info!("Final PC: 0x{:08X}", system.pc());

    Ok(())
}
