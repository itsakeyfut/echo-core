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

//! MIPS instruction disassembler for debugging
//!
//! Converts binary instruction encodings to human-readable assembly mnemonics.

use super::decode::{decode_i_type, decode_j_type, decode_r_type};

/// Instruction disassembler
///
/// Converts 32-bit MIPS instruction encodings to human-readable assembly format.
///
/// # Example
/// ```
/// use echo_core::core::cpu::Disassembler;
///
/// let instruction = 0x00000000; // NOP
/// let disasm = Disassembler::disassemble(instruction, 0xBFC00000);
/// assert_eq!(disasm, "nop");
/// ```
pub struct Disassembler;

impl Disassembler {
    /// Disassemble a single instruction to human-readable format
    ///
    /// # Arguments
    ///
    /// * `instruction` - The 32-bit instruction to disassemble
    /// * `pc` - Program counter (used for jump target calculation)
    ///
    /// # Returns
    ///
    /// String containing the disassembled instruction
    ///
    /// # Example
    /// ```
    /// use echo_core::core::cpu::Disassembler;
    ///
    /// let instruction = 0x3C011234; // LUI r1, 0x1234
    /// let disasm = Disassembler::disassemble(instruction, 0xBFC00000);
    /// assert_eq!(disasm, "lui r1, 0x1234");
    /// ```
    pub fn disassemble(instruction: u32, pc: u32) -> String {
        let opcode = instruction >> 26;

        match opcode {
            0x00 => Self::disasm_special(instruction),
            0x01 => Self::disasm_regimm(instruction),
            0x02 => {
                let (_, target) = decode_j_type(instruction);
                let addr = (pc & 0xF000_0000) | (target << 2);
                format!("j 0x{:08X}", addr)
            }
            0x03 => {
                let (_, target) = decode_j_type(instruction);
                let addr = (pc & 0xF000_0000) | (target << 2);
                format!("jal 0x{:08X}", addr)
            }
            0x04 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("beq r{}, r{}, {}", rs, rt, (imm as i16))
            }
            0x05 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("bne r{}, r{}, {}", rs, rt, (imm as i16))
            }
            0x06 => {
                let (_, rs, _, imm) = decode_i_type(instruction);
                format!("blez r{}, {}", rs, (imm as i16))
            }
            0x07 => {
                let (_, rs, _, imm) = decode_i_type(instruction);
                format!("bgtz r{}, {}", rs, (imm as i16))
            }
            0x08 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("addi r{}, r{}, {}", rt, rs, (imm as i16))
            }
            0x09 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("addiu r{}, r{}, {}", rt, rs, (imm as i16))
            }
            0x0A => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("slti r{}, r{}, {}", rt, rs, (imm as i16))
            }
            0x0B => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("sltiu r{}, r{}, {}", rt, rs, (imm as i16))
            }
            0x0C => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("andi r{}, r{}, 0x{:04X}", rt, rs, imm)
            }
            0x0D => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("ori r{}, r{}, 0x{:04X}", rt, rs, imm)
            }
            0x0E => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("xori r{}, r{}, 0x{:04X}", rt, rs, imm)
            }
            0x0F => {
                let (_, _, rt, imm) = decode_i_type(instruction);
                format!("lui r{}, 0x{:04X}", rt, imm)
            }
            0x10 => Self::disasm_cop0(instruction),
            0x20 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lb r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x21 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lh r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x22 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lwl r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x23 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lw r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x24 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lbu r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x25 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lhu r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x26 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("lwr r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x28 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("sb r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x29 => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("sh r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x2A => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("swl r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x2B => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("sw r{}, {}(r{})", rt, (imm as i16), rs)
            }
            0x2E => {
                let (_, rs, rt, imm) = decode_i_type(instruction);
                format!("swr r{}, {}(r{})", rt, (imm as i16), rs)
            }
            _ => format!("??? 0x{:08X}", instruction),
        }
    }

    /// Disassemble SPECIAL (opcode 0x00) instruction
    fn disasm_special(instruction: u32) -> String {
        let (rs, rt, rd, shamt, funct) = decode_r_type(instruction);

        match funct {
            0x00 if instruction == 0 => "nop".to_string(),
            0x00 => format!("sll r{}, r{}, {}", rd, rt, shamt),
            0x02 => format!("srl r{}, r{}, {}", rd, rt, shamt),
            0x03 => format!("sra r{}, r{}, {}", rd, rt, shamt),
            0x04 => format!("sllv r{}, r{}, r{}", rd, rt, rs),
            0x06 => format!("srlv r{}, r{}, r{}", rd, rt, rs),
            0x07 => format!("srav r{}, r{}, r{}", rd, rt, rs),
            0x08 => format!("jr r{}", rs),
            0x09 => {
                if rd == 31 {
                    format!("jalr r{}", rs)
                } else {
                    format!("jalr r{}, r{}", rd, rs)
                }
            }
            0x0C => "syscall".to_string(),
            0x0D => "break".to_string(),
            0x10 => format!("mfhi r{}", rd),
            0x11 => format!("mthi r{}", rs),
            0x12 => format!("mflo r{}", rd),
            0x13 => format!("mtlo r{}", rs),
            0x18 => format!("mult r{}, r{}", rs, rt),
            0x19 => format!("multu r{}, r{}", rs, rt),
            0x1A => format!("div r{}, r{}", rs, rt),
            0x1B => format!("divu r{}, r{}", rs, rt),
            0x20 => format!("add r{}, r{}, r{}", rd, rs, rt),
            0x21 => format!("addu r{}, r{}, r{}", rd, rs, rt),
            0x22 => format!("sub r{}, r{}, r{}", rd, rs, rt),
            0x23 => format!("subu r{}, r{}, r{}", rd, rs, rt),
            0x24 => format!("and r{}, r{}, r{}", rd, rs, rt),
            0x25 => format!("or r{}, r{}, r{}", rd, rs, rt),
            0x26 => format!("xor r{}, r{}, r{}", rd, rs, rt),
            0x27 => format!("nor r{}, r{}, r{}", rd, rs, rt),
            0x2A => format!("slt r{}, r{}, r{}", rd, rs, rt),
            0x2B => format!("sltu r{}, r{}, r{}", rd, rs, rt),
            _ => format!("??? 0x{:08X}", instruction),
        }
    }

    /// Disassemble REGIMM (opcode 0x01) instruction
    fn disasm_regimm(instruction: u32) -> String {
        let (_, rs, rt, imm) = decode_i_type(instruction);

        match rt {
            0x00 => format!("bltz r{}, {}", rs, (imm as i16)),
            0x01 => format!("bgez r{}, {}", rs, (imm as i16)),
            0x10 => format!("bltzal r{}, {}", rs, (imm as i16)),
            0x11 => format!("bgezal r{}, {}", rs, (imm as i16)),
            _ => format!("??? 0x{:08X}", instruction),
        }
    }

    /// Disassemble COP0 (coprocessor 0) instruction
    fn disasm_cop0(instruction: u32) -> String {
        let rs = (instruction >> 21) & 0x1F;
        let rt = (instruction >> 16) & 0x1F;
        let rd = (instruction >> 11) & 0x1F;

        match rs {
            0x00 => format!("mfc0 r{}, cop0r{}", rt, rd),
            0x04 => format!("mtc0 r{}, cop0r{}", rt, rd),
            0x10 => {
                let funct = instruction & 0x3F;
                match funct {
                    0x10 => "rfe".to_string(),
                    _ => format!("??? 0x{:08X}", instruction),
                }
            }
            _ => format!("??? 0x{:08X}", instruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disasm_nop() {
        let result = Disassembler::disassemble(0x00000000, 0);
        assert_eq!(result, "nop");
    }

    #[test]
    fn test_disasm_lui() {
        let result = Disassembler::disassemble(0x3C011234, 0); // LUI r1, 0x1234
        assert_eq!(result, "lui r1, 0x1234");
    }

    #[test]
    fn test_disasm_addiu() {
        let result = Disassembler::disassemble(0x24220042, 0); // ADDIU r2, r1, 66
        assert_eq!(result, "addiu r2, r1, 66");
    }

    #[test]
    fn test_disasm_or() {
        let result = Disassembler::disassemble(0x00411825, 0); // OR r3, r2, r1
        assert_eq!(result, "or r3, r2, r1");
    }

    #[test]
    fn test_disasm_sw() {
        let result = Disassembler::disassemble(0xAC220000, 0); // SW r2, 0(r1)
        assert_eq!(result, "sw r2, 0(r1)");
    }

    #[test]
    fn test_disasm_lw() {
        let result = Disassembler::disassemble(0x8C220004, 0); // LW r2, 4(r1)
        assert_eq!(result, "lw r2, 4(r1)");
    }

    #[test]
    fn test_disasm_j() {
        let result = Disassembler::disassemble(0x0BF00000, 0xBFC00000); // J 0xBFC00000
        assert_eq!(result, "j 0xBFC00000");
    }

    #[test]
    fn test_disasm_jr() {
        let result = Disassembler::disassemble(0x03E00008, 0); // JR r31
        assert_eq!(result, "jr r31");
    }

    #[test]
    fn test_disasm_unknown() {
        let result = Disassembler::disassemble(0xFFFFFFFF, 0);
        assert!(result.starts_with("???"));
    }
}
