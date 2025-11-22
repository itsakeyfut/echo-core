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

//! Basic timer functionality tests (initialization, counting, overflow)

use super::super::*;

#[test]
fn test_timer_basic_counting() {
    let mut timer = TimerChannel::new(0);

    timer.tick(100, false);
    assert_eq!(timer.read_counter(), 100);

    timer.tick(50, false);
    assert_eq!(timer.read_counter(), 150);
}

#[test]
fn test_timer_overflow() {
    let mut timer = TimerChannel::new(0);

    timer.write_mode(0x0020); // IRQ on max
    timer.write_counter(0xFFFE);

    let irq = timer.tick(1, false);
    assert!(irq);
    assert!(timer.reached_max);
}

#[test]
fn test_timers_tick_all_channels() {
    let mut timers = Timers::new();

    // Configure timer 0
    timers.channel_mut(0).write_target(100);
    timers.channel_mut(0).write_mode(0x0010); // IRQ on target

    // Configure timer 1
    timers.channel_mut(1).write_target(50);
    timers.channel_mut(1).write_mode(0x0010);

    // Configure timer 2
    timers.channel_mut(2).write_target(200);
    timers.channel_mut(2).write_mode(0x0010);

    // Tick all timers
    let irqs = timers.tick(100, false, false);

    // Timer 0 should have hit target
    assert!(irqs[0]);
    // Timer 1 should have hit target (50 < 100)
    assert!(irqs[1]);
    // Timer 2 should not have hit target yet
    assert!(!irqs[2]);
}

#[test]
fn test_timer_clock_source_division() {
    let mut timers = Timers::new();

    // Timer 2 with clock source = 2 (system/8 mode, bit 9 set)
    timers.channel_mut(2).write_mode(0x0200); // Clock source bit 9

    timers.tick(80, false, false);

    // With /8 divider, 80 cycles should advance counter by 10
    assert_eq!(timers.channel(2).read_counter(), 10);
}
