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
    /// use echo_core::core::cpu::CPU;
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut cpu = CPU::new();
    /// let mut bus = Bus::new();
    ///
    /// // Execute one instruction
    /// let cycles = cpu.step(&mut bus).unwrap();
    /// assert_eq!(cycles, 1);
    /// ```
    pub fn step(&mut self, bus: &mut Bus) -> Result<u32> {
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

    /// Decode and execute the current instruction
    ///
    /// This method dispatches the instruction to the appropriate handler
    /// based on its opcode (upper 6 bits).
    ///
    /// # Arguments
    ///
    /// * `bus` - Memory bus for memory operations
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if execution fails
    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<()> {
        let instruction = self.current_instruction;

        // Extract opcode (upper 6 bits)
        let opcode = instruction >> 26;

        match opcode {
            0x00 => self.execute_special(instruction, bus),
            0x01 => self.execute_bcondz(instruction),
            0x02 => self.op_j(instruction),   // J
            0x03 => self.op_jal(instruction), // JAL
            0x0F => self.op_lui(instruction), // LUI
            _ => {
                log::warn!(
                    "Unimplemented opcode: 0x{:02X} at PC=0x{:08X}",
                    opcode,
                    self.pc
                );
                Ok(())
            }
        }
    }

    /// Handle SPECIAL instructions (opcode 0x00)
    ///
    /// SPECIAL instructions use the lower 6 bits (funct field) to determine
    /// the specific operation.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus (for future use)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn execute_special(&mut self, instruction: u32, _bus: &mut Bus) -> Result<()> {
        let (_rs, rt, rd, shamt, funct) = decode_r_type(instruction);

        match funct {
            0x00 => self.op_sll(rt, rd, shamt),
            _ => {
                log::warn!(
                    "Unimplemented SPECIAL function: 0x{:02X} at PC=0x{:08X}",
                    funct,
                    self.pc
                );
                Ok(())
            }
        }
    }

    /// Handle BCONDZ instructions (opcode 0x01)
    ///
    /// BCONDZ instructions include BLTZ, BGEZ, BLTZAL, and BGEZAL.
    /// The rt field determines which specific branch instruction it is.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn execute_bcondz(&mut self, _instruction: u32) -> Result<()> {
        // Stub implementation for Week 1
        // Will be implemented in future issues
        log::warn!("BCONDZ instruction not yet implemented at PC=0x{:08X}", self.pc);
        Ok(())
    }

    /// LUI: Load Upper Immediate
    ///
    /// Loads a 16-bit immediate value into the upper 16 bits of a register,
    /// setting the lower 16 bits to 0.
    ///
    /// Format: lui rt, imm
    /// Operation: rt = imm << 16
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_lui(&mut self, instruction: u32) -> Result<()> {
        let (_, _, rt, imm) = decode_i_type(instruction);
        let value = (imm as u32) << 16;
        self.set_reg(rt, value);
        Ok(())
    }

    /// SLL: Shift Left Logical
    ///
    /// Shifts the value in rt left by shamt bits, storing the result in rd.
    /// Note: SLL with all fields = 0 is NOP.
    ///
    /// Format: sll rd, rt, shamt
    /// Operation: rd = rt << shamt
    ///
    /// # Arguments
    ///
    /// * `rt` - Source register
    /// * `rd` - Destination register
    /// * `shamt` - Shift amount (0-31)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sll(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
        let value = self.reg(rt) << shamt;
        self.set_reg(rd, value);
        Ok(())
    }

    /// J: Jump
    ///
    /// Unconditional jump to target address.
    /// The target address is formed by combining the upper 4 bits of PC
    /// with the 26-bit target field shifted left by 2.
    ///
    /// Format: j target
    /// Operation: PC = (PC & 0xF0000000) | (target << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_j(&mut self, instruction: u32) -> Result<()> {
        let (_, target) = decode_j_type(instruction);
        // Upper 4 bits of PC + target << 2
        let pc_high = self.pc & 0xF0000000;
        self.next_pc = pc_high | (target << 2);
        Ok(())
    }

    /// JAL: Jump and Link
    ///
    /// Unconditional jump to target address, saving return address in r31.
    /// The return address is the address of the instruction after the delay slot.
    ///
    /// Format: jal target
    /// Operation: r31 = PC + 8; PC = (PC & 0xF0000000) | (target << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_jal(&mut self, instruction: u32) -> Result<()> {
        let (_, target) = decode_j_type(instruction);
        // Save return address to r31 (next_pc already points to delay slot + 4)
        self.set_reg(31, self.next_pc);

        let pc_high = self.pc & 0xF0000000;
        self.next_pc = pc_high | (target << 2);
        Ok(())
    }

    /// Execute a branch (sets next_pc)
    ///
    /// This helper method is used by branch instructions to update the PC.
    /// The offset is relative to the address of the delay slot.
    ///
    /// # Arguments
    ///
    /// * `offset` - Branch offset in bytes (should be pre-shifted)
    ///
    /// # Note
    ///
    /// The offset is added to next_pc, which already points to the delay slot + 4.
    /// This correctly implements the MIPS branch semantics.
    fn branch(&mut self, offset: i32) {
        // next_pc already points to delay slot + 4, so add offset from there
        self.next_pc = self.next_pc.wrapping_add(offset as u32);
        self.in_branch_delay = true;
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
}

/// Decode R-type instruction
///
/// R-type instructions are used for register-to-register operations.
///
/// Format: | op (6) | rs (5) | rt (5) | rd (5) | shamt (5) | funct (6) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (rs, rt, rd, shamt, funct)
#[inline(always)]
fn decode_r_type(instr: u32) -> (u8, u8, u8, u8, u8) {
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let rd = ((instr >> 11) & 0x1F) as u8;
    let shamt = ((instr >> 6) & 0x1F) as u8;
    let funct = (instr & 0x3F) as u8;
    (rs, rt, rd, shamt, funct)
}

/// Decode I-type instruction
///
/// I-type instructions are used for immediate operations, loads, stores, and branches.
///
/// Format: | op (6) | rs (5) | rt (5) | immediate (16) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (op, rs, rt, imm)
#[inline(always)]
fn decode_i_type(instr: u32) -> (u8, u8, u8, u16) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let imm = (instr & 0xFFFF) as u16;
    (op, rs, rt, imm)
}

/// Decode J-type instruction
///
/// J-type instructions are used for jump operations.
///
/// Format: | op (6) | target (26) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (op, target)
#[inline(always)]
fn decode_j_type(instr: u32) -> (u8, u32) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let target = instr & 0x03FFFFFF;
    (op, target)
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

    // === Instruction Decode Tests ===

    #[test]
    fn test_decode_r_type() {
        use super::decode_r_type;

        // ADD r3, r1, r2 -> 0x00221820
        let instr = 0x00221820;
        let (rs, rt, rd, shamt, funct) = decode_r_type(instr);
        assert_eq!(rs, 1);
        assert_eq!(rt, 2);
        assert_eq!(rd, 3);
        assert_eq!(shamt, 0);
        assert_eq!(funct, 0x20);
    }

    #[test]
    fn test_decode_i_type() {
        use super::decode_i_type;

        // ADDI r2, r1, 100 -> 0x20220064
        let instr = 0x20220064;
        let (op, rs, rt, imm) = decode_i_type(instr);
        assert_eq!(op, 0x08);
        assert_eq!(rs, 1);
        assert_eq!(rt, 2);
        assert_eq!(imm, 100);
    }

    #[test]
    fn test_decode_j_type() {
        use super::decode_j_type;

        // J 0x100000 -> 0x08040000
        let instr = 0x08040000;
        let (op, target) = decode_j_type(instr);
        assert_eq!(op, 0x02);
        assert_eq!(target, 0x040000);
    }

    // === Instruction Execution Tests ===

    #[test]
    fn test_instruction_fetch() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM instead of BIOS
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // Place a NOP in RAM
        bus.write32(0x80000000, 0x00000000).unwrap();

        let cycles = cpu.step(&mut bus).unwrap();
        assert_eq!(cycles, 1);
        assert_eq!(cpu.pc, 0x80000004);
    }

    #[test]
    fn test_lui_instruction() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // LUI r5, 0x1234 -> 0x3C051234
        bus.write32(0x80000000, 0x3C051234).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(5), 0x12340000);
    }

    #[test]
    fn test_sll_instruction() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // Set up r1 with a value to shift
        cpu.set_reg(1, 0x00000001);

        // SLL r2, r1, 4 -> Shift r1 left by 4, store in r2
        // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=4, funct=0
        // = (1 << 16) | (2 << 11) | (4 << 6) = 0x00011100
        bus.write32(0x80000000, 0x00011100).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(2), 0x00000010); // 1 << 4 = 16
    }

    #[test]
    fn test_nop_instruction() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // Set up some registers
        cpu.set_reg(1, 0x12345678);
        cpu.set_reg(2, 0xABCDEF00);

        // NOP -> 0x00000000 (SLL with all fields = 0)
        bus.write32(0x80000000, 0x00000000).unwrap();

        cpu.step(&mut bus).unwrap();

        // All registers should be unchanged
        assert_eq!(cpu.reg(1), 0x12345678);
        assert_eq!(cpu.reg(2), 0xABCDEF00);
    }

    #[test]
    fn test_pc_increment() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        let initial_pc = cpu.pc;

        // Execute NOP
        bus.write32(initial_pc, 0x00000000).unwrap();
        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.pc, initial_pc + 4);
        assert_eq!(cpu.next_pc, initial_pc + 8);
    }

    #[test]
    fn test_delay_slot_pc_handling() {
        let mut cpu = CPU::new();

        // Simulate branch
        cpu.branch(100); // Branch forward by 100 bytes

        // Verify next_pc is updated
        let expected_target = cpu.next_pc;
        assert!(expected_target != cpu.pc + 4);
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_j_instruction() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // J 0x100000 -> 0x08040000
        // Target address = (0x80000000 & 0xF0000000) | (0x040000 << 2)
        //                = 0x80000000 | 0x00100000 = 0x80100000
        bus.write32(0x80000000, 0x08040000).unwrap();

        cpu.step(&mut bus).unwrap();

        // PC should be updated to point after the delay slot
        // next_pc should be the jump target
        assert_eq!(cpu.next_pc, 0x80100000);
    }

    #[test]
    fn test_jal_instruction() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // JAL 0x100000 -> 0x0C040000
        bus.write32(0x80000000, 0x0C040000).unwrap();

        let initial_pc = cpu.pc;
        cpu.step(&mut bus).unwrap();

        // r31 should contain return address (address after delay slot)
        assert_eq!(cpu.reg(31), initial_pc + 8);

        // next_pc should be the jump target
        assert_eq!(cpu.next_pc, 0x80100000);
    }

    #[test]
    fn test_multiple_instructions() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // LUI r1, 0x1234
        bus.write32(0x80000000, 0x3C011234).unwrap();
        // NOP
        bus.write32(0x80000004, 0x00000000).unwrap();
        // LUI r2, 0x5678
        bus.write32(0x80000008, 0x3C025678).unwrap();

        // Execute first instruction
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.reg(1), 0x12340000);
        assert_eq!(cpu.pc, 0x80000004);

        // Execute second instruction (NOP)
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.pc, 0x80000008);

        // Execute third instruction
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.reg(2), 0x56780000);
        assert_eq!(cpu.pc, 0x8000000C);
    }

    #[test]
    fn test_branch_helper() {
        let mut cpu = CPU::new();

        let initial_next_pc = cpu.next_pc;

        // Branch forward by 100 bytes
        cpu.branch(100);

        assert_eq!(cpu.next_pc, initial_next_pc.wrapping_add(100));
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_branch_backward() {
        let mut cpu = CPU::new();

        let initial_next_pc = cpu.next_pc;

        // Branch backward by 100 bytes
        cpu.branch(-100);

        assert_eq!(cpu.next_pc, initial_next_pc.wrapping_sub(100));
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_pc_accessor() {
        let cpu = CPU::new();
        assert_eq!(cpu.pc(), 0xBFC00000);
    }

    #[test]
    fn test_sll_zero_shift() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        cpu.set_reg(1, 0x12345678);

        // SLL r2, r1, 0 -> Should copy r1 to r2
        // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=0, funct=0
        // = (1 << 16) | (2 << 11) = 0x00011000
        bus.write32(0x80000000, 0x00011000).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(2), 0x12345678);
    }

    #[test]
    fn test_sll_max_shift() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set CPU to execute from RAM
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        cpu.set_reg(1, 0xFFFFFFFF);

        // SLL r2, r1, 31 -> Shift left by 31 bits
        // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=31, funct=0
        // = (1 << 16) | (2 << 11) | (31 << 6) = 0x000117C0
        bus.write32(0x80000000, 0x000117C0).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(2), 0x80000000);
    }

    #[test]
    fn test_instruction_at_different_pc() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set PC to RAM instead of BIOS
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;

        // LUI r3, 0xABCD
        bus.write32(0x80000000, 0x3C03ABCD).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(3), 0xABCD0000);
        assert_eq!(cpu.pc, 0x80000004);
    }
}
