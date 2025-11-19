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
use super::super::{ExceptionCause, CPU};
use crate::core::error::Result;

impl CPU {
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
    pub(crate) fn op_add(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_addu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_addi(&mut self, instruction: u32) -> Result<()> {
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
    pub(crate) fn op_addiu(&mut self, instruction: u32) -> Result<()> {
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
    pub(crate) fn op_sub(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_subu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_slt(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_sltu(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(crate) fn op_slti(&mut self, instruction: u32) -> Result<()> {
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
    pub(crate) fn op_sltiu(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as u32; // Sign extend then treat as unsigned
        let a = self.reg(rs);
        let result = if a < imm { 1 } else { 0 };
        self.set_reg(rt, result);
        Ok(())
    }
}
