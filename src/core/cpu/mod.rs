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

use crate::core::error::Result;
use crate::core::memory::Bus;

/// CPU (MIPS R3000A) emulation implementation
///
/// # Specifications
/// - Architecture: MIPS I (32-bit)
/// - Clock frequency: 33.8688 MHz
/// - Registers: 32 general-purpose registers + special registers
///
/// # Example
/// ```
/// use psrx::core::cpu::CPU;
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

// Module declarations
mod cop0;
mod decode;
mod disassembler;
mod instructions;
#[cfg(test)]
mod tests;
mod tracer;

// Re-exports
pub use cop0::ExceptionCause;
use cop0::COP0;
pub use disassembler::Disassembler;
pub use tracer::CpuTracer;

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
    /// use psrx::core::cpu::CPU;
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
    /// use psrx::core::cpu::CPU;
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
    /// use psrx::core::cpu::CPU;
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
    /// use psrx::core::cpu::CPU;
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
    /// use psrx::core::cpu::CPU;
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

    /// Execute one instruction
    ///
    /// This is the main CPU execution step. It performs:
    /// 1. Load delay resolution
    /// 2. Instruction fetch from memory
    /// 3. PC update (with delay slot handling)
    /// 4. Instruction execution
    ///
    /// # Arguments
    ///
    /// * `bus` - Memory bus for reading instructions and data
    ///
    /// # Returns
    ///
    /// Number of cycles consumed (currently always 1)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cpu::CPU;
    /// use psrx::core::memory::Bus;
    ///
    /// let mut cpu = CPU::new();
    /// let mut bus = Bus::new();
    ///
    /// // Execute one instruction
    /// let cycles = cpu.step(&mut bus).unwrap();
    /// assert_eq!(cycles, 1);
    /// ```
    pub fn step(&mut self, bus: &mut Bus) -> Result<u32> {
        // The instruction fetched below will execute now. If we were in a delay slot,
        // clear the flag; any branch/jump executed in this step will set it again.
        let _was_in_delay = self.in_branch_delay;
        self.in_branch_delay = false;
        // Resolve load delay from previous instruction
        if let Some(delay) = self.load_delay.take() {
            self.set_reg(delay.reg, delay.value);
        }

        // Instruction fetch
        let pc = self.pc;
        self.current_instruction = bus.read32(pc)?;

        // Update PC (delay slot handling)
        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        // Execute instruction
        self.execute_instruction(bus)?;

        // For now, all instructions take 1 cycle
        Ok(1)
    }
    pub fn exception(&mut self, cause: ExceptionCause) {
        // Save current status (push exception level)
        let sr = self.cop0.regs[COP0::SR];
        let mode = sr & 0x3F;
        // Push KU/IE (c→p, p→o) and enter kernel with interrupts disabled.
        let mut new_sr = (sr & !0x3F) | ((mode << 2) & 0x3F);
        new_sr &= !0b11; // IEc=0 (bit 0), KUc=0 (bit 1)
        self.cop0.regs[COP0::SR] = new_sr;

        // Set exception cause
        let cause_reg = self.cop0.regs[COP0::CAUSE];
        self.cop0.regs[COP0::CAUSE] = (cause_reg & !0x7C) | ((cause as u32) << 2);

        // Save exception PC
        // self.pc currently points to (faulting_pc + 4). Adjust accordingly.
        let current_pc = self.pc.wrapping_sub(4);
        let epc = if self.in_branch_delay {
            current_pc.wrapping_sub(4) // branch instruction address
        } else {
            current_pc // faulting instruction address
        };
        self.cop0.regs[COP0::EPC] = epc;

        // Set branch delay flag in CAUSE if in delay slot
        if self.in_branch_delay {
            self.cop0.regs[COP0::CAUSE] |= 1 << 31;
        } else {
            self.cop0.regs[COP0::CAUSE] &= !(1 << 31);
        }

        // Jump to exception handler
        let handler = if (sr & (1 << 22)) != 0 {
            0xBFC00180 // BEV=1: Bootstrap exception vector
        } else {
            0x80000080 // BEV=0: Normal exception vector
        };

        // Log exception details
        log::warn!(
            "EXCEPTION: cause={:?}, EPC=0x{:08X}, handler=0x{:08X}, in_delay={}, instruction=0x{:08X}",
            cause,
            epc,
            handler,
            self.in_branch_delay,
            self.current_instruction
        );

        self.pc = handler;
        self.next_pc = handler.wrapping_add(4);
        self.in_branch_delay = false;
        self.load_delay = None;
    }

    /// Check if currently in branch delay slot
    ///
    /// # Returns
    ///
    /// true if the CPU is currently executing a branch delay slot instruction
    pub fn in_delay_slot(&self) -> bool {
        self.in_branch_delay
    }

    /// Get current PC value
    ///
    /// # Returns
    ///
    /// The current program counter value
    pub fn pc(&self) -> u32 {
        self.pc
    }

    /// Check for pending interrupts and trigger if enabled
    ///
    /// This method checks if interrupts are enabled in the Status Register
    /// and if there are any pending interrupts matching the interrupt mask.
    /// If both conditions are met, an interrupt exception is triggered.
    ///
    /// # Arguments
    ///
    /// * `interrupt_flags` - Pending interrupt flags (bits 0-7 correspond to interrupt sources)
    ///
    /// # Details
    ///
    /// The Status Register (SR) controls interrupt handling:
    /// - Bit 0 (IEc): Interrupt Enable (current)
    /// - Bits [15:8]: Interrupt Mask (IM)
    ///
    /// The CAUSE register stores pending interrupts in bits [15:8].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cpu::CPU;
    ///
    /// let mut cpu = CPU::new();
    /// // Enable interrupts and set interrupt mask
    /// // cpu.cop0.regs[12] |= 0x0401;  // Enable interrupts and mask bit 0
    ///
    /// // Check for pending interrupt 0
    /// cpu.check_interrupts(0x01);
    /// ```
    pub fn check_interrupts(&mut self, interrupt_flags: u32) {
        let sr = self.cop0.regs[COP0::SR];
        let ie = sr & 0x1; // Interrupt Enable (bit 0)
        let im = (sr >> 8) & 0xFF; // Interrupt Mask (bits [15:8])

        let pending_all = interrupt_flags & 0xFF;
        let cause = self.cop0.regs[COP0::CAUSE];
        self.cop0.regs[COP0::CAUSE] = (cause & !0xFF00) | (pending_all << 8);

        if ie != 0 {
            let masked = pending_all & im;
            if masked != 0 {
                self.exception(ExceptionCause::Interrupt);
            }
        }
    }

    /// Dump all CPU registers for debugging
    ///
    /// Prints a formatted dump of all CPU state including:
    /// - Program counter (PC) and next PC
    /// - HI and LO registers
    /// - All 32 general-purpose registers
    /// - COP0 status registers (SR, CAUSE, EPC)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cpu::CPU;
    ///
    /// let cpu = CPU::new();
    /// cpu.dump_registers(); // Print all register values
    /// ```
    pub fn dump_registers(&self) {
        println!("CPU Registers:");
        println!("PC: 0x{:08X}  Next PC: 0x{:08X}", self.pc, self.next_pc);
        println!("HI: 0x{:08X}  LO: 0x{:08X}", self.hi, self.lo);
        println!();

        // Print general-purpose registers in rows of 4
        for i in 0..32 {
            if i % 4 == 0 && i > 0 {
                println!();
            }
            print!("r{:2}: 0x{:08X}  ", i, self.reg(i));
        }
        println!("\n");

        // Print COP0 registers
        println!("COP0 Registers:");
        println!("SR:    0x{:08X}", self.cop0.regs[COP0::SR]);
        println!("CAUSE: 0x{:08X}", self.cop0.regs[COP0::CAUSE]);
        println!("EPC:   0x{:08X}", self.cop0.regs[COP0::EPC]);
        println!("BADA:  0x{:08X}", self.cop0.regs[COP0::BADA]);
        println!("PRID:  0x{:08X}", self.cop0.regs[COP0::PRID]);
    }
}
impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}
