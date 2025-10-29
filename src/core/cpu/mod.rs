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

/// Exception cause codes for MIPS R3000A
///
/// These correspond to the exception codes stored in the CAUSE register
/// when a CPU exception occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExceptionCause {
    /// Interrupt (external or internal)
    Interrupt = 0,
    /// Address error on load
    AddressErrorLoad = 4,
    /// Address error on store
    AddressErrorStore = 5,
    /// Bus error on instruction fetch
    BusErrorInstruction = 6,
    /// Bus error on data access
    BusErrorData = 7,
    /// Syscall instruction executed
    Syscall = 8,
    /// Breakpoint instruction executed
    Breakpoint = 9,
    /// Reserved or illegal instruction
    ReservedInstruction = 10,
    /// Coprocessor unusable
    CoprocessorUnusable = 11,
    /// Arithmetic overflow
    Overflow = 12,
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
            0x02 => self.op_j(instruction),        // J
            0x03 => self.op_jal(instruction),      // JAL
            0x04 => self.op_beq(instruction),      // BEQ
            0x05 => self.op_bne(instruction),      // BNE
            0x06 => self.op_blez(instruction),     // BLEZ
            0x07 => self.op_bgtz(instruction),     // BGTZ
            0x08 => self.op_addi(instruction),     // ADDI
            0x09 => self.op_addiu(instruction),    // ADDIU
            0x0A => self.op_slti(instruction),     // SLTI
            0x0B => self.op_sltiu(instruction),    // SLTIU
            0x0C => self.op_andi(instruction),     // ANDI
            0x0D => self.op_ori(instruction),      // ORI
            0x0E => self.op_xori(instruction),     // XORI
            0x0F => self.op_lui(instruction),      // LUI
            0x20 => self.op_lb(instruction, bus),  // LB
            0x21 => self.op_lh(instruction, bus),  // LH
            0x22 => self.op_lwl(instruction, bus), // LWL
            0x23 => self.op_lw(instruction, bus),  // LW
            0x24 => self.op_lbu(instruction, bus), // LBU
            0x25 => self.op_lhu(instruction, bus), // LHU
            0x26 => self.op_lwr(instruction, bus), // LWR
            0x28 => self.op_sb(instruction, bus),  // SB
            0x29 => self.op_sh(instruction, bus),  // SH
            0x2A => self.op_swl(instruction, bus), // SWL
            0x2B => self.op_sw(instruction, bus),  // SW
            0x2E => self.op_swr(instruction, bus), // SWR
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
        let (rs, rt, rd, shamt, funct) = decode_r_type(instruction);

        match funct {
            0x00 => self.op_sll(rt, rd, shamt), // SLL
            0x02 => self.op_srl(rt, rd, shamt), // SRL
            0x03 => self.op_sra(rt, rd, shamt), // SRA
            0x04 => self.op_sllv(rs, rt, rd),   // SLLV
            0x06 => self.op_srlv(rs, rt, rd),   // SRLV
            0x07 => self.op_srav(rs, rt, rd),   // SRAV
            0x08 => self.op_jr(rs),             // JR
            0x09 => self.op_jalr(rs, rd),       // JALR
            0x20 => self.op_add(rs, rt, rd),    // ADD
            0x21 => self.op_addu(rs, rt, rd),   // ADDU
            0x22 => self.op_sub(rs, rt, rd),    // SUB
            0x23 => self.op_subu(rs, rt, rd),   // SUBU
            0x24 => self.op_and(rs, rt, rd),    // AND
            0x25 => self.op_or(rs, rt, rd),     // OR
            0x26 => self.op_xor(rs, rt, rd),    // XOR
            0x27 => self.op_nor(rs, rt, rd),    // NOR
            0x2A => self.op_slt(rs, rt, rd),    // SLT
            0x2B => self.op_sltu(rs, rt, rd),   // SLTU
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
    fn execute_bcondz(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        // rt field determines the specific instruction
        // Bit 0: BGEZ (1) vs BLTZ (0)
        // Bit 4: link (1) vs no link (0)
        let is_bgez = (rt & 0x01) != 0;
        let is_link = (rt & 0x10) != 0;

        let test = (self.reg(rs) as i32) >= 0;
        let should_branch = if is_bgez { test } else { !test };

        if is_link {
            // Save return address (BLTZAL or BGEZAL)
            self.set_reg(31, self.next_pc);
        }

        if should_branch {
            self.branch(offset);
        }

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
        self.in_branch_delay = true;
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
        self.in_branch_delay = true;
        Ok(())
    }

    /// JR: Jump Register
    ///
    /// Unconditional jump to address in register.
    /// Used for function returns and indirect jumps.
    ///
    /// Format: jr rs
    /// Operation: PC = rs
    ///
    /// # Arguments
    ///
    /// * `rs` - Source register containing target address
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_jr(&mut self, rs: u8) -> Result<()> {
        self.next_pc = self.reg(rs);
        self.in_branch_delay = true;
        Ok(())
    }

    /// JALR: Jump And Link Register
    ///
    /// Unconditional jump to address in register, saving return address.
    /// The return address is saved to register rd (typically r31).
    ///
    /// Format: jalr rs, rd
    /// Operation: rd = PC + 8; PC = rs
    ///
    /// # Arguments
    ///
    /// * `rs` - Source register containing target address
    /// * `rd` - Destination register for return address (default r31 if 0)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_jalr(&mut self, rs: u8, rd: u8) -> Result<()> {
        // Save return address (next_pc already points to delay slot + 4)
        self.set_reg(rd, self.next_pc);
        // Jump to address in rs
        self.next_pc = self.reg(rs);
        self.in_branch_delay = true;
        Ok(())
    }

    // === Branch Instructions ===

    /// BEQ: Branch on Equal
    ///
    /// Conditional branch if two registers are equal.
    /// The branch target is PC + 4 + (offset << 2).
    ///
    /// Format: beq rs, rt, offset
    /// Operation: if (rs == rt) PC = PC + 4 + (sign_extend(offset) << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_beq(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if self.reg(rs) == self.reg(rt) {
            self.branch(offset);
        }
        Ok(())
    }

    /// BNE: Branch on Not Equal
    ///
    /// Conditional branch if two registers are not equal.
    ///
    /// Format: bne rs, rt, offset
    /// Operation: if (rs != rt) PC = PC + 4 + (sign_extend(offset) << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_bne(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if self.reg(rs) != self.reg(rt) {
            self.branch(offset);
        }
        Ok(())
    }

    /// BLEZ: Branch on Less Than or Equal to Zero
    ///
    /// Conditional branch if register is less than or equal to zero (signed).
    ///
    /// Format: blez rs, offset
    /// Operation: if (rs <= 0) PC = PC + 4 + (sign_extend(offset) << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_blez(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, _, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if (self.reg(rs) as i32) <= 0 {
            self.branch(offset);
        }
        Ok(())
    }

    /// BGTZ: Branch on Greater Than Zero
    ///
    /// Conditional branch if register is greater than zero (signed).
    ///
    /// Format: bgtz rs, offset
    /// Operation: if (rs > 0) PC = PC + 4 + (sign_extend(offset) << 2)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_bgtz(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, _, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if (self.reg(rs) as i32) > 0 {
            self.branch(offset);
        }
        Ok(())
    }

    // === Arithmetic Instructions ===

    /// ADD: Add (with overflow exception)
    ///
    /// Adds two registers with signed overflow detection.
    /// If overflow occurs, triggers an Overflow exception.
    ///
    /// Format: add rd, rs, rt
    /// Operation: rd = rs + rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success (exception is triggered internally on overflow)
    fn op_add(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs) as i32;
        let b = self.reg(rt) as i32;

        match a.checked_add(b) {
            Some(result) => {
                self.set_reg(rd, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }

    /// ADDU: Add Unsigned (no overflow exception)
    ///
    /// Adds two registers without overflow detection.
    /// Overflow wraps around (modulo 2^32).
    ///
    /// Format: addu rd, rs, rt
    /// Operation: rd = rs + rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_addu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = self.reg(rs).wrapping_add(self.reg(rt));
        self.set_reg(rd, result);
        Ok(())
    }

    /// ADDI: Add Immediate (with overflow exception)
    ///
    /// Adds a sign-extended immediate value to a register with overflow detection.
    ///
    /// Format: addi rt, rs, imm
    /// Operation: rt = rs + sign_extend(imm)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_addi(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as i32; // Sign extend
        let a = self.reg(rs) as i32;

        match a.checked_add(imm) {
            Some(result) => {
                self.set_reg(rt, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }

    /// ADDIU: Add Immediate Unsigned (no overflow exception)
    ///
    /// Adds a sign-extended immediate value to a register without overflow detection.
    /// Despite the name "unsigned", the immediate is sign-extended.
    ///
    /// Format: addiu rt, rs, imm
    /// Operation: rt = rs + sign_extend(imm)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_addiu(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as u32; // Sign extend
        let result = self.reg(rs).wrapping_add(imm);
        self.set_reg(rt, result);
        Ok(())
    }

    /// SUB: Subtract (with overflow exception)
    ///
    /// Subtracts two registers with signed overflow detection.
    ///
    /// Format: sub rd, rs, rt
    /// Operation: rd = rs - rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register (minuend)
    /// * `rt` - Second source register (subtrahend)
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sub(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs) as i32;
        let b = self.reg(rt) as i32;

        match a.checked_sub(b) {
            Some(result) => {
                self.set_reg(rd, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }

    /// SUBU: Subtract Unsigned (no overflow exception)
    ///
    /// Subtracts two registers without overflow detection.
    ///
    /// Format: subu rd, rs, rt
    /// Operation: rd = rs - rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register (minuend)
    /// * `rt` - Second source register (subtrahend)
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_subu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = self.reg(rs).wrapping_sub(self.reg(rt));
        self.set_reg(rd, result);
        Ok(())
    }

    /// SLT: Set on Less Than (signed)
    ///
    /// Compares two registers as signed integers.
    /// Sets rd to 1 if rs < rt, otherwise 0.
    ///
    /// Format: slt rd, rs, rt
    /// Operation: rd = (rs < rt) ? 1 : 0
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_slt(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs) as i32;
        let b = self.reg(rt) as i32;
        let result = if a < b { 1 } else { 0 };
        self.set_reg(rd, result);
        Ok(())
    }

    /// SLTU: Set on Less Than Unsigned
    ///
    /// Compares two registers as unsigned integers.
    /// Sets rd to 1 if rs < rt, otherwise 0.
    ///
    /// Format: sltu rd, rs, rt
    /// Operation: rd = (rs < rt) ? 1 : 0
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sltu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs);
        let b = self.reg(rt);
        let result = if a < b { 1 } else { 0 };
        self.set_reg(rd, result);
        Ok(())
    }

    /// SLTI: Set on Less Than Immediate (signed)
    ///
    /// Compares a register with a sign-extended immediate as signed integers.
    /// Sets rt to 1 if rs < imm, otherwise 0.
    ///
    /// Format: slti rt, rs, imm
    /// Operation: rt = (rs < sign_extend(imm)) ? 1 : 0
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_slti(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as i32;
        let a = self.reg(rs) as i32;
        let result = if a < imm { 1 } else { 0 };
        self.set_reg(rt, result);
        Ok(())
    }

    /// SLTIU: Set on Less Than Immediate Unsigned
    ///
    /// Compares a register with a sign-extended immediate as unsigned integers.
    /// Despite the name, the immediate is sign-extended before comparison.
    /// Sets rt to 1 if rs < imm, otherwise 0.
    ///
    /// Format: sltiu rt, rs, imm
    /// Operation: rt = (rs < sign_extend(imm)) ? 1 : 0
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sltiu(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as u32; // Sign extend then treat as unsigned
        let a = self.reg(rs);
        let result = if a < imm { 1 } else { 0 };
        self.set_reg(rt, result);
        Ok(())
    }

    // === Logical Instructions ===

    /// AND: Bitwise AND
    ///
    /// Performs bitwise AND operation on two registers.
    ///
    /// Format: and rd, rs, rt
    /// Operation: rd = rs & rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_and(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = self.reg(rs) & self.reg(rt);
        self.set_reg(rd, result);
        Ok(())
    }

    /// ANDI: AND Immediate (zero-extended)
    ///
    /// Performs bitwise AND operation with a zero-extended immediate value.
    /// Note: Unlike ADDI, the immediate is ZERO-extended, not sign-extended.
    ///
    /// Format: andi rt, rs, imm
    /// Operation: rt = rs & zero_extend(imm)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_andi(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let result = self.reg(rs) & (imm as u32); // Zero extend
        self.set_reg(rt, result);
        Ok(())
    }

    /// OR: Bitwise OR
    ///
    /// Performs bitwise OR operation on two registers.
    ///
    /// Format: or rd, rs, rt
    /// Operation: rd = rs | rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_or(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = self.reg(rs) | self.reg(rt);
        self.set_reg(rd, result);
        Ok(())
    }

    /// ORI: OR Immediate (zero-extended)
    ///
    /// Performs bitwise OR operation with a zero-extended immediate value.
    ///
    /// Format: ori rt, rs, imm
    /// Operation: rt = rs | zero_extend(imm)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_ori(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let result = self.reg(rs) | (imm as u32);
        self.set_reg(rt, result);
        Ok(())
    }

    /// XOR: Bitwise XOR
    ///
    /// Performs bitwise XOR operation on two registers.
    ///
    /// Format: xor rd, rs, rt
    /// Operation: rd = rs ^ rt
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_xor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = self.reg(rs) ^ self.reg(rt);
        self.set_reg(rd, result);
        Ok(())
    }

    /// XORI: XOR Immediate (zero-extended)
    ///
    /// Performs bitwise XOR operation with a zero-extended immediate value.
    ///
    /// Format: xori rt, rs, imm
    /// Operation: rt = rs ^ zero_extend(imm)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_xori(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let result = self.reg(rs) ^ (imm as u32);
        self.set_reg(rt, result);
        Ok(())
    }

    /// NOR: Bitwise NOR (NOT OR)
    ///
    /// Performs bitwise NOR operation on two registers.
    /// This is equivalent to NOT(rs OR rt).
    ///
    /// Format: nor rd, rs, rt
    /// Operation: rd = ~(rs | rt)
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_nor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = !(self.reg(rs) | self.reg(rt));
        self.set_reg(rd, result);
        Ok(())
    }

    // === Shift Instructions ===

    /// SRL: Shift Right Logical (zero-fill)
    ///
    /// Shifts the value in rt right by shamt bits, filling with zeros.
    ///
    /// Format: srl rd, rt, shamt
    /// Operation: rd = rt >> shamt (zero-fill)
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
    fn op_srl(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
        let result = self.reg(rt) >> shamt;
        self.set_reg(rd, result);
        Ok(())
    }

    /// SRA: Shift Right Arithmetic (sign-extend)
    ///
    /// Shifts the value in rt right by shamt bits, preserving the sign bit.
    ///
    /// Format: sra rd, rt, shamt
    /// Operation: rd = rt >> shamt (sign-extend)
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
    fn op_sra(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
        let result = ((self.reg(rt) as i32) >> shamt) as u32;
        self.set_reg(rd, result);
        Ok(())
    }

    /// SLLV: Shift Left Logical Variable
    ///
    /// Shifts the value in rt left by the amount specified in the lower 5 bits of rs.
    ///
    /// Format: sllv rd, rt, rs
    /// Operation: rd = rt << (rs & 0x1F)
    ///
    /// # Arguments
    ///
    /// * `rs` - Register containing shift amount (lower 5 bits used)
    /// * `rt` - Source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sllv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let shamt = self.reg(rs) & 0x1F; // Only lower 5 bits
        let result = self.reg(rt) << shamt;
        self.set_reg(rd, result);
        Ok(())
    }

    /// SRLV: Shift Right Logical Variable
    ///
    /// Shifts the value in rt right by the amount specified in the lower 5 bits of rs,
    /// filling with zeros.
    ///
    /// Format: srlv rd, rt, rs
    /// Operation: rd = rt >> (rs & 0x1F)
    ///
    /// # Arguments
    ///
    /// * `rs` - Register containing shift amount (lower 5 bits used)
    /// * `rt` - Source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_srlv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let shamt = self.reg(rs) & 0x1F;
        let result = self.reg(rt) >> shamt;
        self.set_reg(rd, result);
        Ok(())
    }

    /// SRAV: Shift Right Arithmetic Variable
    ///
    /// Shifts the value in rt right by the amount specified in the lower 5 bits of rs,
    /// preserving the sign bit.
    ///
    /// Format: srav rd, rt, rs
    /// Operation: rd = rt >> (rs & 0x1F) (sign-extend)
    ///
    /// # Arguments
    ///
    /// * `rs` - Register containing shift amount (lower 5 bits used)
    /// * `rt` - Source register
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_srav(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let shamt = self.reg(rs) & 0x1F;
        let result = ((self.reg(rt) as i32) >> shamt) as u32;
        self.set_reg(rd, result);
        Ok(())
    }

    // === Load Instructions ===

    /// LW: Load Word (32-bit)
    ///
    /// Loads a 32-bit word from memory with load delay slot.
    /// The address must be 4-byte aligned.
    ///
    /// Format: lw rt, offset(rs)
    /// Operation: rt = memory[rs + sign_extend(offset)]
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) on success, triggers AddressErrorLoad exception on misalignment
    fn op_lw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Check alignment
        if addr & 0x3 != 0 {
            self.exception(ExceptionCause::AddressErrorLoad);
            return Ok(());
        }

        let value = bus.read32(addr)?;
        self.set_reg_delayed(rt, value); // Load delay slot
        Ok(())
    }

    /// LH: Load Halfword (16-bit, sign-extended)
    ///
    /// Loads a 16-bit halfword from memory and sign-extends it to 32 bits.
    /// The address must be 2-byte aligned.
    ///
    /// Format: lh rt, offset(rs)
    /// Operation: rt = sign_extend(memory[rs + sign_extend(offset)])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) on success, triggers AddressErrorLoad exception on misalignment
    fn op_lh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Check alignment
        if addr & 0x1 != 0 {
            self.exception(ExceptionCause::AddressErrorLoad);
            return Ok(());
        }

        let value = bus.read16(addr)? as i16 as i32 as u32; // Sign extend
        self.set_reg_delayed(rt, value); // Load delay slot
        Ok(())
    }

    /// LHU: Load Halfword Unsigned (16-bit, zero-extended)
    ///
    /// Loads a 16-bit halfword from memory and zero-extends it to 32 bits.
    /// The address must be 2-byte aligned.
    ///
    /// Format: lhu rt, offset(rs)
    /// Operation: rt = zero_extend(memory[rs + sign_extend(offset)])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) on success, triggers AddressErrorLoad exception on misalignment
    fn op_lhu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Check alignment
        if addr & 0x1 != 0 {
            self.exception(ExceptionCause::AddressErrorLoad);
            return Ok(());
        }

        let value = bus.read16(addr)? as u32; // Zero extend
        self.set_reg_delayed(rt, value); // Load delay slot
        Ok(())
    }

    /// LB: Load Byte (8-bit, sign-extended)
    ///
    /// Loads an 8-bit byte from memory and sign-extends it to 32 bits.
    /// No alignment restrictions.
    ///
    /// Format: lb rt, offset(rs)
    /// Operation: rt = sign_extend(memory[rs + sign_extend(offset)])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_lb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        let value = bus.read8(addr)? as i8 as i32 as u32; // Sign extend
        self.set_reg_delayed(rt, value); // Load delay slot
        Ok(())
    }

    /// LBU: Load Byte Unsigned (8-bit, zero-extended)
    ///
    /// Loads an 8-bit byte from memory and zero-extends it to 32 bits.
    /// No alignment restrictions.
    ///
    /// Format: lbu rt, offset(rs)
    /// Operation: rt = zero_extend(memory[rs + sign_extend(offset)])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_lbu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        let value = bus.read8(addr)? as u32; // Zero extend
        self.set_reg_delayed(rt, value); // Load delay slot
        Ok(())
    }

    /// LWL: Load Word Left (unaligned load support)
    ///
    /// Stub implementation for Phase 1 Week 2.
    /// Full implementation will be added in Week 3.
    ///
    /// Format: lwl rt, offset(rs)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) - stub implementation
    fn op_lwl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement LWL based on PSX-SPX documentation (Week 3)
        log::warn!(
            "LWL instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
        Ok(())
    }

    /// LWR: Load Word Right (unaligned load support)
    ///
    /// Stub implementation for Phase 1 Week 2.
    /// Full implementation will be added in Week 3.
    ///
    /// Format: lwr rt, offset(rs)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for reading
    ///
    /// # Returns
    ///
    /// Ok(()) - stub implementation
    fn op_lwr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement LWR based on PSX-SPX documentation (Week 3)
        log::warn!(
            "LWR instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
        Ok(())
    }

    // === Store Instructions ===

    /// SW: Store Word (32-bit)
    ///
    /// Stores a 32-bit word to memory.
    /// The address must be 4-byte aligned.
    ///
    /// Format: sw rt, offset(rs)
    /// Operation: memory[rs + sign_extend(offset)] = rt
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for writing
    ///
    /// # Returns
    ///
    /// Ok(()) on success, triggers AddressErrorStore exception on misalignment
    fn op_sw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Check alignment
        if addr & 0x3 != 0 {
            self.exception(ExceptionCause::AddressErrorStore);
            return Ok(());
        }

        bus.write32(addr, self.reg(rt))?;
        Ok(())
    }

    /// SH: Store Halfword (16-bit)
    ///
    /// Stores the lower 16 bits of a register to memory.
    /// The address must be 2-byte aligned.
    ///
    /// Format: sh rt, offset(rs)
    /// Operation: memory[rs + sign_extend(offset)] = rt[15:0]
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for writing
    ///
    /// # Returns
    ///
    /// Ok(()) on success, triggers AddressErrorStore exception on misalignment
    fn op_sh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Check alignment
        if addr & 0x1 != 0 {
            self.exception(ExceptionCause::AddressErrorStore);
            return Ok(());
        }

        bus.write16(addr, self.reg(rt) as u16)?;
        Ok(())
    }

    /// SB: Store Byte (8-bit)
    ///
    /// Stores the lower 8 bits of a register to memory.
    /// No alignment restrictions.
    ///
    /// Format: sb rt, offset(rs)
    /// Operation: memory[rs + sign_extend(offset)] = rt[7:0]
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for writing
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn op_sb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extend
        let addr = self.reg(rs).wrapping_add(offset as u32);

        bus.write8(addr, self.reg(rt) as u8)?;
        Ok(())
    }

    /// SWL: Store Word Left (unaligned store support)
    ///
    /// Stub implementation for Phase 1 Week 2.
    /// Full implementation will be added in Week 3.
    ///
    /// Format: swl rt, offset(rs)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for writing
    ///
    /// # Returns
    ///
    /// Ok(()) - stub implementation
    fn op_swl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement SWL (Week 3)
        log::warn!(
            "SWL instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
        Ok(())
    }

    /// SWR: Store Word Right (unaligned store support)
    ///
    /// Stub implementation for Phase 1 Week 2.
    /// Full implementation will be added in Week 3.
    ///
    /// Format: swr rt, offset(rs)
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus for writing
    ///
    /// # Returns
    ///
    /// Ok(()) - stub implementation
    fn op_swr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement SWR (Week 3)
        log::warn!(
            "SWR instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
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
    #[allow(dead_code)]
    fn branch(&mut self, offset: i32) {
        // next_pc already points to delay slot + 4, so add offset from there
        self.next_pc = self.next_pc.wrapping_add(offset as u32);
        self.in_branch_delay = true;
    }

    /// Trigger a CPU exception
    ///
    /// This method handles CPU exceptions by:
    /// 1. Saving the current processor mode in the Status Register
    /// 2. Recording the exception cause in the CAUSE register
    /// 3. Saving the exception PC (EPC)
    /// 4. Jumping to the exception handler
    ///
    /// # Arguments
    ///
    /// * `cause` - The exception cause code
    ///
    /// # Note
    ///
    /// The exception handler address depends on the BEV bit in the Status Register:
    /// - BEV=1 (bootstrap): 0xBFC00180
    /// - BEV=0 (normal): 0x80000080
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
        self.cop0.regs[COP0::EPC] = if self.in_branch_delay {
            current_pc.wrapping_sub(4) // branch instruction address
        } else {
            current_pc // faulting instruction address
        };

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

    // === Arithmetic Instruction Tests ===

    #[test]
    fn test_add_no_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 20);

        cpu.op_add(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 30);
    }

    #[test]
    fn test_add_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x7FFFFFFF); // Max positive i32
        cpu.set_reg(2, 1);

        cpu.op_add(1, 2, 3).unwrap();

        // Should trigger overflow exception
        // Check that exception was raised (via COP0 CAUSE register)
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::Overflow as u32);
    }

    #[test]
    fn test_add_negative_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x80000000_u32); // Min negative i32
        cpu.set_reg(2, 0xFFFFFFFF_u32); // -1 as u32

        cpu.op_add(1, 2, 3).unwrap();

        // Should trigger overflow exception
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::Overflow as u32);
    }

    #[test]
    fn test_addu_no_exception_on_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.set_reg(2, 1);

        cpu.op_addu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 0); // Wraps around
    }

    #[test]
    fn test_addu_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 200);

        cpu.op_addu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 300);
    }

    #[test]
    fn test_addi_basic() {
        use crate::core::memory::Bus;

        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 100);

        // ADDI r2, r1, 50 -> 0x20220032
        bus.write32(0x80000000, 0x20220032).unwrap();

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.reg(2), 150);
    }

    #[test]
    fn test_addi_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x7FFFFFFF); // Max positive i32

        // ADDI r2, r1, 1 -> 0x20220001
        let instr = 0x20220001;
        cpu.op_addi(instr).unwrap();

        // Should trigger overflow exception
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::Overflow as u32);
    }

    #[test]
    fn test_addiu_sign_extension() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x00000100);

        // ADDIU r2, r1, -1 (0xFFFF sign extends to 0xFFFFFFFF)
        let instr = 0x2422FFFF; // addiu r2, r1, -1
        cpu.op_addiu(instr).unwrap();

        assert_eq!(cpu.reg(2), 0x000000FF);
    }

    #[test]
    fn test_addiu_no_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);

        // ADDIU r2, r1, 1 -> 0x24220001
        let instr = 0x24220001;
        cpu.op_addiu(instr).unwrap();

        assert_eq!(cpu.reg(2), 0); // Wraps around
    }

    #[test]
    fn test_sub() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 30);

        cpu.op_sub(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 70);
    }

    #[test]
    fn test_sub_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x80000000_u32); // Min negative i32
        cpu.set_reg(2, 1);

        cpu.op_sub(1, 2, 3).unwrap();

        // Should trigger overflow exception
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::Overflow as u32);
    }

    #[test]
    fn test_subu_underflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 20);

        cpu.op_subu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 0xFFFFFFF6_u32); // -10 as u32
    }

    #[test]
    fn test_subu_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 30);

        cpu.op_subu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 70);
    }

    #[test]
    fn test_slt_true() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10_u32.wrapping_neg()); // -10 as u32
        cpu.set_reg(2, 5);

        cpu.op_slt(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 1); // -10 < 5
    }

    #[test]
    fn test_slt_false() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 5);

        cpu.op_slt(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 0); // 10 >= 5
    }

    #[test]
    fn test_slt_equal() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 100);

        cpu.op_slt(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 0); // 100 >= 100
    }

    #[test]
    fn test_sltu() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.set_reg(2, 1);

        cpu.op_sltu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 0); // 0xFFFFFFFF > 1 (unsigned)
    }

    #[test]
    fn test_sltu_true() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 5);
        cpu.set_reg(2, 10);

        cpu.op_sltu(1, 2, 3).unwrap();

        assert_eq!(cpu.reg(3), 1); // 5 < 10 (unsigned)
    }

    #[test]
    fn test_slti_true() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 5);

        // SLTI r3, r1, 10 -> 0x2823000A
        let instr = 0x2823000A;
        cpu.op_slti(instr).unwrap();

        assert_eq!(cpu.reg(3), 1); // 5 < 10
    }

    #[test]
    fn test_slti_false() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 15);

        // SLTI r3, r1, 10 -> 0x2823000A
        let instr = 0x2823000A;
        cpu.op_slti(instr).unwrap();

        assert_eq!(cpu.reg(3), 0); // 15 >= 10
    }

    #[test]
    fn test_slti_negative_immediate() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFF6_u32); // -10 as u32

        // SLTI r3, r1, -5 (0xFFFB) -> 0x2823FFFB
        let instr = 0x2823FFFB;
        cpu.op_slti(instr).unwrap();

        assert_eq!(cpu.reg(3), 1); // -10 < -5
    }

    #[test]
    fn test_sltiu_true() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 5);

        // SLTIU r3, r1, 10 -> 0x2C23000A
        let instr = 0x2C23000A;
        cpu.op_sltiu(instr).unwrap();

        assert_eq!(cpu.reg(3), 1); // 5 < 10 (unsigned)
    }

    #[test]
    fn test_sltiu_false() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);

        // SLTIU r3, r1, 10 -> 0x2C23000A
        let instr = 0x2C23000A;
        cpu.op_sltiu(instr).unwrap();

        assert_eq!(cpu.reg(3), 0); // 0xFFFFFFFF > 10 (unsigned)
    }

    #[test]
    fn test_sltiu_sign_extension() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFF0_u32);

        // SLTIU r3, r1, -1 (0xFFFF sign extends to 0xFFFFFFFF)
        let instr = 0x2C23FFFF;
        cpu.op_sltiu(instr).unwrap();

        assert_eq!(cpu.reg(3), 1); // 0xFFFFFFF0 < 0xFFFFFFFF
    }

    #[test]
    fn test_exception_handling() {
        let mut cpu = CPU::new();

        // Trigger an overflow exception
        cpu.exception(ExceptionCause::Overflow);

        // Check CAUSE register
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::Overflow as u32);

        // Check PC jumped to exception handler
        // BEV bit (bit 22) in initial SR (0x10900000) is not set, so should jump to normal handler
        assert_eq!(cpu.pc, 0x80000080);
    }

    #[test]
    fn test_exception_handling_bootstrap() {
        let mut cpu = CPU::new();

        // Set BEV bit (bit 22) in Status Register
        cpu.cop0.regs[COP0::SR] |= 1 << 22;

        // Trigger an exception
        cpu.exception(ExceptionCause::Overflow);

        // Check PC jumped to bootstrap exception handler
        assert_eq!(cpu.pc, 0xBFC00180);
    }

    #[test]
    fn test_exception_epc_saved() {
        let mut cpu = CPU::new();
        // In this core, self.pc points to (current_pc + 4) during execution.
        cpu.pc = 0x80001004;

        cpu.exception(ExceptionCause::Syscall);

        // Check EPC saved correctly
        assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80001000);
    }

    #[test]
    fn test_exception_epc_and_bd_in_delay_slot() {
        let mut cpu = CPU::new();
        // Simulate executing a delay-slot instruction: pc = branch_pc + 8
        cpu.pc = 0x80001008;
        cpu.in_branch_delay = true;
        cpu.exception(ExceptionCause::Overflow);
        // EPC must point to branch instruction; BD must be set.
        assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80001000);
        assert_ne!(cpu.cop0.regs[COP0::CAUSE] & (1 << 31), 0);
    }

    // === Logical Instruction Tests ===

    #[test]
    fn test_and() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0b11110000);
        cpu.set_reg(2, 0b10101010);
        cpu.op_and(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0b10100000);
    }

    #[test]
    fn test_and_all_bits() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.set_reg(2, 0x12345678);
        cpu.op_and(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x12345678);
    }

    #[test]
    fn test_andi_zero_extension() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        // ANDI r3, r1, 0xFFFF -> 0x3023FFFF
        let instr = 0x3023FFFF;
        cpu.op_andi(instr).unwrap();
        assert_eq!(cpu.reg(3), 0x0000FFFF);
    }

    #[test]
    fn test_andi_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x12345678);
        // ANDI r3, r1, 0x00FF -> 0x302300FF
        let instr = 0x302300FF;
        cpu.op_andi(instr).unwrap();
        assert_eq!(cpu.reg(3), 0x00000078);
    }

    #[test]
    fn test_or() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0b11110000);
        cpu.set_reg(2, 0b00001111);
        cpu.op_or(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0b11111111);
    }

    #[test]
    fn test_or_identity() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x12345678);
        cpu.set_reg(2, 0x00000000);
        cpu.op_or(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x12345678);
    }

    #[test]
    fn test_ori_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x12340000);
        // ORI r3, r1, 0x5678 -> 0x34235678
        let instr = 0x34235678;
        cpu.op_ori(instr).unwrap();
        assert_eq!(cpu.reg(3), 0x12345678);
    }

    #[test]
    fn test_ori_zero_extension() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x00000000);
        // ORI r3, r1, 0xFFFF -> 0x3423FFFF
        let instr = 0x3423FFFF;
        cpu.op_ori(instr).unwrap();
        assert_eq!(cpu.reg(3), 0x0000FFFF);
    }

    #[test]
    fn test_xor() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0b11110000);
        cpu.set_reg(2, 0b10101010);
        cpu.op_xor(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0b01011010);
    }

    #[test]
    fn test_xor_same_value() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x12345678);
        cpu.set_reg(2, 0x12345678);
        cpu.op_xor(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x00000000);
    }

    #[test]
    fn test_xori_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFF0000);
        // XORI r3, r1, 0xFFFF -> 0x3823FFFF
        let instr = 0x3823FFFF;
        cpu.op_xori(instr).unwrap();
        assert_eq!(cpu.reg(3), 0xFFFFFFFF);
    }

    #[test]
    fn test_xori_toggle_bits() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x000000FF);
        // XORI r3, r1, 0x00FF -> 0x382300FF
        let instr = 0x382300FF;
        cpu.op_xori(instr).unwrap();
        assert_eq!(cpu.reg(3), 0x00000000);
    }

    #[test]
    fn test_nor() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x00000000);
        cpu.set_reg(2, 0x00000000);
        cpu.op_nor(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0xFFFFFFFF);
    }

    #[test]
    fn test_nor_with_values() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x0F0F0F0F);
        cpu.set_reg(2, 0xF0F0F0F0);
        cpu.op_nor(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x00000000);
    }

    // === Shift Instruction Tests ===

    #[test]
    fn test_sll_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x00000001);
        cpu.op_sll(1, 2, 4).unwrap();
        assert_eq!(cpu.reg(2), 0x00000010);
    }

    #[test]
    fn test_sll_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x80000000);
        cpu.op_sll(1, 2, 1).unwrap();
        assert_eq!(cpu.reg(2), 0x00000000);
    }

    #[test]
    fn test_srl_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xF0000000);
        cpu.op_srl(1, 2, 4).unwrap();
        assert_eq!(cpu.reg(2), 0x0F000000);
    }

    #[test]
    fn test_srl_zero_fill() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.op_srl(1, 2, 1).unwrap();
        assert_eq!(cpu.reg(2), 0x7FFFFFFF);
    }

    #[test]
    fn test_sra_positive() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x70000000);
        cpu.op_sra(1, 2, 4).unwrap();
        assert_eq!(cpu.reg(2), 0x07000000);
    }

    #[test]
    fn test_sra_negative() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xF0000000); // Negative number
        cpu.op_sra(1, 2, 4).unwrap();
        assert_eq!(cpu.reg(2), 0xFF000000); // Sign-extended
    }

    #[test]
    fn test_sra_negative_shift_one() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x80000000); // Most negative i32
        cpu.op_sra(1, 2, 1).unwrap();
        assert_eq!(cpu.reg(2), 0xC0000000); // Sign-extended
    }

    #[test]
    fn test_sllv_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 4); // Shift amount
        cpu.set_reg(2, 1);
        cpu.op_sllv(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 16);
    }

    #[test]
    fn test_sllv_mask() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x100); // Only lower 5 bits used (0)
        cpu.set_reg(2, 1);
        cpu.op_sllv(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 1); // Shift by 0
    }

    #[test]
    fn test_sllv_max_shift() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 31); // Maximum shift
        cpu.set_reg(2, 1);
        cpu.op_sllv(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x80000000);
    }

    #[test]
    fn test_srlv_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 4); // Shift amount
        cpu.set_reg(2, 0x00000100);
        cpu.op_srlv(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x00000010);
    }

    #[test]
    fn test_srlv_mask() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x120); // Only lower 5 bits used (0)
        cpu.set_reg(2, 0x12345678);
        cpu.op_srlv(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x12345678); // No shift
    }

    #[test]
    fn test_srav_positive() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 4); // Shift amount
        cpu.set_reg(2, 0x70000000);
        cpu.op_srav(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0x07000000);
    }

    #[test]
    fn test_srav_negative() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 4); // Shift amount
        cpu.set_reg(2, 0xF0000000); // Negative
        cpu.op_srav(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0xFF000000); // Sign-extended
    }

    #[test]
    fn test_srav_mask() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x104); // Only lower 5 bits used (4)
        cpu.set_reg(2, 0x80000000); // Negative
        cpu.op_srav(1, 2, 3).unwrap();
        assert_eq!(cpu.reg(3), 0xF8000000); // Sign-extended
    }

    // === Load/Store Instruction Tests ===

    #[test]
    fn test_lw_basic() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up base address
        cpu.set_reg(1, 0x80000000);

        // Store value in memory
        bus.write32(0x80000000, 0x12345678).unwrap();

        // LW r2, 0(r1) -> load from 0x80000000 into r2
        let instr = 0x8C220000; // opcode=0x23, rs=1, rt=2, offset=0
        cpu.op_lw(instr, &mut bus).unwrap();

        // Value not yet visible due to load delay
        assert_eq!(cpu.reg(2), 0);

        // Execute another load to flush the delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x12345678);
    }

    #[test]
    fn test_lw_with_offset() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up base address
        cpu.set_reg(1, 0x80000000);

        // Store value in memory
        bus.write32(0x80000010, 0xAABBCCDD).unwrap();

        // LW r2, 16(r1) -> load from 0x80000010 into r2
        let instr = 0x8C220010; // opcode=0x23, rs=1, rt=2, offset=16
        cpu.op_lw(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0xAABBCCDD);
    }

    #[test]
    fn test_lw_negative_offset() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up base address
        cpu.set_reg(1, 0x80000020);

        // Store value in memory
        bus.write32(0x80000010, 0xDEADBEEF).unwrap();

        // LW r2, -16(r1) -> load from 0x80000010 into r2
        let instr = 0x8C22FFF0; // opcode=0x23, rs=1, rt=2, offset=-16 (0xFFF0)
        cpu.op_lw(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0xDEADBEEF);
    }

    #[test]
    fn test_lw_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up base address (misaligned)
        cpu.set_reg(1, 0x80000001);

        // LW r2, 0(r1) -> should trigger exception
        let instr = 0x8C220000;
        cpu.op_lw(instr, &mut bus).unwrap();

        // Check exception was raised
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::AddressErrorLoad as u32);
    }

    #[test]
    fn test_lh_sign_extension() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store negative halfword
        bus.write16(0x80000000, 0x8000).unwrap(); // -32768 as i16

        // LH r2, 0(r1)
        let instr = 0x84220000; // opcode=0x21, rs=1, rt=2, offset=0
        cpu.op_lh(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0xFFFF8000); // Sign-extended
    }

    #[test]
    fn test_lh_positive() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store positive halfword
        bus.write16(0x80000000, 0x1234).unwrap();

        // LH r2, 0(r1)
        let instr = 0x84220000;
        cpu.op_lh(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x00001234); // Zero upper bits
    }

    #[test]
    fn test_lh_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000001); // Misaligned

        // LH r2, 0(r1) -> should trigger exception
        let instr = 0x84220000;
        cpu.op_lh(instr, &mut bus).unwrap();

        // Check exception was raised
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::AddressErrorLoad as u32);
    }

    #[test]
    fn test_lhu_zero_extension() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store halfword with high bit set
        bus.write16(0x80000000, 0x8000).unwrap();

        // LHU r2, 0(r1)
        let instr = 0x94220000; // opcode=0x25, rs=1, rt=2, offset=0
        cpu.op_lhu(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x00008000); // Zero-extended, not sign-extended
    }

    #[test]
    fn test_lhu_max_value() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        bus.write16(0x80000000, 0xFFFF).unwrap();

        // LHU r2, 0(r1)
        let instr = 0x94220000;
        cpu.op_lhu(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x0000FFFF); // Zero-extended
    }

    #[test]
    fn test_lb_sign_extension() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store negative byte
        bus.write8(0x80000000, 0x80).unwrap(); // -128 as i8

        // LB r2, 0(r1)
        let instr = 0x80220000; // opcode=0x20, rs=1, rt=2, offset=0
        cpu.op_lb(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0xFFFFFF80); // Sign-extended
    }

    #[test]
    fn test_lb_positive() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store positive byte
        bus.write8(0x80000000, 0x42).unwrap();

        // LB r2, 0(r1)
        let instr = 0x80220000;
        cpu.op_lb(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x00000042);
    }

    #[test]
    fn test_lbu_zero_extension() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store byte with high bit set
        bus.write8(0x80000000, 0xFF).unwrap();

        // LBU r2, 0(r1)
        let instr = 0x90220000; // opcode=0x24, rs=1, rt=2, offset=0
        cpu.op_lbu(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x000000FF); // Zero-extended, not sign-extended
    }

    #[test]
    fn test_lbu_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Byte loads can be unaligned
        cpu.set_reg(1, 0x80000001);
        bus.write8(0x80000001, 0xAB).unwrap();

        // LBU r2, 0(r1)
        let instr = 0x90220000;
        cpu.op_lbu(instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(3, 0);
        assert_eq!(cpu.reg(2), 0x000000AB);
    }

    #[test]
    fn test_sw_basic() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);
        cpu.set_reg(2, 0x12345678);

        // SW r2, 0(r1) -> store to 0x80000000
        let instr = 0xAC220000; // opcode=0x2B, rs=1, rt=2, offset=0
        cpu.op_sw(instr, &mut bus).unwrap();

        // Verify value was written
        assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
    }

    #[test]
    fn test_sw_with_offset() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);
        cpu.set_reg(2, 0xDEADBEEF);

        // SW r2, 16(r1) -> store to 0x80000010
        let instr = 0xAC220010; // opcode=0x2B, rs=1, rt=2, offset=16
        cpu.op_sw(instr, &mut bus).unwrap();

        // Verify value was written
        assert_eq!(bus.read32(0x80000010).unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn test_sw_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000001); // Misaligned
        cpu.set_reg(2, 0x12345678);

        // SW r2, 0(r1) -> should trigger exception
        let instr = 0xAC220000;
        cpu.op_sw(instr, &mut bus).unwrap();

        // Check exception was raised
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::AddressErrorStore as u32);
    }

    #[test]
    fn test_sh_basic() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);
        cpu.set_reg(2, 0x12345678);

        // SH r2, 0(r1) -> store lower 16 bits to 0x80000000
        let instr = 0xA4220000; // opcode=0x29, rs=1, rt=2, offset=0
        cpu.op_sh(instr, &mut bus).unwrap();

        // Verify value was written (only lower 16 bits)
        assert_eq!(bus.read16(0x80000000).unwrap(), 0x5678);
    }

    #[test]
    fn test_sh_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000001); // Misaligned
        cpu.set_reg(2, 0x1234);

        // SH r2, 0(r1) -> should trigger exception
        let instr = 0xA4220000;
        cpu.op_sh(instr, &mut bus).unwrap();

        // Check exception was raised
        let cause = cpu.cop0.regs[COP0::CAUSE];
        let exception_code = (cause >> 2) & 0x1F;
        assert_eq!(exception_code, ExceptionCause::AddressErrorStore as u32);
    }

    #[test]
    fn test_sb_basic() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);
        cpu.set_reg(2, 0x12345678);

        // SB r2, 0(r1) -> store lower 8 bits to 0x80000000
        let instr = 0xA0220000; // opcode=0x28, rs=1, rt=2, offset=0
        cpu.op_sb(instr, &mut bus).unwrap();

        // Verify value was written (only lower 8 bits)
        assert_eq!(bus.read8(0x80000000).unwrap(), 0x78);
    }

    #[test]
    fn test_sb_unaligned() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Byte stores can be unaligned
        cpu.set_reg(1, 0x80000001);
        cpu.set_reg(2, 0xAB);

        // SB r2, 0(r1)
        let instr = 0xA0220000;
        cpu.op_sb(instr, &mut bus).unwrap();

        // Verify value was written
        assert_eq!(bus.read8(0x80000001).unwrap(), 0xAB);
    }

    #[test]
    fn test_load_delay_slot_interaction() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up memory
        bus.write32(0x80000000, 0x11111111).unwrap();
        bus.write32(0x80000004, 0x22222222).unwrap();

        cpu.set_reg(1, 0x80000000);
        cpu.set_reg(2, 0x80000004);

        // LW r3, 0(r1) - Load first value
        let instr1 = 0x8C230000;
        cpu.op_lw(instr1, &mut bus).unwrap();

        // r3 not yet available
        assert_eq!(cpu.reg(3), 0);

        // LW r4, 0(r2) - Load second value, flushes first delay
        let instr2 = 0x8C440000;
        cpu.op_lw(instr2, &mut bus).unwrap();

        // Now r3 has first value, r4 still waiting
        assert_eq!(cpu.reg(3), 0x11111111);
        assert_eq!(cpu.reg(4), 0);

        // Another instruction flushes second delay
        cpu.set_reg_delayed(5, 0);
        assert_eq!(cpu.reg(4), 0x22222222);
    }

    #[test]
    fn test_load_store_round_trip() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store sequence
        cpu.set_reg(2, 0x12345678);
        let sw_instr = 0xAC220000;
        cpu.op_sw(sw_instr, &mut bus).unwrap();

        // Load back
        let lw_instr = 0x8C230000;
        cpu.op_lw(lw_instr, &mut bus).unwrap();

        // Flush delay
        cpu.set_reg_delayed(4, 0);

        // Verify round trip
        assert_eq!(cpu.reg(3), 0x12345678);
    }

    #[test]
    fn test_mixed_size_load_store() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        cpu.set_reg(1, 0x80000000);

        // Store a word
        cpu.set_reg(2, 0x12345678);
        let sw_instr = 0xAC220000;
        cpu.op_sw(sw_instr, &mut bus).unwrap();

        // Load individual bytes
        let lb_instr0 = 0x80230000; // LB r3, 0(r1)
        cpu.op_lb(lb_instr0, &mut bus).unwrap();
        cpu.set_reg_delayed(0, 0); // Flush
        assert_eq!(cpu.reg(3), 0x00000078); // Little-endian, byte 0

        let lb_instr1 = 0x80230001; // LB r3, 1(r1)
        cpu.op_lb(lb_instr1, &mut bus).unwrap();
        cpu.set_reg_delayed(0, 0); // Flush
        assert_eq!(cpu.reg(3), 0x00000056); // Byte 1

        // Load halfword
        let lh_instr = 0x84230000; // LH r3, 0(r1)
        cpu.op_lh(lh_instr, &mut bus).unwrap();
        cpu.set_reg_delayed(0, 0); // Flush
        assert_eq!(cpu.reg(3), 0x00005678); // Lower halfword
    }

    // === Branch and Jump Instruction Tests ===

    #[test]
    fn test_jr_instruction() {
        let mut cpu = CPU::new();
        cpu.set_reg(31, 0x80001234);

        // JR r31
        cpu.op_jr(31).unwrap();

        assert_eq!(cpu.next_pc, 0x80001234);
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_jalr_instruction() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 0x80005678);

        // JALR r1, r31
        cpu.op_jalr(1, 31).unwrap();

        assert_eq!(cpu.next_pc, 0x80005678);
        assert_eq!(cpu.reg(31), 0x80000004); // Return address
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_beq_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 100);

        // BEQ r1, r2, 8 (branch offset = 8)
        let beq_instr = 0x10220002; // offset = 2 words
        cpu.op_beq(beq_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_beq_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 200);

        // BEQ r1, r2, 8
        let beq_instr = 0x10220002;
        cpu.op_beq(beq_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bne_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 200);

        // BNE r1, r2, 8
        let bne_instr = 0x14220002;
        cpu.op_bne(bne_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bne_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 100);
        cpu.set_reg(2, 100);

        // BNE r1, r2, 8
        let bne_instr = 0x14220002;
        cpu.op_bne(bne_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_blez_taken_zero() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 0);

        // BLEZ r1, 8
        let blez_instr = 0x18200002;
        cpu.op_blez(blez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_blez_taken_negative() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BLEZ r1, 8
        let blez_instr = 0x18200002;
        cpu.op_blez(blez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_blez_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BLEZ r1, 8
        let blez_instr = 0x18200002;
        cpu.op_blez(blez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bgtz_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BGTZ r1, 8
        let bgtz_instr = 0x1C200002;
        cpu.op_bgtz(bgtz_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bgtz_not_taken_zero() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 0);

        // BGTZ r1, 8
        let bgtz_instr = 0x1C200002;
        cpu.op_bgtz(bgtz_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bgtz_not_taken_negative() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BGTZ r1, 8
        let bgtz_instr = 0x1C200002;
        cpu.op_bgtz(bgtz_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bltz_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BLTZ r1, 8 (rt=0x00 for BLTZ)
        let bltz_instr = 0x04200002;
        cpu.execute_bcondz(bltz_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bltz_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BLTZ r1, 8
        let bltz_instr = 0x04200002;
        cpu.execute_bcondz(bltz_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bgez_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BGEZ r1, 8 (rt=0x01 for BGEZ)
        let bgez_instr = 0x04210002;
        cpu.execute_bcondz(bgez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bgez_taken_zero() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 0);

        // BGEZ r1, 8
        let bgez_instr = 0x04210002;
        cpu.execute_bcondz(bgez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken (zero is >= 0)
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bgez_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BGEZ r1, 8
        let bgez_instr = 0x04210002;
        cpu.execute_bcondz(bgez_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bltzal_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BLTZAL r1, 8 (rt=0x10 for BLTZAL)
        let bltzal_instr = 0x04300002;
        cpu.execute_bcondz(bltzal_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert_eq!(cpu.reg(31), 0x80000004); // Return address saved
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bltzal_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BLTZAL r1, 8
        let bltzal_instr = 0x04300002;
        cpu.execute_bcondz(bltzal_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert_eq!(cpu.reg(31), 0x80000004); // Return address still saved
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_bgezal_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, 10);

        // BGEZAL r1, 8 (rt=0x11 for BGEZAL)
        let bgezal_instr = 0x04310002;
        cpu.execute_bcondz(bgezal_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
        assert_eq!(cpu.reg(31), 0x80000004); // Return address saved
        assert!(cpu.in_delay_slot());
    }

    #[test]
    fn test_bgezal_not_taken() {
        let mut cpu = CPU::new();
        cpu.pc = 0x80000000;
        cpu.next_pc = 0x80000004;
        cpu.set_reg(1, (-10i32) as u32);

        // BGEZAL r1, 8
        let bgezal_instr = 0x04310002;
        cpu.execute_bcondz(bgezal_instr).unwrap();

        assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
        assert_eq!(cpu.reg(31), 0x80000004); // Return address still saved
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_branch_delay_slot_cleared_after_step() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set up a simple program that doesn't branch
        cpu.pc = 0xBFC00000;
        cpu.next_pc = 0xBFC00004;
        cpu.in_branch_delay = true;

        // Write a NOP instruction (SLL r0, r0, 0)
        bus.write32(0xBFC00000, 0x00000000).unwrap();

        // Execute step
        cpu.step(&mut bus).unwrap();

        // Branch delay flag should be cleared
        assert!(!cpu.in_delay_slot());
    }

    #[test]
    fn test_jump_preserves_upper_pc_bits() {
        let mut cpu = CPU::new();
        cpu.pc = 0xBFC00000;
        cpu.next_pc = 0xBFC00004;

        // J 0x00100000 (should preserve upper 4 bits of PC)
        let j_instr = 0x08100000;
        cpu.op_j(j_instr).unwrap();

        // Upper 4 bits should be 0xB (from 0xBFC00000)
        assert_eq!(cpu.next_pc & 0xF0000000, 0xB0000000);
        assert_eq!(cpu.next_pc, 0xB0400000);
    }
}
