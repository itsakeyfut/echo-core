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

//! Core emulation components
//!
//! This module contains all hardware emulation components:
//! - CPU (MIPS R3000A)
//! - Memory bus
//! - GPU (Graphics Processing Unit)
//! - SPU (Sound Processing Unit)
//! - System integration

pub mod cpu;
pub mod error;
pub mod gpu;
pub mod memory;
pub mod spu;
pub mod system;

// Re-export commonly used types
pub use cpu::CPU;
pub use error::{EmulatorError, GpuError, Result};
pub use gpu::GPU;
pub use memory::Bus;
pub use spu::SPU;
pub use system::System;
