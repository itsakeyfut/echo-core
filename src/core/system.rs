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
    /// Running state
    running: bool,
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
            running: false,
        }
    }

    /// Load BIOS from file
    ///
    /// Loads a BIOS ROM file into the system. The BIOS must be 512KB in size.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the BIOS file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if BIOS was loaded successfully
    /// - `Err(EmulatorError)` if loading fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use echo_core::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.load_bios("SCPH1001.BIN").unwrap();
    /// ```
    pub fn load_bios(&mut self, path: &str) -> Result<()> {
        self.bus.load_bios(path)
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
        self.running = true;
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

    /// Execute multiple instructions
    ///
    /// Executes exactly `n` instructions unless an error occurs.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of instructions to execute
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all instructions executed successfully
    /// - `Err(EmulatorError)` if any instruction fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use echo_core::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.step_n(100).unwrap(); // Execute 100 instructions
    /// ```
    pub fn step_n(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    /// Execute one frame worth of instructions
    ///
    /// The PlayStation CPU runs at approximately 33.8688 MHz.
    /// At 60 fps, one frame requires approximately 564,480 cycles.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if frame executed successfully
    /// - `Err(EmulatorError)` if execution fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use echo_core::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.reset();
    /// system.run_frame().unwrap(); // Execute one frame
    /// ```
    pub fn run_frame(&mut self) -> Result<()> {
        // PSX CPU runs at ~33.8688 MHz
        // At 60 fps, one frame = 33868800 / 60 ≈ 564,480 cycles
        const CYCLES_PER_FRAME: u64 = 564_480;
        let target = self.cycles + CYCLES_PER_FRAME;

        while self.cycles < target && self.running {
            self.step()?;
        }

        Ok(())
    }

    /// Get current PC value
    ///
    /// # Returns
    /// Current program counter value
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::system::System;
    ///
    /// let system = System::new();
    /// assert_eq!(system.pc(), 0xBFC00000);
    /// ```
    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }

    /// Get total cycles executed
    ///
    /// # Returns
    /// Total number of cycles since reset
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::system::System;
    ///
    /// let system = System::new();
    /// assert_eq!(system.cycles(), 0);
    /// ```
    pub fn cycles(&self) -> u64 {
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
        assert_eq!(system.cycles(), 0);
        assert_eq!(system.pc(), 0xBFC00000);
    }

    #[test]
    fn test_system_step() {
        let mut system = System::new();

        // Write NOP instruction directly to BIOS memory for testing
        // NOP = 0x00000000
        system
            .bus_mut()
            .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        let initial_pc = system.pc();
        system.step().unwrap();

        assert_eq!(system.pc(), initial_pc + 4);
        assert_eq!(system.cycles(), 1);
    }

    #[test]
    fn test_system_step_n() {
        let mut system = System::new();

        // Fill BIOS with NOPs for testing
        for i in 0..10 {
            let offset = (i * 4) as usize;
            system
                .bus_mut()
                .write_bios_for_test(offset, &[0x00, 0x00, 0x00, 0x00]);
        }

        system.step_n(10).unwrap();

        assert_eq!(system.cycles(), 10);
    }

    #[test]
    fn test_system_reset() {
        let mut system = System::new();

        // Setup BIOS with NOP for testing
        system
            .bus_mut()
            .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        // Execute some instructions to change state
        system.step().unwrap();
        system.step().unwrap();

        assert!(system.cycles() > 0);

        system.reset();
        assert_eq!(system.cycles(), 0);
        assert_eq!(system.pc(), 0xBFC00000);
        assert!(system.running);
    }

    #[test]
    fn test_system_run_frame() {
        let mut system = System::new();

        // Create an infinite loop in BIOS for testing:
        // 0xBFC00000: j 0xBFC00000  (jump to self)
        // Encoding: opcode=2 (J), target=0x0F000000 (0xBFC00000 >> 2)
        // Full instruction: 0x0BF00000
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);

        // 0xBFC00004: nop (delay slot)
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();
        let initial_cycles = system.cycles();

        system.run_frame().unwrap();

        // Should execute approximately one frame worth of cycles (564,480)
        let cycles_executed = system.cycles() - initial_cycles;
        assert!(cycles_executed >= 564_480);
    }

    #[test]
    fn test_system_pc_accessor() {
        let system = System::new();
        assert_eq!(system.pc(), 0xBFC00000);
    }

    #[test]
    fn test_system_cycles_accessor() {
        let system = System::new();
        assert_eq!(system.cycles(), 0);
    }
}
