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

//! PlayStation Controller Port Registers
//!
//! This module manages the memory-mapped I/O registers for controller communication.

use super::super::controller::Controller;

/// PlayStation Controller Port Registers
///
/// Manages the memory-mapped I/O registers for controller communication.
///
/// # Register Map
/// - 0x1F801040: JOY_TX_DATA / JOY_RX_DATA (read/write)
/// - 0x1F801044: JOY_STAT (Status register)
/// - 0x1F801048: JOY_MODE (Mode register)
/// - 0x1F80104A: JOY_CTRL (Control register)
/// - 0x1F80104E: JOY_BAUD (Baud rate)
///
/// # Protocol
/// The controller uses a synchronous serial protocol:
/// 1. Write to JOY_CTRL to select controller
/// 2. Write bytes to JOY_TX_DATA
/// 3. Read responses from JOY_RX_DATA
/// 4. Write to JOY_CTRL to deselect controller
pub struct ControllerPorts {
    /// JOY_TX_DATA (0x1F801040) - Transmit data
    tx_data: u8,

    /// JOY_RX_DATA (0x1F801040) - Receive data (same register)
    rx_data: u8,

    /// JOY_STAT (0x1F801044) - Status register
    stat: u32,

    /// JOY_MODE (0x1F801048) - Mode register
    mode: u16,

    /// JOY_CTRL (0x1F80104A) - Control register
    ctrl: u16,

    /// JOY_BAUD (0x1F80104E) - Baud rate
    baud: u16,

    /// Connected controllers (port 1 and 2)
    controllers: [Option<Controller>; 2],

    /// Currently selected port (0 or 1)
    selected_port: Option<usize>,
}

impl ControllerPorts {
    /// Create new controller ports with default state
    ///
    /// Initializes with one controller connected to port 1.
    pub fn new() -> Self {
        Self {
            tx_data: 0xFF,
            rx_data: 0xFF,
            stat: 0x05, // TX ready (bit 0), RX ready (bit 2)
            mode: 0x000D,
            ctrl: 0,
            baud: 0,
            controllers: [Some(Controller::new()), None], // Port 1 has controller
            selected_port: None,
        }
    }

    /// Write to TX_DATA register (0x1F801040)
    ///
    /// Transmits a byte to the selected controller and receives a response byte.
    ///
    /// # Arguments
    ///
    /// * `value` - Byte to transmit
    pub fn write_tx_data(&mut self, value: u8) {
        self.tx_data = value;

        // If controller is selected, perform transfer
        if let Some(port) = self.selected_port {
            if let Some(controller) = &mut self.controllers[port] {
                self.rx_data = controller.transfer(value);
            } else {
                self.rx_data = 0xFF; // No controller
            }
        } else {
            self.rx_data = 0xFF;
        }

        // Set RX ready flag (bit 1)
        self.stat |= 0x02;
    }

    /// Read from RX_DATA register (0x1F801040)
    ///
    /// Returns the last received byte from the controller.
    ///
    /// # Returns
    ///
    /// Received byte
    pub fn read_rx_data(&mut self) -> u8 {
        // Clear RX ready flag
        self.stat &= !0x02;
        self.rx_data
    }

    /// Write to CTRL register (0x1F80104A)
    ///
    /// Controls controller selection and interrupt acknowledgment.
    ///
    /// # Arguments
    ///
    /// * `value` - Control register value
    pub fn write_ctrl(&mut self, value: u16) {
        self.ctrl = value;

        // Check for controller select (bit 1)
        if (value & 0x0002) != 0 {
            // Determine which port based on DTR bits
            let port = if (value & 0x2000) != 0 { 1 } else { 0 };
            self.selected_port = Some(port);

            if let Some(controller) = &mut self.controllers[port] {
                controller.select();
            }

            log::trace!("Controller port {} selected", port + 1);
        } else {
            // Deselect
            if let Some(port) = self.selected_port {
                if let Some(controller) = &mut self.controllers[port] {
                    controller.deselect();
                }
                log::trace!("Controller port {} deselected", port + 1);
            }
            self.selected_port = None;
        }

        // Acknowledge interrupt (bit 4)
        if (value & 0x0010) != 0 {
            self.stat &= !0x0200; // Clear IRQ flag
        }
    }

    /// Read STAT register (0x1F801044)
    ///
    /// Returns the controller port status.
    ///
    /// # Returns
    ///
    /// Status register value
    #[inline]
    pub fn read_stat(&self) -> u32 {
        self.stat
    }

    /// Read MODE register (0x1F801048)
    ///
    /// # Returns
    ///
    /// Mode register value
    #[inline]
    pub fn read_mode(&self) -> u16 {
        self.mode
    }

    /// Write MODE register (0x1F801048)
    ///
    /// # Arguments
    ///
    /// * `value` - Mode register value
    #[inline]
    pub fn write_mode(&mut self, value: u16) {
        self.mode = value;
    }

    /// Read CTRL register (0x1F80104A)
    ///
    /// # Returns
    ///
    /// Control register value
    #[inline]
    pub fn read_ctrl(&self) -> u16 {
        self.ctrl
    }

    /// Read BAUD register (0x1F80104E)
    ///
    /// # Returns
    ///
    /// Baud rate register value
    #[inline]
    pub fn read_baud(&self) -> u16 {
        self.baud
    }

    /// Write BAUD register (0x1F80104E)
    ///
    /// # Arguments
    ///
    /// * `value` - Baud rate value
    #[inline]
    pub fn write_baud(&mut self, value: u16) {
        self.baud = value;
    }

    /// Get mutable reference to controller at port (0 or 1)
    ///
    /// # Arguments
    ///
    /// * `port` - Port number (0 = port 1, 1 = port 2)
    ///
    /// # Returns
    ///
    /// Optional mutable reference to controller
    pub fn get_controller_mut(&mut self, port: usize) -> Option<&mut Controller> {
        self.controllers.get_mut(port).and_then(|c| c.as_mut())
    }
}

impl Default for ControllerPorts {
    fn default() -> Self {
        Self::new()
    }
}
