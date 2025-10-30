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

//! Coprocessor 0 (System Control) instructions

use super::super::cop0::COP0;
use super::CPU;
use crate::core::error::Result;

impl CPU {
    /// MFC0: Move From Coprocessor 0
    ///
    /// Moves the contents of a COP0 register to a general-purpose register.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Format
    ///
    /// MFC0 rt, rd
    ///
    /// # Example
    ///
    /// ```text
    /// MFC0 $t0, $12  # Move Status Register to $t0
    /// ```
    pub(in crate::core::cpu) fn op_mfc0(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.cop0.regs[rd as usize];
        self.set_reg_delayed(rt, value);
        Ok(())
    }

    /// MTC0: Move To Coprocessor 0
    ///
    /// Moves the contents of a general-purpose register to a COP0 register.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Format
    ///
    /// MTC0 rt, rd
    ///
    /// # Example
    ///
    /// ```text
    /// MTC0 $t0, $12  # Move $t0 to Status Register
    /// ```
    pub(in crate::core::cpu) fn op_mtc0(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.reg(rt);
        self.cop0.regs[rd as usize] = value;
        Ok(())
    }

    /// RFE: Return From Exception
    ///
    /// Restores the previous processor mode by shifting the mode bits
    /// in the Status Register (SR) right by 2 bits.
    ///
    /// # Arguments
    ///
    /// * `_instruction` - The full 32-bit instruction (unused)
    ///
    /// # Details
    ///
    /// The Status Register contains mode bits in positions [5:0]:
    /// - Bits [1:0]: Current mode (KUc, IEc)
    /// - Bits [3:2]: Previous mode (KUp, IEp)
    /// - Bits [5:4]: Old mode (KUo, IEo)
    ///
    /// RFE shifts these bits right by 2, restoring the previous mode.
    ///
    /// # Example
    ///
    /// ```text
    /// RFE  # Return from exception handler
    /// ```
    pub(in crate::core::cpu) fn op_rfe(&mut self, _instruction: u32) -> Result<()> {
        let sr = self.cop0.regs[COP0::SR];
        // Shift mode bits right by 2 (restore previous mode)
        let mode = sr & 0x3F;
        self.cop0.regs[COP0::SR] = (sr & !0x3F) | (mode >> 2);
        Ok(())
    }
}
