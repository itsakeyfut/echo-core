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
    pub(crate) fn op_lw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lh(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lhu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lbu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lwl(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
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
    pub(crate) fn op_lwr(&mut self, _instruction: u32, _bus: &mut Bus) -> Result<()> {
        // TODO: Implement LWR based on PSX-SPX documentation (Week 3)
        log::warn!(
            "LWR instruction not yet implemented at PC=0x{:08X}",
            self.pc
        );
        Ok(())
    }
}
