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

//! Interrupt acknowledgment tests
//!
//! Tests for interrupt acknowledgment operations, including
//! clearing individual and multiple interrupts.

use super::super::*;

#[test]
fn test_interrupt_acknowledge() {
    let mut ic = InterruptController::new();

    ic.request(interrupts::VBLANK);
    ic.write_mask(interrupts::VBLANK as u32);

    assert!(ic.is_pending());

    // Acknowledge by writing 0 to the bit we want to clear (write inverted mask)
    ic.write_status(!interrupts::VBLANK as u32);

    assert!(!ic.is_pending());
    assert_eq!(ic.read_status(), 0);
}

#[test]
fn test_acknowledge_specific_interrupt() {
    let mut ic = InterruptController::new();

    // Request two interrupts
    ic.request(interrupts::VBLANK | interrupts::TIMER0);
    ic.write_mask(0xFFFF); // Enable all

    assert!(ic.is_pending());

    // Acknowledge only VBLANK (write 0 to VBLANK bit, 1 to others)
    ic.write_status(!interrupts::VBLANK as u32);

    // TIMER0 should still be pending
    assert!(ic.is_pending());
    assert_eq!(ic.read_status(), interrupts::TIMER0 as u32);
}
