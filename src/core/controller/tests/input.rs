// SPDX-License-Identifier: MPL-2.0
//! Input handling tests
//!
//! Tests for button press/release operations and input state management.

use super::super::*;

#[test]
fn test_button_press_release() {
    let mut controller = Controller::new();

    // All buttons should be released initially (0xFFFF)
    assert_eq!(controller.get_buttons(), 0xFFFF);

    // Press CROSS
    controller.press_button(buttons::CROSS);
    assert_eq!(controller.get_buttons() & buttons::CROSS, 0);
    assert_ne!(controller.get_buttons(), 0xFFFF);

    // Release CROSS
    controller.release_button(buttons::CROSS);
    assert_eq!(controller.get_buttons(), 0xFFFF);
}

#[test]
fn test_multiple_buttons() {
    let mut controller = Controller::new();

    // Press multiple buttons
    controller.press_button(buttons::CROSS);
    controller.press_button(buttons::CIRCLE);
    controller.press_button(buttons::START);

    // Check all are pressed (0)
    assert_eq!(controller.get_buttons() & buttons::CROSS, 0);
    assert_eq!(controller.get_buttons() & buttons::CIRCLE, 0);
    assert_eq!(controller.get_buttons() & buttons::START, 0);

    // Other buttons should still be released (1)
    assert_ne!(controller.get_buttons() & buttons::SQUARE, 0);
    assert_ne!(controller.get_buttons() & buttons::SELECT, 0);
}

#[test]
fn test_set_button_state() {
    let mut controller = Controller::new();

    // Press via set_button_state
    controller.set_button_state(buttons::TRIANGLE, true);
    assert_eq!(controller.get_buttons() & buttons::TRIANGLE, 0);

    // Release via set_button_state
    controller.set_button_state(buttons::TRIANGLE, false);
    assert_eq!(
        controller.get_buttons() & buttons::TRIANGLE,
        buttons::TRIANGLE
    );
}

#[test]
fn test_all_button_definitions() {
    let mut controller = Controller::new();

    // Test each button individually
    let all_buttons = [
        buttons::SELECT,
        buttons::L3,
        buttons::R3,
        buttons::START,
        buttons::UP,
        buttons::RIGHT,
        buttons::DOWN,
        buttons::LEFT,
        buttons::L2,
        buttons::R2,
        buttons::L1,
        buttons::R1,
        buttons::TRIANGLE,
        buttons::CIRCLE,
        buttons::CROSS,
        buttons::SQUARE,
    ];

    for &button in &all_buttons {
        controller.press_button(button);
        assert_eq!(
            controller.get_buttons() & button,
            0,
            "Button {:04X} should be pressed",
            button
        );

        controller.release_button(button);
        assert_eq!(
            controller.get_buttons() & button,
            button,
            "Button {:04X} should be released",
            button
        );
    }
}
