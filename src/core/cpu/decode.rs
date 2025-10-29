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

/// Decode R-type instruction
///
/// R-type instructions are used for register-to-register operations.
///
/// Format: | op (6) | rs (5) | rt (5) | rd (5) | shamt (5) | funct (6) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (rs, rt, rd, shamt, funct)
#[inline(always)]
pub(super) fn decode_r_type(instr: u32) -> (u8, u8, u8, u8, u8) {
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let rd = ((instr >> 11) & 0x1F) as u8;
    let shamt = ((instr >> 6) & 0x1F) as u8;
    let funct = (instr & 0x3F) as u8;
    (rs, rt, rd, shamt, funct)
}

/// Decode I-type instruction
///
/// I-type instructions are used for immediate operations, loads, stores, and branches.
///
/// Format: | op (6) | rs (5) | rt (5) | immediate (16) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (op, rs, rt, imm)
#[inline(always)]
pub(super) fn decode_i_type(instr: u32) -> (u8, u8, u8, u16) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let imm = (instr & 0xFFFF) as u16;
    (op, rs, rt, imm)
}

/// Decode J-type instruction
///
/// J-type instructions are used for jump operations.
///
/// Format: | op (6) | target (26) |
///
/// # Arguments
///
/// * `instr` - The 32-bit instruction
///
/// # Returns
///
/// Tuple of (op, target)
#[inline(always)]
pub(super) fn decode_j_type(instr: u32) -> (u8, u32) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let target = instr & 0x03FFFFFF;
    (op, target)
}
