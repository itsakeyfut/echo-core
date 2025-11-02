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

use super::super::decode::decode_i_type;
use super::super::CPU;
use crate::core::error::Result;

impl CPU {
    // === Branch Instructions ===

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
    pub(in crate::core::cpu) fn execute_bcondz(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_beq(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_bne(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_blez(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_bgtz(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, _, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if (self.reg(rs) as i32) > 0 {
            self.branch(offset);
        }
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
    /// The branch target is computed as (B + 4) + offset per MIPS semantics,
    /// where B is the branch instruction address. At the time this function
    /// is called (during execute_instruction), self.pc contains the delay slot
    /// address (B + 4), so we use self.pc as the base for the calculation.
    pub(in crate::core::cpu) fn branch(&mut self, offset: i32) {
        // self.pc points to the delay-slot address (B + 4) during execution.
        // Target = (B + 4) + offset
        let base = self.pc;
        self.next_pc = base.wrapping_add(offset as u32);
        self.in_branch_delay = true;
    }
}
