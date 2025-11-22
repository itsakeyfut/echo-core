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

use super::super::decode::*;

#[test]
fn test_decode_r_type() {
    // ADD r3, r1, r2 -> 0x00221820
    let instr = 0x00221820;
    let (rs, rt, rd, shamt, funct) = decode_r_type(instr);
    assert_eq!(rs, 1);
    assert_eq!(rt, 2);
    assert_eq!(rd, 3);
    assert_eq!(shamt, 0);
    assert_eq!(funct, 0x20);
}

#[test]
fn test_decode_i_type() {
    // ADDI r2, r1, 100 -> 0x20220064
    let instr = 0x20220064;
    let (op, rs, rt, imm) = decode_i_type(instr);
    assert_eq!(op, 0x08);
    assert_eq!(rs, 1);
    assert_eq!(rt, 2);
    assert_eq!(imm, 100);
}

#[test]
fn test_decode_j_type() {
    // J 0x100000 -> 0x08040000
    let instr = 0x08040000;
    let (op, target) = decode_j_type(instr);
    assert_eq!(op, 0x02);
    assert_eq!(target, 0x040000);
}
