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

use super::decode::{decode_i_type, decode_j_type, decode_r_type};
use super::{ExceptionCause, CPU};
use crate::core::error::Result;
use crate::core::memory::Bus;

impl CPU {
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
    pub(super) fn execute_instruction(&mut self, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn execute_special(&mut self, instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(super) fn execute_bcondz(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_lui(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_sll(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
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
    pub(super) fn op_j(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_jal(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_jr(&mut self, rs: u8) -> Result<()> {
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
    pub(super) fn op_jalr(&mut self, rs: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_beq(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_bne(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_blez(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_bgtz(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_add(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_addu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_addi(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_addiu(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_sub(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_subu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_slt(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_sltu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_slti(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_sltiu(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_and(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_andi(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_or(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_ori(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_xor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_xori(&mut self, instruction: u32) -> Result<()> {
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
    pub(super) fn op_nor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_srl(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
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
    pub(super) fn op_sra(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
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
    pub(super) fn op_sllv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_srlv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_srav(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(super) fn op_lw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lhu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lbu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lwl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_lwr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_sw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_sh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_sb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_swl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(super) fn op_swr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(super) fn branch(&mut self, offset: i32) {
        // next_pc already points to delay slot + 4, so add offset from there
        self.next_pc = self.next_pc.wrapping_add(offset as u32);
        self.in_branch_delay = true;
    }
}
