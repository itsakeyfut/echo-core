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

//! CPU execution tests

use super::super::*;

#[test]
fn test_system_step() {
    let mut system = System::new();

    // Write NOP instruction directly to BIOS memory for testing
    // NOP = 0x00000000
    system
        .bus_mut()
        .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

    let initial_pc = system.pc();
    system.step().unwrap();

    assert_eq!(system.pc(), initial_pc + 4);
    assert_eq!(system.cycles(), 1);
}

#[test]
fn test_system_step_n() {
    let mut system = System::new();

    // Fill BIOS with NOPs for testing
    for i in 0..10 {
        let offset = (i * 4) as usize;
        system
            .bus_mut()
            .write_bios_for_test(offset, &[0x00, 0x00, 0x00, 0x00]);
    }

    system.step_n(10).unwrap();

    assert_eq!(system.cycles(), 10);
}

#[test]
fn test_system_run_frame() {
    let mut system = System::new();

    // Create an infinite loop in BIOS for testing:
    // 0xBFC00000: j 0xBFC00000  (jump to self)
    // Encoding: opcode=2 (J), target=0x0F000000 (0xBFC00000 >> 2)
    // Full instruction: 0x0BF00000
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);

    // 0xBFC00004: nop (delay slot)
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();
    let initial_cycles = system.cycles();

    system.run_frame().unwrap();

    // Should execute approximately one frame worth of cycles (564,480)
    let cycles_executed = system.cycles() - initial_cycles;
    assert!(cycles_executed >= 564_480);
}

#[test]
fn test_run_frame_uses_timing_system() {
    let mut system = System::new();

    // Create an infinite loop in BIOS
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Run one frame
    system.run_frame().unwrap();

    // Verify that timing system's global counter was updated
    const CYCLES_PER_FRAME: u64 = 564_480;
    assert!(system.timing.global_tick_counter >= CYCLES_PER_FRAME);
    assert_eq!(system.cycles(), system.timing.global_tick_counter);
}

#[test]
fn test_frame_target_stops_execution() {
    let mut system = System::new();

    // Create an infinite loop in BIOS
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Run frame should set frame target and stop execution
    let initial_cycles = system.cycles();
    system.run_frame().unwrap();

    const CYCLES_PER_FRAME: u64 = 564_480;
    let cycles_executed = system.cycles() - initial_cycles;

    // Verify frame target mechanism works:
    // 1. Should execute at least the target number of cycles
    assert!(
        cycles_executed >= CYCLES_PER_FRAME,
        "Expected at least {} cycles, got {}",
        CYCLES_PER_FRAME,
        cycles_executed
    );

    // 2. Should stop execution (not run indefinitely)
    // The infinite loop test proves the frame target mechanism stopped execution
    // Note: May overshoot target due to instruction and event processing granularity
}
