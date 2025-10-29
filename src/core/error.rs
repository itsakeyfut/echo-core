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

/// Emulator error types
use std::fmt;

/// Result type for emulator operations
pub type Result<T> = std::result::Result<T, EmulatorError>;

/// Emulator error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmulatorError {
    /// Unaligned memory access error
    UnalignedAccess {
        /// The address that was accessed
        address: u32,
        /// The size of the access (2 for 16-bit, 4 for 32-bit)
        size: u32,
    },

    /// Invalid memory access (unmapped region)
    InvalidAddress {
        /// The address that was accessed
        address: u32,
    },

    /// I/O error (file operations)
    IoError {
        /// Error message
        message: String,
    },

    /// BIOS file error
    BiosError {
        /// Error message
        message: String,
    },

    /// BIOS has an unexpected size
    InvalidBiosSize {
        /// Expected size in bytes
        expected: usize,
        /// Actual size in bytes
        got: usize,
    },
}

impl fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmulatorError::UnalignedAccess { address, size } => {
                write!(
                    f,
                    "Unaligned {}-bit access at address 0x{:08X}",
                    size * 8,
                    address
                )
            }
            EmulatorError::InvalidAddress { address } => {
                write!(f, "Invalid memory access at address 0x{:08X}", address)
            }
            EmulatorError::IoError { message } => {
                write!(f, "I/O error: {}", message)
            }
            EmulatorError::BiosError { message } => {
                write!(f, "BIOS error: {}", message)
            }
            EmulatorError::InvalidBiosSize { expected, got } => {
                write!(
                    f,
                    "Invalid BIOS size: expected {} bytes, got {} bytes",
                    expected, got
                )
            }
        }
    }
}

impl std::error::Error for EmulatorError {}

impl From<std::io::Error> for EmulatorError {
    fn from(err: std::io::Error) -> Self {
        EmulatorError::IoError {
            message: err.to_string(),
        }
    }
}
