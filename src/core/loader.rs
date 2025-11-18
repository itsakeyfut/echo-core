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

//! Game loading system for PlayStation 1
//!
//! This module handles the loading of game executables including:
//! - SYSTEM.CNF configuration file parsing
//! - PSX-EXE executable format loading
//! - ISO9660 filesystem basic support
//!
//! # Game Boot Sequence
//!
//! When a game boots from CD, the BIOS follows this sequence:
//! 1. Read SYSTEM.CNF from disc to find the boot executable
//! 2. Load the PSX-EXE file specified in SYSTEM.CNF
//! 3. Copy executable data to RAM at the specified load address
//! 4. Set CPU registers (PC, GP, SP, FP) from executable header
//! 5. Jump to executable entry point
//!
//! # SYSTEM.CNF Format
//!
//! The SYSTEM.CNF file is a plain text configuration file with key=value pairs:
//!
//! ```text
//! BOOT = cdrom:\SLUS_000.01;1
//! TCB = 4
//! EVENT = 10
//! STACK = 0x801FFF00
//! ```
//!
//! # PSX-EXE Format
//!
//! PSX-EXE files have a 2048-byte header followed by the executable code:
//!
//! ```text
//! 0x00-0x07: "PS-X EXE" magic
//! 0x10-0x13: Initial PC (entry point)
//! 0x14-0x17: Initial GP (global pointer)
//! 0x18-0x1B: Load address
//! 0x1C-0x1F: Load size
//! 0x30-0x33: Stack base
//! 0x34-0x37: Stack offset
//! 0x800+:    Executable data
//! ```
//!
//! # Example
//!
//! ```no_run
//! use psrx::core::loader::{SystemConfig, PSXExecutable};
//!
//! // Parse SYSTEM.CNF
//! let config_data = "BOOT = cdrom:\\SLUS_000.01;1\nSTACK = 0x801FFF00";
//! let config = SystemConfig::parse(config_data).unwrap();
//! assert_eq!(config.boot_file, "cdrom:\\SLUS_000.01;1");
//!
//! // Load PSX-EXE (with actual file data)
//! // let exe_data = std::fs::read("game.exe").unwrap();
//! // let exe = PSXExecutable::load(&exe_data).unwrap();
//! ```

use super::error::{EmulatorError, Result};

/// SYSTEM.CNF configuration
///
/// Represents the parsed contents of a PlayStation SYSTEM.CNF file.
/// This file is read from the CD-ROM during boot to determine which
/// executable to load and various system settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemConfig {
    /// Boot file path (e.g., "cdrom:\SLUS_000.01;1")
    pub boot_file: String,

    /// TCB (Thread Control Block) count
    ///
    /// Default: false (TCB count controlled by kernel)
    pub tce: bool,

    /// Event name/configuration
    pub event: String,

    /// Stack pointer address
    ///
    /// Default: 0x801FFF00
    pub stack: u32,
}

impl SystemConfig {
    /// Parse SYSTEM.CNF data from a string
    ///
    /// Parses the key=value pairs in the SYSTEM.CNF file and returns
    /// a SystemConfig structure with the parsed values.
    ///
    /// # Arguments
    ///
    /// * `data` - SYSTEM.CNF file contents as a string
    ///
    /// # Returns
    ///
    /// - `Ok(SystemConfig)` if parsing succeeds
    /// - `Err(EmulatorError)` if parsing fails
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::loader::SystemConfig;
    ///
    /// let data = r#"
    ///     BOOT = cdrom:\SLUS_000.01;1
    ///     TCB = 4
    ///     EVENT = 10
    ///     STACK = 0x801FFF00
    /// "#;
    ///
    /// let config = SystemConfig::parse(data).unwrap();
    /// assert_eq!(config.boot_file, "cdrom:\\SLUS_000.01;1");
    /// assert_eq!(config.stack, 0x801FFF00);
    /// ```
    pub fn parse(data: &str) -> Result<Self> {
        let mut boot_file = String::new();
        let mut tce = false;
        let mut event = String::new();
        let mut stack = 0x801FFF00; // Default stack address

        for line in data.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key=value pair
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "BOOT" => boot_file = value.to_string(),
                    "TCB" => tce = value == "1",
                    "EVENT" => event = value.to_string(),
                    "STACK" => {
                        // Parse hex value (with or without 0x prefix)
                        let hex_str = value.trim_start_matches("0x").trim_start_matches("0X");
                        stack = u32::from_str_radix(hex_str, 16).map_err(|e| {
                            EmulatorError::LoaderError(format!("Invalid STACK value: {}", e))
                        })?;
                    }
                    _ => {
                        log::warn!("Unknown SYSTEM.CNF key: {}", key);
                    }
                }
            }
        }

        if boot_file.is_empty() {
            return Err(EmulatorError::LoaderError(
                "BOOT file not specified in SYSTEM.CNF".to_string(),
            ));
        }

        Ok(Self {
            boot_file,
            tce,
            event,
            stack,
        })
    }
}

/// PSX-EXE executable
///
/// Represents a loaded PlayStation executable with its header information
/// and executable data. PSX-EXE files have a specific format with a 2048-byte
/// header followed by the actual code/data.
#[derive(Debug, Clone)]
pub struct PSXExecutable {
    /// Initial program counter (entry point)
    pub pc: u32,

    /// Initial global pointer (GP register, r28)
    pub gp: u32,

    /// Load address in RAM
    pub load_address: u32,

    /// Size of data to load
    pub load_size: u32,

    /// Stack base address
    pub stack_base: u32,

    /// Stack offset from base
    pub stack_offset: u32,

    /// Executable data (code and initialized data)
    pub data: Vec<u8>,
}

impl PSXExecutable {
    /// PSX-EXE header size
    const HEADER_SIZE: usize = 0x800;

    /// Load a PSX-EXE file from binary data
    ///
    /// Parses the PSX-EXE header and extracts the executable data.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw PSX-EXE file data (header + executable)
    ///
    /// # Returns
    ///
    /// - `Ok(PSXExecutable)` if loading succeeds
    /// - `Err(EmulatorError)` if the file is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::loader::PSXExecutable;
    ///
    /// // Create minimal valid PSX-EXE header for testing
    /// let mut exe_data = vec![0u8; 0x900];
    /// exe_data[0..8].copy_from_slice(b"PS-X EXE");
    ///
    /// let exe = PSXExecutable::load(&exe_data).unwrap();
    /// ```
    pub fn load(data: &[u8]) -> Result<Self> {
        // Check minimum size (header must be at least 2048 bytes)
        if data.len() < Self::HEADER_SIZE {
            return Err(EmulatorError::LoaderError(
                "Invalid PSX-EXE: file too small".to_string(),
            ));
        }

        // Check magic number "PS-X EXE"
        if &data[0..8] != b"PS-X EXE" {
            return Err(EmulatorError::LoaderError(
                "Invalid PSX-EXE: bad magic number".to_string(),
            ));
        }

        // Parse header fields (little-endian)
        let pc = u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]]);
        let gp = u32::from_le_bytes([data[0x14], data[0x15], data[0x16], data[0x17]]);
        let load_address = u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]]);
        let load_size = u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]);
        let stack_base = u32::from_le_bytes([data[0x30], data[0x31], data[0x32], data[0x33]]);
        let stack_offset = u32::from_le_bytes([data[0x34], data[0x35], data[0x36], data[0x37]]);

        // Extract executable data
        let data_start = Self::HEADER_SIZE;
        let data_end = data_start + load_size as usize;

        if data_end > data.len() {
            return Err(EmulatorError::LoaderError(format!(
                "Invalid PSX-EXE: load_size (0x{:X}) exceeds file size",
                load_size
            )));
        }

        let exe_data = data[data_start..data_end].to_vec();

        log::info!(
            "PSX-EXE loaded: PC=0x{:08X}, GP=0x{:08X}, Load=0x{:08X}, Size=0x{:X}",
            pc,
            gp,
            load_address,
            load_size
        );

        Ok(Self {
            pc,
            gp,
            load_address,
            load_size,
            stack_base,
            stack_offset,
            data: exe_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_cnf_parsing() {
        let data = r#"
            BOOT = cdrom:\SLUS_000.01;1
            TCB = 1
            EVENT = 10
            STACK = 0x801FFF00
        "#;

        let config = SystemConfig::parse(data).unwrap();
        assert_eq!(config.boot_file, "cdrom:\\SLUS_000.01;1");
        assert_eq!(config.tce, true);
        assert_eq!(config.event, "10");
        assert_eq!(config.stack, 0x801FFF00);
    }

    #[test]
    fn test_system_cnf_with_comments() {
        let data = r#"
            # This is a comment
            BOOT = cdrom:\GAME.EXE;1
            # Another comment
            STACK = 0x801FF000
        "#;

        let config = SystemConfig::parse(data).unwrap();
        assert_eq!(config.boot_file, "cdrom:\\GAME.EXE;1");
        assert_eq!(config.stack, 0x801FF000);
    }

    #[test]
    fn test_system_cnf_default_values() {
        let data = "BOOT = cdrom:\\BOOT.EXE;1";

        let config = SystemConfig::parse(data).unwrap();
        assert_eq!(config.boot_file, "cdrom:\\BOOT.EXE;1");
        assert_eq!(config.tce, false);
        assert_eq!(config.event, "");
        assert_eq!(config.stack, 0x801FFF00); // Default value
    }

    #[test]
    fn test_system_cnf_missing_boot() {
        let data = "STACK = 0x801FFF00";

        let result = SystemConfig::parse(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_psx_exe_loading() {
        // Create minimal valid PSX-EXE header
        let mut data = vec![0u8; 0x900];

        // Magic "PS-X EXE"
        data[0..8].copy_from_slice(b"PS-X EXE");

        // PC = 0x80010000
        data[0x10..0x14].copy_from_slice(&0x80010000u32.to_le_bytes());

        // GP = 0x80020000
        data[0x14..0x18].copy_from_slice(&0x80020000u32.to_le_bytes());

        // Load address = 0x80010000
        data[0x18..0x1C].copy_from_slice(&0x80010000u32.to_le_bytes());

        // Load size = 0x100
        data[0x1C..0x20].copy_from_slice(&0x100u32.to_le_bytes());

        // Stack base = 0x801FFF00
        data[0x30..0x34].copy_from_slice(&0x801FFF00u32.to_le_bytes());

        // Stack offset = 0
        data[0x34..0x38].copy_from_slice(&0u32.to_le_bytes());

        let exe = PSXExecutable::load(&data).unwrap();

        assert_eq!(exe.pc, 0x80010000);
        assert_eq!(exe.gp, 0x80020000);
        assert_eq!(exe.load_address, 0x80010000);
        assert_eq!(exe.load_size, 0x100);
        assert_eq!(exe.stack_base, 0x801FFF00);
        assert_eq!(exe.stack_offset, 0);
        assert_eq!(exe.data.len(), 0x100);
    }

    #[test]
    fn test_psx_exe_invalid_magic() {
        let mut data = vec![0u8; 0x900];
        data[0..8].copy_from_slice(b"INVALID!");

        let result = PSXExecutable::load(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_psx_exe_too_small() {
        let data = vec![0u8; 0x100]; // Less than 2048 bytes

        let result = PSXExecutable::load(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_psx_exe_size_mismatch() {
        let mut data = vec![0u8; 0x800 + 0x10];

        // Magic "PS-X EXE"
        data[0..8].copy_from_slice(b"PS-X EXE");

        // Load size larger than actual data
        data[0x1C..0x20].copy_from_slice(&0x1000u32.to_le_bytes());

        let result = PSXExecutable::load(&data);
        assert!(result.is_err());
    }
}
