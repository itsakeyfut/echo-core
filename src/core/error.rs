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
use thiserror::Error;

/// Result type for emulator operations
pub type Result<T> = std::result::Result<T, EmulatorError>;

/// Main error type for the emulator
#[derive(Error, Debug)]
pub enum EmulatorError {
    #[error("BIOS file not found: {0}")]
    BiosNotFound(String),

    #[error("Invalid BIOS size: {got} bytes (expected {expected})")]
    InvalidBiosSize { expected: usize, got: usize },

    #[error("Invalid memory access at 0x{address:08X}")]
    InvalidMemoryAccess { address: u32 },

    #[error("Unaligned memory access: {size}-byte access at 0x{address:08X}")]
    UnalignedAccess { address: u32, size: u8 },

    #[error("Unsupported instruction: 0x{0:08X}")]
    UnsupportedInstruction(u32),

    #[error("CPU exception: {0}")]
    CpuException(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid register index: {index} (valid range: 0-31)")]
    InvalidRegister { index: u8 },

    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),

    #[error("CD-ROM error: {0}")]
    CdRom(#[from] CdRomError),

    #[error("Loader error: {0}")]
    LoaderError(String),
}

/// GPU-specific error types
#[derive(Error, Debug)]
pub enum GpuError {
    #[error("Invalid VRAM access at ({x}, {y})")]
    InvalidVramAccess { x: u16, y: u16 },

    #[error("Invalid GP0 command: {command:#010x}")]
    InvalidGp0Command { command: u32 },

    #[error("Invalid GP1 command: {command:#010x}")]
    InvalidGp1Command { command: u32 },

    #[error("DMA error: {0}")]
    DmaError(String),

    #[error("Rendering backend error: {0}")]
    BackendError(String),
}

/// CD-ROM-specific error types
#[derive(Error, Debug)]
pub enum CdRomError {
    #[error("No disc inserted")]
    NoDisc,

    #[error("Invalid sector: {sector}")]
    InvalidSector { sector: u32 },

    #[error("Read error at sector {sector}: {reason}")]
    ReadError { sector: u32, reason: String },

    #[error("Invalid command: {command:#04x}")]
    InvalidCommand { command: u8 },

    #[error("Invalid parameter count: expected {expected}, got {got}")]
    InvalidParameterCount { expected: usize, got: usize },

    #[error("Seek error: {0}")]
    SeekError(String),

    #[error("Disc load error: {0}")]
    DiscLoadError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
