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

//! Interrupt masking tests
//!
//! Tests for interrupt mask register operations and interrupt
//! enable/disable functionality.

use super::super::*;

#[test]
fn test_interrupt_masking() {
    let mut ic = InterruptController::new();

    // Request interrupt but it's masked
    ic.request(interrupts::VBLANK);
    ic.write_mask(0); // Mask all
    assert!(!ic.is_pending());

    // Unmask
    ic.write_mask(interrupts::VBLANK as u32);
    assert!(ic.is_pending());
}

#[test]
fn test_partial_masking() {
    let mut ic = InterruptController::new();

    // Request multiple interrupts
    ic.request(interrupts::VBLANK | interrupts::TIMER0);

    // Only enable VBLANK
    ic.write_mask(interrupts::VBLANK as u32);

    // Should be pending because VBLANK is enabled
    assert!(ic.is_pending());

    // Change mask to only TIMER0
    ic.write_mask(interrupts::TIMER0 as u32);

    // Should still be pending because TIMER0 is now enabled
    assert!(ic.is_pending());

    // Mask both
    ic.write_mask(0);
    assert!(!ic.is_pending());
}

#[test]
fn test_mask_read_write() {
    let mut ic = InterruptController::new();

    ic.write_mask(0x1234);
    assert_eq!(ic.read_mask(), 0x1234);

    ic.write_mask(0xABCD);
    assert_eq!(ic.read_mask(), 0xABCD);
}

#[test]
fn test_no_pending_when_all_masked() {
    let mut ic = InterruptController::new();

    // Request all interrupts
    ic.request(0xFFFF);

    // Mask all interrupts
    ic.write_mask(0x0000);

    // Should not be pending
    assert!(!ic.is_pending());
}

#[test]
fn test_pending_with_any_unmasked() {
    let mut ic = InterruptController::new();

    // Request multiple interrupts
    ic.request(interrupts::VBLANK | interrupts::GPU | interrupts::TIMER0);

    // Enable only one of them
    ic.write_mask(interrupts::GPU as u32);

    // Should be pending
    assert!(ic.is_pending());
}
