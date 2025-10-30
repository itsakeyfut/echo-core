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
    // === Logical Instructions ===

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
    pub(in crate::core::cpu) fn op_lui(&mut self, instruction: u32) -> Result<()> {
        let (_, _, rt, imm) = decode_i_type(instruction);
        let value = (imm as u32) << 16;
        self.set_reg(rt, value);
        Ok(())
    }

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
    pub(in crate::core::cpu) fn op_and(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_andi(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_or(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_ori(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_xor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_xori(&mut self, instruction: u32) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_nor(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let result = !(self.reg(rs) | self.reg(rt));
        self.set_reg(rd, result);
        Ok(())
    }
}
