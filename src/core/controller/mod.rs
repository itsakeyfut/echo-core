//! PlayStation Controller/Gamepad System
//!
//! This module implements the PlayStation digital controller (standard gamepad)
//! with serial communication protocol and button state management.
//!
//! # Controller Communication Protocol
//!
//! The controller uses a synchronous serial protocol:
//! 1. Select controller (chip select)
//! 2. Transfer bytes bidirectionally
//! 3. Deselect controller
//!
//! Digital pad response format:
//! - Byte 0: 0xFF (initial)
//! - Byte 1: 0x41 (Controller ID - Digital Pad)
//! - Byte 2: 0x5A (Always 0x5A)
//! - Byte 3: Button state low byte
//! - Byte 4: Button state high byte
//!
//! # Button Encoding
//!
//! Buttons use active-low encoding (0 = pressed, 1 = released).
//! This matches the PlayStation hardware behavior.

/// Button bit definitions for PlayStation controller
///
/// All buttons use active-low logic:
/// - 0 = button is pressed
/// - 1 = button is released
pub mod buttons {
    /// SELECT button (bit 0)
    pub const SELECT: u16 = 1 << 0;
    /// L3 button (left stick press) (bit 1)
    pub const L3: u16 = 1 << 1;
    /// R3 button (right stick press) (bit 2)
    pub const R3: u16 = 1 << 2;
    /// START button (bit 3)
    pub const START: u16 = 1 << 3;
    /// D-Pad UP (bit 4)
    pub const UP: u16 = 1 << 4;
    /// D-Pad RIGHT (bit 5)
    pub const RIGHT: u16 = 1 << 5;
    /// D-Pad DOWN (bit 6)
    pub const DOWN: u16 = 1 << 6;
    /// D-Pad LEFT (bit 7)
    pub const LEFT: u16 = 1 << 7;
    /// L2 shoulder button (bit 8)
    pub const L2: u16 = 1 << 8;
    /// R2 shoulder button (bit 9)
    pub const R2: u16 = 1 << 9;
    /// L1 shoulder button (bit 10)
    pub const L1: u16 = 1 << 10;
    /// R1 shoulder button (bit 11)
    pub const R1: u16 = 1 << 11;
    /// Triangle button (bit 12)
    pub const TRIANGLE: u16 = 1 << 12;
    /// Circle button (bit 13)
    pub const CIRCLE: u16 = 1 << 13;
    /// Cross (X) button (bit 14)
    pub const CROSS: u16 = 1 << 14;
    /// Square button (bit 15)
    pub const SQUARE: u16 = 1 << 15;
}

/// Serial communication state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SerialState {
    /// Controller not selected
    Idle,
    /// Controller selected, ready for transfer
    Selected,
    /// Data transfer in progress
    Transferring,
}

/// PlayStation digital controller (gamepad)
///
/// Implements the standard PSX digital controller with 14 buttons
/// and serial communication protocol.
///
/// # Examples
///
/// ```
/// use psrx::core::controller::{Controller, buttons};
///
/// let mut controller = Controller::new();
///
/// // Press a button
/// controller.press_button(buttons::CROSS);
///
/// // Perform serial transfer
/// controller.select();
/// let response = controller.transfer(0x01);
/// controller.deselect();
/// ```
#[derive(Debug, Clone)]
pub struct Controller {
    /// Button state bitfield (active low: 0 = pressed, 1 = released)
    buttons: u16,

    /// Serial communication state
    state: SerialState,

    /// Transmit buffer (controller -> console)
    tx_buffer: Vec<u8>,

    /// Receive buffer (console -> controller)
    rx_buffer: Vec<u8>,

    /// Current byte index being transferred
    transfer_index: usize,
}

impl Controller {
    /// Create a new controller with all buttons released
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::Controller;
    ///
    /// let controller = Controller::new();
    /// assert_eq!(controller.get_buttons(), 0xFFFF); // All released
    /// ```
    pub fn new() -> Self {
        Self {
            buttons: 0xFFFF, // All buttons released (active low)
            state: SerialState::Idle,
            tx_buffer: Vec::new(),
            rx_buffer: Vec::new(),
            transfer_index: 0,
        }
    }

    /// Press a button (set bit to 0 for active-low)
    ///
    /// # Arguments
    ///
    /// * `button` - Button bit mask from `buttons` module
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::{Controller, buttons};
    ///
    /// let mut controller = Controller::new();
    /// controller.press_button(buttons::CROSS);
    /// assert_eq!(controller.get_buttons() & buttons::CROSS, 0);
    /// ```
    #[inline]
    pub fn press_button(&mut self, button: u16) {
        self.buttons &= !button;
    }

    /// Release a button (set bit to 1 for active-low)
    ///
    /// # Arguments
    ///
    /// * `button` - Button bit mask from `buttons` module
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::{Controller, buttons};
    ///
    /// let mut controller = Controller::new();
    /// controller.press_button(buttons::CROSS);
    /// controller.release_button(buttons::CROSS);
    /// assert_eq!(controller.get_buttons(), 0xFFFF);
    /// ```
    #[inline]
    pub fn release_button(&mut self, button: u16) {
        self.buttons |= button;
    }

    /// Set button state directly
    ///
    /// # Arguments
    ///
    /// * `button` - Button bit mask from `buttons` module
    /// * `pressed` - true to press, false to release
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::{Controller, buttons};
    ///
    /// let mut controller = Controller::new();
    /// controller.set_button_state(buttons::START, true);
    /// assert_eq!(controller.get_buttons() & buttons::START, 0);
    /// ```
    #[inline]
    pub fn set_button_state(&mut self, button: u16, pressed: bool) {
        if pressed {
            self.press_button(button);
        } else {
            self.release_button(button);
        }
    }

    /// Get current button state
    ///
    /// # Returns
    ///
    /// 16-bit button state (active low: 0 = pressed, 1 = released)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::Controller;
    ///
    /// let controller = Controller::new();
    /// assert_eq!(controller.get_buttons(), 0xFFFF);
    /// ```
    #[inline]
    pub fn get_buttons(&self) -> u16 {
        self.buttons
    }

    /// Select the controller (chip select)
    ///
    /// Prepares the controller for serial communication by setting up
    /// the transmit buffer with the response sequence.
    ///
    /// Response format:
    /// - Byte 0: 0xFF (will be overwritten by first transfer)
    /// - Byte 1: 0x41 (Controller ID - Digital Pad)
    /// - Byte 2: 0x5A (Always 0x5A)
    /// - Byte 3: Button state low byte
    /// - Byte 4: Button state high byte
    pub fn select(&mut self) {
        self.state = SerialState::Selected;
        self.transfer_index = 0;

        // Prepare response buffer for digital controller
        self.tx_buffer = vec![
            0xFF,                               // Initial byte (will be overwritten)
            0x41,                               // Controller ID: 0x41 = Digital Pad
            0x5A,                               // Always 0x5A
            (self.buttons & 0xFF) as u8,        // Button state low byte
            ((self.buttons >> 8) & 0xFF) as u8, // Button state high byte
        ];

        log::trace!("Controller selected, buttons: 0x{:04X}", self.buttons);
    }

    /// Deselect the controller (chip deselect)
    ///
    /// Ends serial communication and clears internal buffers.
    pub fn deselect(&mut self) {
        self.state = SerialState::Idle;
        self.transfer_index = 0;
        self.tx_buffer.clear();
        self.rx_buffer.clear();

        log::trace!("Controller deselected");
    }

    /// Transfer a byte (simultaneous TX/RX)
    ///
    /// Transfers one byte in both directions simultaneously (full-duplex).
    /// The controller sends its response byte while receiving a byte from
    /// the console.
    ///
    /// # Arguments
    ///
    /// * `tx_byte` - Byte transmitted from console to controller
    ///
    /// # Returns
    ///
    /// Byte transmitted from controller to console
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::controller::Controller;
    ///
    /// let mut controller = Controller::new();
    /// controller.select();
    /// let response = controller.transfer(0x01);
    /// assert_eq!(response, 0xFF);
    /// ```
    pub fn transfer(&mut self, tx_byte: u8) -> u8 {
        if self.state == SerialState::Idle {
            return 0xFF;
        }

        self.state = SerialState::Transferring;

        // Store received byte
        self.rx_buffer.push(tx_byte);

        // Get response byte
        let rx_byte = if self.transfer_index < self.tx_buffer.len() {
            self.tx_buffer[self.transfer_index]
        } else {
            0xFF
        };

        self.transfer_index += 1;

        log::trace!(
            "Controller transfer: TX=0x{:02X} RX=0x{:02X} (index {})",
            tx_byte,
            rx_byte,
            self.transfer_index
        );

        rx_byte
    }

    /// Check if controller acknowledged the command
    ///
    /// The first byte sent from console should be 0x01 to indicate
    /// a command. This checks if that acknowledgment was received.
    ///
    /// # Returns
    ///
    /// true if controller received acknowledgment (0x01)
    pub fn is_acknowledged(&self) -> bool {
        self.rx_buffer.first() == Some(&0x01)
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
