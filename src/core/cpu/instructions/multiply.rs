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
    // === Multiply/Divide Instructions ===

    /// MULT: Multiply (signed)
    ///
    /// Multiplies two 32-bit signed integers and stores the 64-bit result
    /// in the HI and LO registers.
    ///
    /// Format: mult rs, rt
    /// Operation: (HI, LO) = rs * rt (signed 64-bit result)
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Multiply 100 * 200 = 20000
    /// // LO = 20000 (0x4E20), HI = 0
    /// cpu.set_reg(1, 100);
    /// cpu.set_reg(2, 200);
    /// cpu.op_mult(1, 2);
    /// ```
    pub(crate) fn op_mult(&mut self, rs: u8, rt: u8) -> Result<()> {
        let a = self.reg(rs) as i32 as i64;
        let b = self.reg(rt) as i32 as i64;
        let result = a * b;

        self.lo = result as u32;
        self.hi = (result >> 32) as u32;
        Ok(())
    }

    /// MULTU: Multiply Unsigned
    ///
    /// Multiplies two 32-bit unsigned integers and stores the 64-bit result
    /// in the HI and LO registers.
    ///
    /// Format: multu rs, rt
    /// Operation: (HI, LO) = rs * rt (unsigned 64-bit result)
    ///
    /// # Arguments
    ///
    /// * `rs` - First source register
    /// * `rt` - Second source register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Multiply 0xFFFFFFFF * 2
    /// // Result = 0x1FFFFFFFE
    /// // LO = 0xFFFFFFFE, HI = 1
    /// cpu.set_reg(1, 0xFFFFFFFF);
    /// cpu.set_reg(2, 2);
    /// cpu.op_multu(1, 2);
    /// ```
    pub(crate) fn op_multu(&mut self, rs: u8, rt: u8) -> Result<()> {
        let a = self.reg(rs) as u64;
        let b = self.reg(rt) as u64;
        let result = a * b;

        self.lo = result as u32;
        self.hi = (result >> 32) as u32;
        Ok(())
    }

    /// DIV: Divide (signed)
    ///
    /// Divides two 32-bit signed integers and stores quotient in LO
    /// and remainder in HI.
    ///
    /// Format: div rs, rt
    /// Operation: LO = rs / rt (quotient), HI = rs % rt (remainder)
    ///
    /// # Arguments
    ///
    /// * `rs` - Dividend register
    /// * `rt` - Divisor register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Special Cases
    ///
    /// * Division by zero: LO = 0xFFFFFFFF or 1 (based on sign), HI = numerator
    /// * Overflow (0x80000000 / -1): LO = 0x80000000, HI = 0
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 100 / 7 = 14 remainder 2
    /// cpu.set_reg(1, 100);
    /// cpu.set_reg(2, 7);
    /// cpu.op_div(1, 2);
    /// // LO = 14, HI = 2
    /// ```
    pub(crate) fn op_div(&mut self, rs: u8, rt: u8) -> Result<()> {
        let numerator = self.reg(rs) as i32;
        let denominator = self.reg(rt) as i32;

        if denominator == 0 {
            // PSX doesn't trap on divide by zero
            // Result is undefined but follows a pattern
            self.lo = if numerator >= 0 { 0xFFFFFFFF } else { 1 };
            self.hi = numerator as u32;
        } else if numerator as u32 == 0x80000000 && denominator == -1 {
            // Overflow case: i32::MIN / -1
            self.lo = 0x80000000;
            self.hi = 0;
        } else {
            self.lo = (numerator / denominator) as u32;
            self.hi = (numerator % denominator) as u32;
        }
        Ok(())
    }

    /// DIVU: Divide Unsigned
    ///
    /// Divides two 32-bit unsigned integers and stores quotient in LO
    /// and remainder in HI.
    ///
    /// Format: divu rs, rt
    /// Operation: LO = rs / rt (quotient), HI = rs % rt (remainder)
    ///
    /// # Arguments
    ///
    /// * `rs` - Dividend register
    /// * `rt` - Divisor register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Special Cases
    ///
    /// * Division by zero: LO = 0xFFFFFFFF, HI = numerator
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 100 / 7 = 14 remainder 2
    /// cpu.set_reg(1, 100);
    /// cpu.set_reg(2, 7);
    /// cpu.op_divu(1, 2);
    /// // LO = 14, HI = 2
    /// ```
    pub(crate) fn op_divu(&mut self, rs: u8, rt: u8) -> Result<()> {
        let numerator = self.reg(rs);
        let denominator = self.reg(rt);

        if denominator == 0 {
            // PSX doesn't trap on divide by zero
            self.lo = 0xFFFFFFFF;
            self.hi = numerator;
        } else {
            self.lo = numerator / denominator;
            self.hi = numerator % denominator;
        }
        Ok(())
    }

    /// MFHI: Move From HI
    ///
    /// Copies the value from the HI register to a general-purpose register.
    ///
    /// Format: mfhi rd
    /// Operation: rd = HI
    ///
    /// # Arguments
    ///
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// cpu.hi = 0x12345678;
    /// cpu.op_mfhi(3);
    /// assert_eq!(cpu.reg(3), 0x12345678);
    /// ```
    pub(crate) fn op_mfhi(&mut self, rd: u8) -> Result<()> {
        self.set_reg(rd, self.hi);
        Ok(())
    }

    /// MFLO: Move From LO
    ///
    /// Copies the value from the LO register to a general-purpose register.
    ///
    /// Format: mflo rd
    /// Operation: rd = LO
    ///
    /// # Arguments
    ///
    /// * `rd` - Destination register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// cpu.lo = 0xABCDEF00;
    /// cpu.op_mflo(4);
    /// assert_eq!(cpu.reg(4), 0xABCDEF00);
    /// ```
    pub(crate) fn op_mflo(&mut self, rd: u8) -> Result<()> {
        self.set_reg(rd, self.lo);
        Ok(())
    }

    /// MTHI: Move To HI
    ///
    /// Copies the value from a general-purpose register to the HI register.
    ///
    /// Format: mthi rs
    /// Operation: HI = rs
    ///
    /// # Arguments
    ///
    /// * `rs` - Source register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// cpu.set_reg(5, 0x12345678);
    /// cpu.op_mthi(5);
    /// assert_eq!(cpu.hi, 0x12345678);
    /// ```
    pub(crate) fn op_mthi(&mut self, rs: u8) -> Result<()> {
        self.hi = self.reg(rs);
        Ok(())
    }

    /// MTLO: Move To LO
    ///
    /// Copies the value from a general-purpose register to the LO register.
    ///
    /// Format: mtlo rs
    /// Operation: LO = rs
    ///
    /// # Arguments
    ///
    /// * `rs` - Source register
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Example
    ///
    /// ```ignore
    /// cpu.set_reg(6, 0xABCDEF00);
    /// cpu.op_mtlo(6);
    /// assert_eq!(cpu.lo, 0xABCDEF00);
    /// ```
    pub(crate) fn op_mtlo(&mut self, rs: u8) -> Result<()> {
        self.lo = self.reg(rs);
        Ok(())
    }
}
