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

use super::super::*;

#[test]
fn test_serial_select_deselect() {
    let mut controller = Controller::new();

    assert_eq!(controller.state, SerialState::Idle);

    // Select controller
    controller.select();
    assert_eq!(controller.state, SerialState::Selected);
    assert_eq!(controller.tx_buffer.len(), 5);
    assert_eq!(controller.tx_buffer[1], 0x41); // Digital pad ID

    // Deselect controller
    controller.deselect();
    assert_eq!(controller.state, SerialState::Idle);
    assert_eq!(controller.tx_buffer.len(), 0);
}

#[test]
fn test_serial_protocol() {
    let mut controller = Controller::new();

    // Press some buttons
    controller.press_button(buttons::CROSS);
    controller.press_button(buttons::START);

    // Select controller
    controller.select();

    // Transfer sequence
    assert_eq!(controller.transfer(0x01), 0xFF); // Initial
    assert_eq!(controller.transfer(0x42), 0x41); // Controller ID
    assert_eq!(controller.transfer(0x00), 0x5A); // 0x5A

    // Button states (CROSS and START pressed)
    let byte3 = controller.transfer(0x00);
    let byte4 = controller.transfer(0x00);

    let buttons_state = (byte4 as u16) << 8 | byte3 as u16;
    assert_eq!(buttons_state & buttons::CROSS, 0);
    assert_eq!(buttons_state & buttons::START, 0);
    assert_ne!(buttons_state & buttons::CIRCLE, 0);
    assert_ne!(buttons_state & buttons::SQUARE, 0);
}

#[test]
fn test_is_acknowledged() {
    let mut controller = Controller::new();

    controller.select();
    controller.transfer(0x01);

    assert!(controller.is_acknowledged());
}

#[test]
fn test_transfer_when_idle() {
    let mut controller = Controller::new();

    // Transfer without selecting should return 0xFF
    let response = controller.transfer(0x42);
    assert_eq!(response, 0xFF);
    assert_eq!(controller.state, SerialState::Idle);
}

#[test]
fn test_button_state_in_serial_response() {
    let mut controller = Controller::new();

    // Set specific button pattern
    controller.press_button(buttons::UP);
    controller.press_button(buttons::CROSS);
    controller.press_button(buttons::L1);

    controller.select();

    // Skip to button data
    controller.transfer(0x01);
    controller.transfer(0x42);
    controller.transfer(0x00);

    let low_byte = controller.transfer(0x00);
    let high_byte = controller.transfer(0x00);

    let expected_state = controller.get_buttons();
    let received_state = (high_byte as u16) << 8 | low_byte as u16;

    assert_eq!(received_state, expected_state);
}
