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
fn test_cpu_initialization() {
    let cpu = CPU::new();
    assert_eq!(cpu.pc, 0xBFC00000);
    assert_eq!(cpu.next_pc, 0xBFC00004);
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn test_register_r0_is_hardwired() {
    let mut cpu = CPU::new();
    cpu.set_reg(0, 0xDEADBEEF);
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn test_register_read_write() {
    let mut cpu = CPU::new();
    cpu.set_reg(5, 0x12345678);
    assert_eq!(cpu.reg(5), 0x12345678);
}

#[test]
fn test_cpu_reset() {
    let mut cpu = CPU::new();

    // Modify some state
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.pc = 0x80000000;
    cpu.hi = 0x12345678;
    cpu.lo = 0x87654321;

    // Reset
    cpu.reset();

    // Verify all state is reset
    assert_eq!(cpu.reg(1), 0);
    assert_eq!(cpu.pc, 0xBFC00000);
    assert_eq!(cpu.next_pc, 0xBFC00004);
    assert_eq!(cpu.hi, 0);
    assert_eq!(cpu.lo, 0);
}

#[test]
fn test_multiple_registers() {
    let mut cpu = CPU::new();

    // Test writing to multiple registers
    for i in 1..32 {
        cpu.set_reg(i, i as u32 * 100);
    }

    // Verify all values
    for i in 1..32 {
        assert_eq!(cpu.reg(i), i as u32 * 100);
    }

    // r0 should still be 0
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn test_pc_accessor() {
    let cpu = CPU::new();
    assert_eq!(cpu.pc(), 0xBFC00000);
}
