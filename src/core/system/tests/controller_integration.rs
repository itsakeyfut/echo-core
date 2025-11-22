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

//! Controller port integration tests

use super::super::*;

#[test]
fn test_controller_ports_initialization() {
    let system = System::new();

    // Controller port 1 should have a controller
    assert!(system
        .controller_ports()
        .borrow_mut()
        .get_controller_mut(0)
        .is_some());
}

#[test]
fn test_controller_ports_select() {
    let mut ports = ControllerPorts::new();

    // Select port 1
    ports.write_ctrl(0x0002); // SELECT bit

    // Transfer data
    ports.write_tx_data(0x01);
    assert_eq!(ports.read_rx_data(), 0xFF);

    ports.write_tx_data(0x42);
    assert_eq!(ports.read_rx_data(), 0x41); // Digital pad ID
}

#[test]
fn test_controller_ports_button_state() {
    let system = System::new();

    // Press a button on port 1
    let controller_ports = system.controller_ports();
    let mut ports_borrow = controller_ports.borrow_mut();
    if let Some(controller) = ports_borrow.get_controller_mut(0) {
        use crate::core::controller::buttons;
        controller.press_button(buttons::CROSS);
        assert_eq!(controller.get_buttons() & buttons::CROSS, 0);
    }
}
