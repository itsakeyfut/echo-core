// SPDX-License-Identifier: MPL-2.0
//! Basic controller functionality tests
//!
//! Tests for controller initialization and basic state management.

use super::super::*;

#[test]
fn test_controller_initialization() {
    let controller = Controller::new();

    // All buttons should be released initially (0xFFFF - active low)
    assert_eq!(controller.get_buttons(), 0xFFFF);
    assert_eq!(controller.state, SerialState::Idle);
}
