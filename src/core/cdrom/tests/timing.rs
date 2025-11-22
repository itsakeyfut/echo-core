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

//! Timing event tests (Issue #132 Testing Requirements)

use super::super::*;
use crate::core::timing::TimingEventManager;

#[test]
fn test_getstat_command_timing() {
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;

    // Write GetStat command
    cdrom.write_register(CDROM::REG_DATA, 0x01);

    // Command should be queued, not executed
    assert!(cdrom.response_fifo.is_empty());
    assert!(cdrom.command_to_schedule.is_some());

    // Process to schedule the command
    cdrom.process_events(&mut timing, &[]);

    // Advance time less than ACK delay (5000 cycles)
    timing.pending_ticks = 4000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Response should NOT be available yet
    assert!(cdrom.response_fifo.is_empty());

    // Advance time past ACK delay
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Now response should be available (INT3)
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04); // INT3
}

#[test]
fn test_getid_command_timing_multi_stage() {
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;

    // Load a disc
    cdrom.disc = Some(crate::core::cdrom::DiscImage::new_dummy());

    // Write GetID command
    cdrom.write_register(CDROM::REG_DATA, 0x1A);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay (~5000 cycles)
    timing.pending_ticks = 6000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // First response (INT3) should be available
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04); // INT3

    // Clear interrupt and response
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: Second response delay (~33000 cycles)
    timing.pending_ticks = 34000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery (scheduled with minimum delay)
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Second response (INT2) should be available with disc info
    assert!(!cdrom.response_fifo.is_empty());
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02); // INT2

    // Response should contain disc info (8 bytes)
    assert!(cdrom.response_fifo.len() >= 8);
}

#[test]
fn test_readtoc_command_timing() {
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);
    cdrom.status.motor_on = true;
    cdrom.disc = Some(crate::core::cdrom::DiscImage::new_dummy());

    // Write ReadTOC command
    cdrom.write_register(CDROM::REG_DATA, 0x1E);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay
    timing.pending_ticks = 6000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT3 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: TOC read delay (~500000 cycles = ~15ms)
    timing.pending_ticks = 510000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT2 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    assert!(!cdrom.response_fifo.is_empty());
}

#[test]
fn test_init_command_timing() {
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);

    // Write Init command
    cdrom.write_register(CDROM::REG_DATA, 0x0A);

    // Process to schedule
    cdrom.process_events(&mut timing, &[]);

    // Stage 1: ACK delay (Init has longer delay: ~20000 cycles)
    timing.pending_ticks = 25000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT3 should be triggered, motor should be on
    assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
    assert!(cdrom.status.motor_on);
    cdrom.response_fifo.clear();
    cdrom.interrupt_flag = 0;

    // Stage 2: Init complete delay (~70000 cycles)
    timing.pending_ticks = 75000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Stage 3: Async interrupt delivery
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // INT2 should be triggered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
}

#[test]
fn test_interrupt_delivery_delays() {
    let mut cdrom = CDROM::new();
    let mut timing = TimingEventManager::new();

    cdrom.register_events(&mut timing);

    // Trigger first interrupt
    cdrom.async_response_fifo.push_back(0x02);
    cdrom.schedule_async_interrupt(2, &mut timing);

    // Advance time
    timing.pending_ticks = 2000;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // First interrupt should be delivered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    let first_time = cdrom.last_interrupt_time;

    // Clear interrupt
    cdrom.interrupt_flag = 0;

    // Try to trigger second interrupt immediately
    cdrom.async_response_fifo.push_back(0x02);
    cdrom.pending_async_interrupt = 0; // Reset for new interrupt
    cdrom.schedule_async_interrupt(2, &mut timing);

    // Should be delayed by MINIMUM_INTERRUPT_DELAY (1000 cycles)
    timing.pending_ticks = 500;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Should NOT be delivered yet (too soon)
    assert_eq!(cdrom.interrupt_flag, 0);

    // Advance past minimum delay
    timing.pending_ticks = 600;
    let triggered = timing.run_events();
    cdrom.process_events(&mut timing, &triggered);

    // Now should be delivered
    assert_eq!(cdrom.interrupt_flag & 0x02, 0x02);
    assert!(cdrom.last_interrupt_time > first_time);
}
