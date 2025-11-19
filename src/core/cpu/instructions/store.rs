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
use crate::core::memory::Bus;

impl CPU {
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
    pub(crate) fn op_sw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_sh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_sb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_swl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_swr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement SWR (Week 3)
        log::warn!(
            "SWR instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
        Ok(())
    }
}
