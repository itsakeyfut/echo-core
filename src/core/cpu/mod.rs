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

/// CPU (MIPS R3000A) emulation implementation
///
/// # Specifications
/// - Architecture: MIPS I (32-bit)
/// - Clock frequency: 33.8688 MHz
/// - Registers: 32 general-purpose registers + special registers
///
/// # Example
/// ```
/// use echo_core::core::cpu::CPU;
///
/// let mut cpu = CPU::new();
/// cpu.reset();
/// assert_eq!(cpu.reg(0), 0); // r0 is always 0
/// ```
pub struct CPU {
    /// General purpose registers (r0-r31)
    ///
    /// r0 is hardwired to always return 0
    regs: [u32; 32],

    /// Program counter
    pc: u32,

    /// Next PC (for delay slot handling)
    next_pc: u32,

    /// HI register (multiplication/division result upper 32 bits)
    hi: u32,

    /// LO register (multiplication/division result lower 32 bits)
    lo: u32,

    /// Coprocessor 0 (System Control Unit)
    cop0: COP0,

    /// Load delay slot management
    ///
    /// On PSX, load instruction results cannot be used in the next instruction
    load_delay: Option<LoadDelay>,

    /// Branch delay slot flag
    in_branch_delay: bool,

    /// Current instruction (for debugging)
    current_instruction: u32,
}

/// Load delay management structure
///
/// The MIPS R3000A has a load delay slot - the result of a load instruction
/// cannot be used in the immediately following instruction. This structure
/// manages that delay.
#[derive(Debug, Clone, Copy)]
pub struct LoadDelay {
    /// Target register
    reg: u8,
    /// Value to load
    value: u32,
}

/// Coprocessor 0 (System Control)
///
/// COP0 is the system control unit responsible for exception handling,
/// status management, cache control, and other system functions.
pub struct COP0 {
    /// COP0 registers (32 registers)
    regs: [u32; 32],
}

impl COP0 {
    /// Breakpoint PC
    pub const BPC: usize = 3;
    /// Breakpoint Data Address
    pub const BDA: usize = 5;
    /// Target Address
    pub const TAR: usize = 6;
    /// Cache control
    pub const DCIC: usize = 7;
    /// Bad Virtual Address
    pub const BADA: usize = 8;
    /// Data Address Mask
    pub const BDAM: usize = 9;
    /// PC Mask
    pub const BPCM: usize = 11;
    /// Status Register
    pub const SR: usize = 12;
    /// Cause Register
    pub const CAUSE: usize = 13;
    /// Exception PC
    pub const EPC: usize = 14;
    /// Processor ID
    pub const PRID: usize = 15;

    /// Create a new COP0 instance
    ///
    /// # Returns
    /// Initialized COP0 instance with reset values
    fn new() -> Self {
        let mut regs = [0u32; 32];
        // Status Register initial value
        regs[Self::SR] = 0x10900000;
        // Processor ID (R3000A identifier)
        regs[Self::PRID] = 0x00000002;

        Self { regs }
    }

    /// Reset COP0 registers to initial state
    fn reset(&mut self) {
        self.regs = [0u32; 32];
        self.regs[Self::SR] = 0x10900000;
        self.regs[Self::PRID] = 0x00000002;
    }
}

impl CPU {
    /// Create a new CPU instance with initial state
    ///
    /// The CPU is initialized with the following state:
    /// - All general purpose registers: 0
    /// - PC: 0xBFC00000 (BIOS entry point)
    /// - next_pc: 0xBFC00004
    /// - COP0 SR: 0x10900000
    /// - COP0 PRID: 0x00000002
    ///
    /// # Returns
    /// Initialized CPU instance
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::CPU;
    ///
    /// let cpu = CPU::new();
    /// assert_eq!(cpu.reg(0), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            regs: [0u32; 32],
            pc: 0xBFC00000,      // BIOS entry point
            next_pc: 0xBFC00004, // Next instruction
            hi: 0,
            lo: 0,
            cop0: COP0::new(),
            load_delay: None,
            in_branch_delay: false,
            current_instruction: 0,
        }
    }

    /// Reset CPU to initial state
    ///
    /// Resets all registers and state to initial values.
    /// This mimics the behavior of power-on or hardware reset.
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::CPU;
    ///
    /// let mut cpu = CPU::new();
    /// // ... use CPU ...
    /// cpu.reset(); // Return to initial state
    /// ```
    pub fn reset(&mut self) {
        self.regs = [0u32; 32];
        self.pc = 0xBFC00000;
        self.next_pc = 0xBFC00004;
        self.hi = 0;
        self.lo = 0;
        self.cop0.reset();
        self.load_delay = None;
        self.in_branch_delay = false;
        self.current_instruction = 0;
    }

    /// Read from general purpose register
    ///
    /// # Arguments
    /// - `index`: Register number (0-31)
    ///
    /// # Returns
    /// Register value. r0 always returns 0.
    ///
    /// # Note
    /// r0 is hardwired to always return 0.
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::CPU;
    ///
    /// let cpu = CPU::new();
    /// let value = cpu.reg(1);  // Get r1 value
    /// assert_eq!(cpu.reg(0), 0); // r0 is always 0
    /// ```
    #[inline(always)]
    pub fn reg(&self, index: u8) -> u32 {
        if index == 0 {
            0
        } else {
            self.regs[index as usize]
        }
    }

    /// Write to general purpose register
    ///
    /// # Arguments
    /// - `index`: Register number (0-31)
    /// - `value`: Value to write
    ///
    /// # Note
    /// Writes to r0 are ignored (r0 is always 0).
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::CPU;
    ///
    /// let mut cpu = CPU::new();
    /// cpu.set_reg(1, 0x12345678);
    /// assert_eq!(cpu.reg(1), 0x12345678);
    ///
    /// // Writes to r0 are ignored
    /// cpu.set_reg(0, 0xDEADBEEF);
    /// assert_eq!(cpu.reg(0), 0);
    /// ```
    #[inline(always)]
    pub fn set_reg(&mut self, index: u8, value: u32) {
        if index != 0 {
            self.regs[index as usize] = value;
        }
    }

    /// Write to register with load delay
    ///
    /// The MIPS R3000A has a load delay slot - the result of a load instruction
    /// cannot be used in the immediately following instruction.
    /// This method manages the load delay slot.
    ///
    /// # Behavior
    /// 1. Execute current load delay if present
    /// 2. Set new load delay
    ///
    /// # Arguments
    /// - `index`: Target register number (0-31)
    /// - `value`: Value to load
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::CPU;
    ///
    /// let mut cpu = CPU::new();
    /// cpu.set_reg_delayed(3, 100);
    /// // At this point, r3 does not yet have the value
    /// assert_eq!(cpu.reg(3), 0);
    ///
    /// // The next load delay instruction executes the previous delay
    /// cpu.set_reg_delayed(4, 200);
    /// assert_eq!(cpu.reg(3), 100);
    /// ```
    pub fn set_reg_delayed(&mut self, index: u8, value: u32) {
        // Execute current load delay
        if let Some(delay) = self.load_delay.take() {
            self.set_reg(delay.reg, delay.value);
        }

        // Set new load delay
        if index != 0 {
            self.load_delay = Some(LoadDelay { reg: index, value });
        }
    }
}

impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_initialization() {
        let cpu = CPU::new();
        assert_eq!(cpu.pc, 0xBFC00000);
        assert_eq!(cpu.next_pc, 0xBFC00004);
        assert_eq!(cpu.reg(0), 0);
    }

    #[test]
    fn test_register_r0_is_hardwired() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0xDEADBEEF);
        assert_eq!(cpu.reg(0), 0);
    }

    #[test]
    fn test_register_read_write() {
        let mut cpu = CPU::new();
        cpu.set_reg(5, 0x12345678);
        assert_eq!(cpu.reg(5), 0x12345678);
    }

    #[test]
    fn test_load_delay_slot() {
        let mut cpu = CPU::new();
        cpu.set_reg_delayed(3, 100);

        // Value not yet visible
        assert_eq!(cpu.reg(3), 0);

        // Execute load delay
        cpu.set_reg_delayed(4, 200);

        // Now r3 should have the value
        assert_eq!(cpu.reg(3), 100);
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = CPU::new();

        // Modify some state
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.pc = 0x80000000;
        cpu.hi = 0x12345678;
        cpu.lo = 0x87654321;

        // Reset
        cpu.reset();

        // Verify all state is reset
        assert_eq!(cpu.reg(1), 0);
        assert_eq!(cpu.pc, 0xBFC00000);
        assert_eq!(cpu.next_pc, 0xBFC00004);
        assert_eq!(cpu.hi, 0);
        assert_eq!(cpu.lo, 0);
    }

    #[test]
    fn test_cop0_initialization() {
        let cpu = CPU::new();
        assert_eq!(cpu.cop0.regs[COP0::SR], 0x10900000);
        assert_eq!(cpu.cop0.regs[COP0::PRID], 0x00000002);
    }

    #[test]
    fn test_multiple_registers() {
        let mut cpu = CPU::new();

        // Test writing to multiple registers
        for i in 1..32 {
            cpu.set_reg(i, i as u32 * 100);
        }

        // Verify all values
        for i in 1..32 {
            assert_eq!(cpu.reg(i), i as u32 * 100);
        }

        // r0 should still be 0
        assert_eq!(cpu.reg(0), 0);
    }

    #[test]
    fn test_load_delay_chain() {
        let mut cpu = CPU::new();

        // Chain multiple load delays
        cpu.set_reg_delayed(1, 10);
        assert_eq!(cpu.reg(1), 0);

        cpu.set_reg_delayed(2, 20);
        assert_eq!(cpu.reg(1), 10);
        assert_eq!(cpu.reg(2), 0);

        cpu.set_reg_delayed(3, 30);
        assert_eq!(cpu.reg(1), 10);
        assert_eq!(cpu.reg(2), 20);
        assert_eq!(cpu.reg(3), 0);

        // Final load delay to flush
        cpu.set_reg_delayed(4, 40);
        assert_eq!(cpu.reg(1), 10);
        assert_eq!(cpu.reg(2), 20);
        assert_eq!(cpu.reg(3), 30);
        assert_eq!(cpu.reg(4), 0);
    }

    #[test]
    fn test_load_delay_r0_ignored() {
        let mut cpu = CPU::new();

        // Load delay to r0 should be ignored
        cpu.set_reg_delayed(0, 100);
        cpu.set_reg_delayed(1, 200);

        // r0 should still be 0, r1 should be 0 (delay not executed yet)
        assert_eq!(cpu.reg(0), 0);
        assert_eq!(cpu.reg(1), 0);

        // Execute another load to flush
        cpu.set_reg_delayed(2, 300);
        assert_eq!(cpu.reg(0), 0);
        assert_eq!(cpu.reg(1), 200);
    }
}
