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

use super::super::*;

#[test]
fn test_cop0_initialization() {
    let cpu = CPU::new();
    assert_eq!(cpu.cop0.regs[COP0::SR], 0x10900000);
    assert_eq!(cpu.cop0.regs[COP0::PRID], 0x00000002);
}

#[test]
fn test_rfe_restores_mode() {
    let mut cpu = CPU::new();

    // Simulate exception (shifts mode left by setting mode bits manually)
    cpu.cop0.regs[COP0::SR] = 0x0000000C; // Mode bits = 0b001100 (previous mode = 0b11)

    cpu.op_rfe(0).unwrap();

    // RFE should shift mode bits right
    let sr = cpu.cop0.regs[COP0::SR];
    assert_eq!(sr & 0x3F, 0x03); // Mode bits should be 0b000011
}

#[test]
fn test_mfc0_reads_cop0_register() {
    let mut cpu = CPU::new();

    // Set a value in COP0 SR register
    cpu.cop0.regs[COP0::SR] = 0x12345678;

    // MFC0 $t0, $12 (move SR to register 8)
    // Encoding: bits [25:21] = 0x00 (MFC0), bits [20:16] = rt, bits [15:11] = rd
    let instruction = (0x10 << 26) | (8 << 16) | (12 << 11);
    cpu.op_mfc0(instruction).unwrap();

    // Execute the load delay
    cpu.set_reg_delayed(9, 0); // Dummy operation to trigger delay

    // Register 8 should now have the SR value
    assert_eq!(cpu.reg(8), 0x12345678);
}

#[test]
fn test_mtc0_writes_cop0_register() {
    let mut cpu = CPU::new();

    // Set a value in general register
    cpu.set_reg(8, 0x87654321);

    // MTC0 $t0, $12 (move register 8 to SR)
    // Encoding: bits [25:21] = 0x04 (MTC0), bits [20:16] = rt, bits [15:11] = rd
    let instruction = (0x10 << 26) | (0x04 << 21) | (8 << 16) | (12 << 11);
    cpu.op_mtc0(instruction).unwrap();

    // SR should now have the value from register 8
    assert_eq!(cpu.cop0.regs[COP0::SR], 0x87654321);
}
