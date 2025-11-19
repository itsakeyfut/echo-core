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

use super::super::decode::decode_j_type;
use super::super::CPU;
use crate::core::error::Result;

impl CPU {
    // === Jump Instructions ===

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
    pub(crate) fn op_j(&mut self, instruction: u32) -> Result<()> {
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
    pub(crate) fn op_jal(&mut self, instruction: u32) -> Result<()> {
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
    pub(crate) fn op_jr(&mut self, rs: u8) -> Result<()> {
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
    pub(crate) fn op_jalr(&mut self, rs: u8, rd: u8) -> Result<()> {
        // Save return address (next_pc already points to delay slot + 4)
        self.set_reg(rd, self.next_pc);
        // Jump to address in rs
        self.next_pc = self.reg(rs);
        self.in_branch_delay = true;
        Ok(())
    }
}
