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

//! psrx: A PlayStation (PSX) emulator
//!
//! This crate provides a complete PSX emulator implementation.
//!
//! # Architecture
//!
//! The emulator is organized into the following modules:
//!
//! - [`core`]: Core emulation components (CPU, Memory, GPU, SPU, System)
//!
//! # Example
//!
//! ```no_run
//! use psrx::core::system::System;
//!
//! let mut system = System::new();
//! // system.load_bios("path/to/bios.bin")?;
//! // system.run()?;
//! # Ok::<(), psrx::core::error::EmulatorError>(())
//! ```
//!
//! # Getting Started
//!
//! 1. Create a [`core::system::System`] instance
//! 2. Load a BIOS file
//! 3. Run the emulation loop
//!
//! # Modules
//!
//! - [`core::cpu`]: MIPS R3000A CPU emulation
//! - [`core::memory`]: Memory bus and address translation
//! - [`core::gpu`]: Graphics processing unit (stub, Phase 2)
//! - [`core::spu`]: Sound processing unit (stub, Phase 4)
//! - [`core::system`]: System integration and main loop
//!
//! # Error Handling
//!
//! All fallible operations return [`core::error::Result<T>`] which is an alias for
//! `Result<T, EmulatorError>`.

pub mod core;

// Re-export commonly used types
pub use core::error::{EmulatorError, Result};
