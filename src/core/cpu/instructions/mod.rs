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

//! CPU instruction implementations
//!
//! This module contains all MIPS R3000A instruction implementations,
//! organized by instruction type for better maintainability.

use super::decode::decode_r_type;
use super::CPU;
use crate::core::error::Result;
use crate::core::memory::Bus;

// Instruction modules organized by type
mod arithmetic;
mod branch;
mod cop0;
mod exception;
mod jump;
mod load;
mod logical;
mod multiply;
mod shift;
mod store;

impl CPU {
    /// Decode and execute the current instruction
    ///
    /// This method dispatches the instruction to the appropriate handler
    /// based on its opcode (upper 6 bits).
    ///
    /// # Arguments
    ///
    /// * `bus` - Memory bus for memory operations
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if execution fails
    pub(super) fn execute_instruction(&mut self, bus: &mut Bus) -> Result<()> {
        let instruction = self.current_instruction;

        // Extract opcode (upper 6 bits)
        let opcode = instruction >> 26;

        match opcode {
            0x00 => self.execute_special(instruction, bus),
            0x01 => self.execute_bcondz(instruction),
            0x02 => self.op_j(instruction),         // J
            0x03 => self.op_jal(instruction),       // JAL
            0x04 => self.op_beq(instruction),       // BEQ
            0x05 => self.op_bne(instruction),       // BNE
            0x06 => self.op_blez(instruction),      // BLEZ
            0x07 => self.op_bgtz(instruction),      // BGTZ
            0x08 => self.op_addi(instruction),      // ADDI
            0x09 => self.op_addiu(instruction),     // ADDIU
            0x0A => self.op_slti(instruction),      // SLTI
            0x0B => self.op_sltiu(instruction),     // SLTIU
            0x0C => self.op_andi(instruction),      // ANDI
            0x0D => self.op_ori(instruction),       // ORI
            0x0E => self.op_xori(instruction),      // XORI
            0x0F => self.op_lui(instruction),       // LUI
            0x10 => self.execute_cop0(instruction), // COP0
            0x20 => self.op_lb(instruction, bus),   // LB
            0x21 => self.op_lh(instruction, bus),   // LH
            0x22 => self.op_lwl(instruction, bus),  // LWL
            0x23 => self.op_lw(instruction, bus),   // LW
            0x24 => self.op_lbu(instruction, bus),  // LBU
            0x25 => self.op_lhu(instruction, bus),  // LHU
            0x26 => self.op_lwr(instruction, bus),  // LWR
            0x28 => self.op_sb(instruction, bus),   // SB
            0x29 => self.op_sh(instruction, bus),   // SH
            0x2A => self.op_swl(instruction, bus),  // SWL
            0x2B => self.op_sw(instruction, bus),   // SW
            0x2E => self.op_swr(instruction, bus),  // SWR
            _ => {
                log::warn!(
                    "Unimplemented opcode: 0x{:02X} at PC=0x{:08X}",
                    opcode,
                    self.pc
                );
                Ok(())
            }
        }
    }

    /// Handle SPECIAL instructions (opcode 0x00)
    ///
    /// SPECIAL instructions use the lower 6 bits (funct field) to determine
    /// the specific operation.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    /// * `bus` - Memory bus (for future use)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    pub(super) fn execute_special(&mut self, instruction: u32, _bus: &mut Bus) -> Result<()> {
        let (rs, rt, rd, shamt, funct) = decode_r_type(instruction);

        match funct {
            0x00 => self.op_sll(rt, rd, shamt),   // SLL
            0x02 => self.op_srl(rt, rd, shamt),   // SRL
            0x03 => self.op_sra(rt, rd, shamt),   // SRA
            0x04 => self.op_sllv(rs, rt, rd),     // SLLV
            0x06 => self.op_srlv(rs, rt, rd),     // SRLV
            0x07 => self.op_srav(rs, rt, rd),     // SRAV
            0x08 => self.op_jr(rs),               // JR
            0x09 => self.op_jalr(rs, rd),         // JALR
            0x0C => self.op_syscall(instruction), // SYSCALL
            0x0D => self.op_break(instruction),   // BREAK
            0x10 => self.op_mfhi(rd),             // MFHI
            0x11 => self.op_mthi(rs),             // MTHI
            0x12 => self.op_mflo(rd),             // MFLO
            0x13 => self.op_mtlo(rs),             // MTLO
            0x18 => self.op_mult(rs, rt),         // MULT
            0x19 => self.op_multu(rs, rt),        // MULTU
            0x1A => self.op_div(rs, rt),          // DIV
            0x1B => self.op_divu(rs, rt),         // DIVU
            0x20 => self.op_add(rs, rt, rd),      // ADD
            0x21 => self.op_addu(rs, rt, rd),     // ADDU
            0x22 => self.op_sub(rs, rt, rd),      // SUB
            0x23 => self.op_subu(rs, rt, rd),     // SUBU
            0x24 => self.op_and(rs, rt, rd),      // AND
            0x25 => self.op_or(rs, rt, rd),       // OR
            0x26 => self.op_xor(rs, rt, rd),      // XOR
            0x27 => self.op_nor(rs, rt, rd),      // NOR
            0x2A => self.op_slt(rs, rt, rd),      // SLT
            0x2B => self.op_sltu(rs, rt, rd),     // SLTU
            _ => {
                log::warn!(
                    "Unimplemented SPECIAL function: 0x{:02X} at PC=0x{:08X}",
                    funct,
                    self.pc
                );
                Ok(())
            }
        }
    }

    /// Handle COP0 instructions (opcode 0x10)
    ///
    /// COP0 instructions are used to interact with Coprocessor 0 (System Control).
    ///
    /// # Arguments
    ///
    /// * `instruction` - The full 32-bit instruction
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    fn execute_cop0(&mut self, instruction: u32) -> Result<()> {
        // COP0 sub-opcode is in bits [25:21]
        let sub_op = (instruction >> 21) & 0x1F;

        match sub_op {
            0x00 => self.op_mfc0(instruction), // MFC0
            0x04 => self.op_mtc0(instruction), // MTC0
            0x10 => {
                // COP0 function field (bits [5:0])
                let funct = instruction & 0x3F;
                match funct {
                    0x10 => self.op_rfe(instruction), // RFE
                    _ => {
                        log::warn!(
                            "Unimplemented COP0 function: 0x{:02X} at PC=0x{:08X}",
                            funct,
                            self.pc
                        );
                        Ok(())
                    }
                }
            }
            _ => {
                log::warn!(
                    "Unimplemented COP0 sub-opcode: 0x{:02X} at PC=0x{:08X}",
                    sub_op,
                    self.pc
                );
                Ok(())
            }
        }
    }
}
