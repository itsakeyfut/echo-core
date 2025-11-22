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
use crate::core::memory::Bus;

#[test]
fn test_load_delay_slot() {
    let mut cpu = CPU::new();
    cpu.set_reg_delayed(3, 100);

    // Value not yet visible
    assert_eq!(cpu.reg(3), 0);

    // Execute load delay
    cpu.set_reg_delayed(4, 200);

    // Now r3 should have the value
    assert_eq!(cpu.reg(3), 100);
}

#[test]
fn test_load_delay_chain() {
    let mut cpu = CPU::new();

    // Chain multiple load delays
    cpu.set_reg_delayed(1, 10);
    assert_eq!(cpu.reg(1), 0);

    cpu.set_reg_delayed(2, 20);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 0);

    cpu.set_reg_delayed(3, 30);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 20);
    assert_eq!(cpu.reg(3), 0);

    // Final load delay to flush
    cpu.set_reg_delayed(4, 40);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 20);
    assert_eq!(cpu.reg(3), 30);
    assert_eq!(cpu.reg(4), 0);
}

#[test]
fn test_load_delay_r0_ignored() {
    let mut cpu = CPU::new();

    // Load delay to r0 should be ignored
    cpu.set_reg_delayed(0, 100);
    cpu.set_reg_delayed(1, 200);

    // r0 should still be 0, r1 should be 0 (delay not executed yet)
    assert_eq!(cpu.reg(0), 0);
    assert_eq!(cpu.reg(1), 0);

    // Execute another load to flush
    cpu.set_reg_delayed(2, 300);
    assert_eq!(cpu.reg(0), 0);
    assert_eq!(cpu.reg(1), 200);
}

#[test]
fn test_load_delay_slot_interaction() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up memory
    bus.write32(0x80000000, 0x11111111).unwrap();
    bus.write32(0x80000004, 0x22222222).unwrap();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0x80000004);

    // LW r3, 0(r1) - Load first value
    let instr1 = 0x8C230000;
    cpu.op_lw(instr1, &mut bus).unwrap();

    // r3 not yet available
    assert_eq!(cpu.reg(3), 0);

    // LW r4, 0(r2) - Load second value, flushes first delay
    let instr2 = 0x8C440000;
    cpu.op_lw(instr2, &mut bus).unwrap();

    // Now r3 has first value, r4 still waiting
    assert_eq!(cpu.reg(3), 0x11111111);
    assert_eq!(cpu.reg(4), 0);

    // Another instruction flushes second delay
    cpu.set_reg_delayed(5, 0);
    assert_eq!(cpu.reg(4), 0x22222222);
}
