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

//! Basic interrupt controller tests
//!
//! Tests for basic interrupt functionality including initialization,
//! interrupt requests, and status register operations.

use super::super::*;

#[test]
fn test_interrupt_request() {
    let mut ic = InterruptController::new();

    ic.request(interrupts::VBLANK);
    assert_eq!(ic.status, interrupts::VBLANK);
    assert_eq!(ic.read_status(), interrupts::VBLANK as u32);
}

#[test]
fn test_multiple_interrupt_requests() {
    let mut ic = InterruptController::new();

    ic.request(interrupts::VBLANK);
    ic.request(interrupts::TIMER0);

    assert_eq!(ic.status, interrupts::VBLANK | interrupts::TIMER0);
}

#[test]
fn test_status_read_write() {
    let mut ic = InterruptController::new();

    ic.request(0x00FF);
    assert_eq!(ic.read_status(), 0x00FF);

    // Writing all 1s should not clear anything (1s leave bits unchanged)
    ic.write_status(0xFFFF);
    assert_eq!(ic.read_status(), 0x00FF);

    // Writing all 0s should clear everything (0s clear bits)
    ic.write_status(0x0000);
    assert_eq!(ic.read_status(), 0x0000);
}

#[test]
fn test_all_interrupt_sources() {
    let mut ic = InterruptController::new();

    // Test all defined interrupt sources
    let all_interrupts = interrupts::VBLANK
        | interrupts::GPU
        | interrupts::CDROM
        | interrupts::DMA
        | interrupts::TIMER0
        | interrupts::TIMER1
        | interrupts::TIMER2
        | interrupts::CONTROLLER
        | interrupts::SIO
        | interrupts::SPU
        | interrupts::LIGHTPEN;

    ic.request(all_interrupts);
    ic.write_mask(all_interrupts as u32);

    assert!(ic.is_pending());
    assert_eq!(ic.read_status(), all_interrupts as u32);
}
