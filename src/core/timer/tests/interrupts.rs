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

//! Timer interrupt operation tests

use super::super::*;

#[test]
fn test_timer_target_interrupt() {
    let mut timer = TimerChannel::new(0);

    // Set target to 100
    timer.write_target(100);

    // Enable target IRQ
    timer.write_mode(0x0010); // IRQ on target

    // Count to target
    let irq = timer.tick(100, false);

    assert!(irq);
    assert!(timer.reached_target);
    assert!(timer.irq_pending());
}

#[test]
fn test_timer_irq_one_shot() {
    let mut timer = TimerChannel::new(0);

    timer.write_target(10);
    // IRQ on target, one-shot mode (IRQ repeat bit = 0)
    timer.write_mode(0x0010);

    // First target hit should trigger IRQ
    let irq1 = timer.tick(10, false);
    assert!(irq1);

    // Continue counting past target with reset disabled
    timer.write_mode(0x0010); // Reset counter
    let irq2 = timer.tick(20, false);
    assert!(irq2); // Should trigger again at target

    // But IRQ flag should not be set again in one-shot mode
    // unless it was cleared
}

#[test]
fn test_timer_irq_repeat() {
    let mut timer = TimerChannel::new(0);

    timer.write_target(10);
    // IRQ on target, repeat mode
    timer.write_mode(0x0050); // bit 4 (target IRQ) + bit 6 (repeat)

    let irq1 = timer.tick(10, false);
    assert!(irq1);

    // Even if IRQ is still pending, repeat mode should allow re-trigger
    timer.write_mode(0x0058); // Reset counter + repeat + target IRQ
    let irq2 = timer.tick(10, false);
    assert!(irq2);
}
