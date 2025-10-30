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

use super::super::CPU;
use crate::core::error::Result;

impl CPU {
    // === Shift Instructions ===

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
    pub(in crate::core::cpu) fn op_sll(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
        let value = self.reg(rt) << shamt;
        self.set_reg(rd, value);
        Ok(())
    }

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
    pub(in crate::core::cpu) fn op_srl(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_sra(&mut self, rt: u8, rd: u8, shamt: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_sllv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_srlv(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
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
    pub(in crate::core::cpu) fn op_srav(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let shamt = self.reg(rs) & 0x1F;
        let result = ((self.reg(rt) as i32) >> shamt) as u32;
        self.set_reg(rd, result);
        Ok(())
    }
}
