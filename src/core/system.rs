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

//! System integration module
//!
//! This module ties together all emulator components (CPU, Memory, GPU, SPU)
//! and provides the main emulation loop.

use super::cpu::CPU;
use super::error::Result;
use super::gpu::GPU;
use super::memory::Bus;
use super::spu::SPU;

/// PlayStation System
///
/// Integrates all hardware components and manages the emulation loop.
///
/// # Components
/// - CPU: MIPS R3000A processor
/// - Bus: Memory bus for RAM, BIOS, and I/O
/// - GPU: Graphics processing unit
/// - SPU: Sound processing unit
///
/// # Example
/// ```no_run
/// use echo_core::core::system::System;
///
/// let mut system = System::new();
/// // system.load_bios("path/to/bios.bin")?;
/// // system.run();
/// ```
pub struct System {
    /// CPU instance
    cpu: CPU,
    /// Memory bus
    bus: Bus,
    /// GPU instance
    gpu: GPU,
    /// SPU instance
    spu: SPU,
    /// Total cycles executed
    cycles: u64,
}

impl System {
    /// Create a new System instance
    ///
    /// Initializes all hardware components to their reset state.
    ///
    /// # Returns
    /// Initialized System instance
    pub fn new() -> Self {
        Self {
            cpu: CPU::new(),
            bus: Bus::new(),
            gpu: GPU::new(),
            spu: SPU::new(),
            cycles: 0,
        }
    }

    /// Reset the system to initial state
    ///
    /// Resets all components as if the console was power-cycled.
    /// This clears RAM/scratchpad but preserves loaded BIOS.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.bus.reset();
        self.gpu = GPU::new();
        self.spu = SPU::new();
        self.cycles = 0;
    }

    /// Execute one CPU instruction
    ///
    /// # Returns
    /// Number of cycles consumed
    ///
    /// # Errors
    /// Returns error if instruction execution fails
    pub fn step(&mut self) -> Result<u32> {
        let cpu_cycles = self.cpu.step(&mut self.bus)?;
        self.cycles += cpu_cycles as u64;

        // TODO: Step GPU and SPU in future phases
        // self.gpu.step()?;
        // self.spu.step()?;

        Ok(cpu_cycles)
    }

    /// Get total cycles executed
    ///
    /// # Returns
    /// Total number of cycles since reset
    pub fn total_cycles(&self) -> u64 {
        self.cycles
    }

    /// Get reference to CPU
    ///
    /// # Returns
    /// Reference to CPU instance
    pub fn cpu(&self) -> &CPU {
        &self.cpu
    }

    /// Get mutable reference to CPU
    ///
    /// # Returns
    /// Mutable reference to CPU instance
    pub fn cpu_mut(&mut self) -> &mut CPU {
        &mut self.cpu
    }

    /// Get reference to memory bus
    ///
    /// # Returns
    /// Reference to Bus instance
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Get mutable reference to memory bus
    ///
    /// # Returns
    /// Mutable reference to Bus instance
    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_initialization() {
        let system = System::new();
        assert_eq!(system.total_cycles(), 0);
        assert_eq!(system.cpu().pc(), 0xBFC00000);
    }

    #[test]
    fn test_system_reset() {
        let mut system = System::new();

        // Execute some instructions to change state
        // (This will work once we have proper BIOS/instructions loaded)

        system.reset();
        assert_eq!(system.total_cycles(), 0);
        assert_eq!(system.cpu().pc(), 0xBFC00000);
    }
}
