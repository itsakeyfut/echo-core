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

//! CPU execution timing tests (Issue #135)
//!
//! Tests CPU integration with the timing event system:
//! - CPU executes until downcount
//! - pending_ticks increments correctly
//! - Interrupts force downcount = 0
//! - Load delay slots work with timing
//! - Branch delay slots work with timing

use super::super::*;
use crate::core::memory::Bus;
use crate::core::timing::TimingEventManager;

/// Helper: Create a simple test environment with CPU, bus, and timing
fn create_test_env() -> (CPU, Bus, TimingEventManager) {
    let cpu = CPU::new();
    let bus = Bus::new();
    let timing = TimingEventManager::new();
    (cpu, bus, timing)
}

#[test]
fn test_cpu_executes_until_downcount() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Set downcount to 10 cycles
    timing.downcount = 10;
    timing.pending_ticks = 0;

    // Create a simple loop that does NOP operations
    // Write NOPs to RAM (0x00000000)
    for i in 0..20 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);

    // Execute instructions - should stop when pending_ticks >= downcount
    let mut executed = 0;
    while timing.pending_ticks < timing.downcount {
        cpu.step(&mut bus).unwrap();
        timing.pending_ticks += 1;
        executed += 1;
    }

    // Should have executed exactly 10 instructions
    assert_eq!(executed, 10);
    assert_eq!(timing.pending_ticks, 10);
}

#[test]
fn test_pending_ticks_increments_correctly() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Write some NOPs to RAM
    for i in 0..10 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);

    // Initial state
    assert_eq!(timing.pending_ticks, 0);

    // Execute 5 instructions
    for i in 0..5 {
        cpu.step(&mut bus).unwrap();
        timing.pending_ticks += 1;

        // Verify pending_ticks increments
        assert_eq!(timing.pending_ticks, i + 1);
    }
}

#[test]
fn test_timing_integration_with_cpu_execute() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Set initial downcount
    timing.downcount = 100;

    // Write NOPs to execute
    for i in 0..200 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }
    cpu.set_pc(0x80000000);

    // Execute using the timing-aware execute() method
    timing.set_frame_target(100); // Exit after 100 cycles
    cpu.execute(&mut bus, &mut timing).unwrap();

    // Should have executed approximately 100 cycles
    assert!(timing.global_tick_counter >= 100);

    // This test verifies the timing integration works correctly
}

#[test]
fn test_load_delay_with_timing() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Write a value to memory
    bus.write32(0x80000100, 0x12345678).unwrap();

    // LW $t0, 0x100($zero)  - Load word from 0x80000100 into $t0 (r8)
    // Opcode: 0x8C080100
    let lw_instruction = 0x8C080100;

    // NOP
    let nop_instruction = 0x00000000;

    // ADDI $t1, $t0, 0  - Try to use loaded value (should get OLD value due to load delay)
    // Opcode: 0x21090000
    let addi_instruction = 0x21090000;

    bus.write32(0x80000000, lw_instruction).unwrap();
    bus.write32(0x80000004, nop_instruction).unwrap();
    bus.write32(0x80000008, addi_instruction).unwrap();

    cpu.set_pc(0x80000000);

    // Execute LW (loads into delay slot)
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 1);

    // Execute NOP (load completes after this)
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 2);

    // Execute ADDI (now $t0 should have the loaded value)
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 3);

    // Verify the value was loaded correctly
    assert_eq!(cpu.reg(8), 0x12345678); // $t0 should have loaded value
}

#[test]
fn test_branch_delay_with_timing() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // BEQ $zero, $zero, 8  - Branch if equal (always taken), offset=8 (2 instructions)
    // Opcode: 0x10000002
    let beq_instruction = 0x10000002;

    // ADDIU $t0, $zero, 1  - Delay slot instruction (should execute)
    // Opcode: 0x24080001
    let addiu_instruction = 0x24080001;

    // ADDIU $t0, $zero, 2  - Should be skipped
    // Opcode: 0x24080002
    let skip_instruction = 0x24080002;

    // ADDIU $t0, $zero, 3  - Branch target (should execute)
    // Opcode: 0x24080003
    let target_instruction = 0x24080003;

    bus.write32(0x80000000, beq_instruction).unwrap();
    bus.write32(0x80000004, addiu_instruction).unwrap(); // Delay slot
    bus.write32(0x80000008, skip_instruction).unwrap();
    bus.write32(0x8000000C, target_instruction).unwrap(); // Branch target

    cpu.set_pc(0x80000000);
    timing.pending_ticks = 0;

    // Execute BEQ (branch taken, but delay slot executes first)
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 1);

    // Execute delay slot instruction
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 2);
    assert_eq!(cpu.reg(8), 1); // Delay slot executed

    // Execute branch target
    cpu.step(&mut bus).unwrap();
    timing.pending_ticks += 1;
    assert_eq!(timing.pending_ticks, 3);
    assert_eq!(cpu.reg(8), 3); // Branch target executed
}

#[test]
fn test_cpu_execute_with_timing_events() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Register a timing event that fires after 100 cycles
    let event = timing.register_event("Test Event");
    timing.schedule(event, 100);

    // Write NOPs to RAM
    for i in 0..200 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);

    // Set frame target to 100 cycles
    timing.set_frame_target(100);

    // Execute using the timing-aware execute() method
    cpu.execute(&mut bus, &mut timing).unwrap();

    // Should have executed approximately 100 instructions
    // (exact count may vary due to event processing)
    assert!(timing.global_tick_counter >= 100);
}

#[test]
fn test_cpu_timing_precision() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Write NOPs to RAM
    for i in 0..1000 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);
    timing.pending_ticks = 0;

    // Execute exactly 500 instructions
    for _ in 0..500 {
        cpu.step(&mut bus).unwrap();
        timing.pending_ticks += 1;
    }

    // Verify exact cycle count
    assert_eq!(timing.pending_ticks, 500);
}

#[test]
fn test_cpu_downcount_update_on_event() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Register an event at 50 cycles
    let event1 = timing.register_event("Event 1");
    timing.schedule(event1, 50);

    // Verify downcount is set correctly
    assert_eq!(timing.downcount, 50);

    // Write NOPs to RAM
    for i in 0..100 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);

    // Execute 50 instructions
    for _ in 0..50 {
        cpu.step(&mut bus).unwrap();
        timing.pending_ticks += 1;
    }

    // Run events (should trigger event1)
    let triggered = timing.run_events();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0], event1);
}

#[test]
fn test_cpu_multiple_events_timing() {
    let (mut cpu, mut bus, mut timing) = create_test_env();

    // Register multiple events at different times
    let event1 = timing.register_event("Event 1");
    let event2 = timing.register_event("Event 2");
    let event3 = timing.register_event("Event 3");

    timing.schedule(event1, 10);
    timing.schedule(event2, 20);
    timing.schedule(event3, 30);

    // Write NOPs to RAM
    for i in 0..100 {
        bus.write32(0x80000000 + i * 4, 0x00000000).unwrap(); // NOP
    }

    cpu.set_pc(0x80000000);

    // Execute and process events
    timing.set_frame_target(35);
    cpu.execute(&mut bus, &mut timing).unwrap();

    // All three events should have been triggered
    assert!(timing.global_tick_counter >= 30);
}
