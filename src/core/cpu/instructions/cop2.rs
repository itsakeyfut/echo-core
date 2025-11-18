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

//! COP2 (GTE) instruction implementations
//!
//! This module implements CPU instructions that interact with Coprocessor 2
//! (the Geometry Transformation Engine).

use super::CPU;
use crate::core::error::Result;

impl CPU {
    /// MFC2: Move From Coprocessor 2 (data register)
    ///
    /// Reads a value from a GTE data register and stores it in a CPU register.
    ///
    /// Format: MFC2 rt, rd
    /// - rt: CPU destination register (bits [20:16])
    /// - rd: GTE data register (bits [15:11])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Example
    ///
    /// ```text
    /// MFC2 r5, [25]  // r5 = GTE.data[25] (MAC1)
    /// ```
    pub(super) fn op_mfc2(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.gte.read_data(rd as usize);
        self.set_reg_delayed(rt, value as u32);

        log::trace!("MFC2: r{} = GTE.data[{}] (0x{:08X})", rt, rd, value);

        Ok(())
    }

    /// CFC2: Move From Coprocessor 2 (control register)
    ///
    /// Reads a value from a GTE control register and stores it in a CPU register.
    ///
    /// Format: CFC2 rt, rd
    /// - rt: CPU destination register (bits [20:16])
    /// - rd: GTE control register (bits [15:11])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Example
    ///
    /// ```text
    /// CFC2 r5, [26]  // r5 = GTE.control[26] (H - projection plane distance)
    /// ```
    pub(super) fn op_cfc2(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.gte.read_control(rd as usize);
        self.set_reg_delayed(rt, value as u32);

        log::trace!("CFC2: r{} = GTE.control[{}] (0x{:08X})", rt, rd, value);

        Ok(())
    }

    /// MTC2: Move To Coprocessor 2 (data register)
    ///
    /// Writes a value from a CPU register to a GTE data register.
    ///
    /// Format: MTC2 rt, rd
    /// - rt: CPU source register (bits [20:16])
    /// - rd: GTE data register (bits [15:11])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Example
    ///
    /// ```text
    /// MTC2 r5, [0]  // GTE.data[0] = r5 (set VXY0)
    /// ```
    pub(super) fn op_mtc2(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.reg(rt) as i32;
        self.gte.write_data(rd as usize, value);

        log::trace!("MTC2: GTE.data[{}] = r{} (0x{:08X})", rd, rt, value);

        Ok(())
    }

    /// CTC2: Move To Coprocessor 2 (control register)
    ///
    /// Writes a value from a CPU register to a GTE control register.
    ///
    /// Format: CTC2 rt, rd
    /// - rt: CPU source register (bits [20:16])
    /// - rd: GTE control register (bits [15:11])
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Example
    ///
    /// ```text
    /// CTC2 r5, [26]  // GTE.control[26] = r5 (set H - projection plane distance)
    /// ```
    pub(super) fn op_ctc2(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.reg(rt) as i32;
        self.gte.write_control(rd as usize, value);

        log::trace!("CTC2: GTE.control[{}] = r{} (0x{:08X})", rd, rt, value);

        Ok(())
    }

    /// Execute GTE command
    ///
    /// Executes a GTE transformation/calculation command.
    ///
    /// Format: COP2 command
    /// - Bits [24:0]: GTE command word
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Common Commands
    ///
    /// - 0x0180001: RTPS (Rotate, Translate, Perspective transform Single)
    /// - 0x0280030: RTPT (Rotate, Translate, Perspective transform Triple)
    /// - 0x01400006: NCLIP (Normal clipping)
    /// - 0x0400012: MVMVA (Matrix-Vector multiply with vector addition)
    pub(super) fn op_gte_command(&mut self, instruction: u32) -> Result<()> {
        // The lower 25 bits contain the GTE command
        let command = instruction & 0x01FFFFFF;

        log::trace!("GTE command: 0x{:08X}", command);

        self.gte.execute(command);

        Ok(())
    }
}
