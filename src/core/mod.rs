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
//! - CD-ROM (Disc drive)
//! - DMA (Direct Memory Access)
//! - Controller (Input devices)
//! - Timer (3 timer/counter channels)
//! - Interrupt Controller (IRQ management)
//! - Timing Event System (Global timing and event scheduling)
//! - System integration

pub mod cdrom;
pub mod controller;
pub mod cpu;
pub mod dma;
pub mod error;
pub mod gpu;
pub mod interrupt;
pub mod loader;
pub mod memory;
pub mod spu;
pub mod system;
pub mod timer;
pub mod timing;

// Re-export commonly used types
pub use cdrom::CDROM;
pub use controller::Controller;
pub use cpu::CPU;
pub use dma::DMA;
pub use error::{CdRomError, EmulatorError, GpuError, Result};
pub use gpu::GPU;
pub use interrupt::InterruptController;
pub use loader::{PSXExecutable, SystemConfig};
pub use memory::Bus;
pub use spu::SPU;
pub use system::System;
pub use timer::Timers;
pub use timing::TimingEventManager;
