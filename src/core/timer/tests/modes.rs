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

//! Timer mode operation tests

use super::super::*;

#[test]
fn test_timer_reset_on_target() {
    let mut timer = TimerChannel::new(0);

    timer.write_target(50);
    timer.write_mode(0x0008); // Reset on target

    timer.tick(50, false);
    assert_eq!(timer.read_counter(), 0); // Should reset

    timer.tick(25, false);
    assert_eq!(timer.read_counter(), 25);
}

#[test]
fn test_timer_mode_write_resets_counter() {
    let mut timer = TimerChannel::new(0);

    // Count to 100
    timer.tick(100, false);
    assert_eq!(timer.read_counter(), 100);

    // Writing mode should reset counter
    timer.write_mode(0x0000);
    assert_eq!(timer.read_counter(), 0);
}

#[test]
fn test_timer_mode_read_resets_flags() {
    let mut timer = TimerChannel::new(0);

    timer.write_target(50);
    timer.write_mode(0x0010); // IRQ on target

    timer.tick(50, false);
    assert!(timer.reached_target);

    // Reading mode should reset flags
    let _mode = timer.read_mode();
    assert!(!timer.reached_target);
}
