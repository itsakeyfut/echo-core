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

//! PlayStation emulator with Slint UI
//!
//! This binary provides a graphical interface for the emulator using Slint.
//! It displays the GPU framebuffer in real-time and provides an FPS counter.

use clap::Parser;
use log::{error, info};
use psrx::core::system::System;
use psrx::frontend::Frontend;
use std::env;

/// PlayStation (PSX) emulator with UI
#[derive(Parser)]
#[command(name = "psrx-ui")]
#[command(about = "PlayStation emulator with graphical interface", long_about = None)]
struct Args {
    /// Path to PlayStation BIOS file (e.g., SCPH1001.BIN)
    bios_file: String,

    /// Path to CD-ROM disc image (.cue file)
    #[arg(short = 'c', long)]
    cdrom: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (for development configuration)
    // This allows developers to configure trace settings, log levels, etc.
    // File is optional - if not present, will use defaults or OS environment variables
    if let Err(e) = dotenvy::dotenv() {
        // Only log if the error is NOT "file not found"
        if !e.to_string().contains("not found") {
            eprintln!("Warning: Failed to load .env file: {}", e);
        }
    }

    // Initialize logger with default level INFO
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("PSRX v{}", env!("CARGO_PKG_VERSION"));
    info!("PlayStation emulator with UI");

    // Parse command line arguments
    let args = Args::parse();

    info!("Loading BIOS from: {}", args.bios_file);

    // Create and initialize system
    let mut system = System::new();

    // Load BIOS
    if let Err(e) = system.load_bios(&args.bios_file) {
        error!("Failed to load BIOS: {}", e);
        return Err(Box::new(e));
    }

    info!("BIOS loaded successfully");

    // Load CD-ROM disc if specified
    if let Some(cdrom_path) = &args.cdrom {
        info!("Loading CD-ROM disc from: {}", cdrom_path);
        if let Err(e) = system.cdrom().borrow_mut().load_disc(cdrom_path) {
            error!("Failed to load CD-ROM disc: {}", e);
            return Err(Box::new(e));
        }
        info!("CD-ROM disc loaded successfully");
    }

    // Reset system to start execution
    info!("Starting emulator...");
    system.reset();

    // Create and run frontend
    let frontend = Frontend::new(system);
    frontend.run()?;

    info!("Emulator stopped");
    Ok(())
}
