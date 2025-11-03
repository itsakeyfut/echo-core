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

//! CD-ROM drive emulation for PlayStation 1
//!
//! This module emulates the Sony CXD2510Q CD-ROM controller, which handles:
//! - Disc reading and seeking
//! - Audio CD playback
//! - Command processing via parameter and response FIFOs
//! - Interrupt generation for command completion
//! - Data transfer via DMA
//!
//! # CD-ROM Commands
//!
//! The CD-ROM controller supports various commands sent via the command register:
//!
//! | Command | Name    | Description                              |
//! |---------|---------|------------------------------------------|
//! | 0x01    | GetStat | Get current drive status                 |
//! | 0x02    | SetLoc  | Set seek target position (MSF format)    |
//! | 0x06    | ReadN   | Start reading data sectors               |
//! | 0x09    | Pause   | Pause reading or audio playback          |
//! | 0x0A    | Init    | Initialize drive                         |
//! | 0x0E    | SetMode | Set drive mode (speed, sector size, etc) |
//! | 0x15    | SeekL   | Seek to target position (data)           |
//! | 0x1A    | GetID   | Get disc identification                  |
//! | 0x1B    | ReadS   | Start reading sectors with retry         |
//! | 0x1E    | ReadTOC | Read table of contents                   |
//!
//! # MSF Addressing
//!
//! The CD-ROM uses MSF (Minute:Second:Frame) addressing format:
//! - Minute: 0-99 (BCD)
//! - Second: 0-59 (BCD)
//! - Frame: 0-74 (BCD) - 75 frames per second
//!
//! MSF addresses are stored in BCD (Binary-Coded Decimal) format.
//!
//! # Interrupt Levels
//!
//! The CD-ROM controller generates 5 levels of interrupts:
//! - INT1: Reserved (unused)
//! - INT2: Command complete
//! - INT3: Command acknowledge (first response)
//! - INT4: Command error
//! - INT5: Read error
//!
//! # Example
//!
//! ```rust
//! use psrx::core::cdrom::CDROM;
//!
//! let mut cdrom = CDROM::new();
//!
//! // Send GetStat command
//! cdrom.execute_command(0x01);
//!
//! // Check response FIFO
//! assert!(!cdrom.response_fifo().is_empty());
//! ```

use std::collections::VecDeque;

/// CD-ROM drive controller
///
/// Emulates the Sony CXD2510Q CD-ROM controller with command processing,
/// FIFO buffers, and interrupt generation.
pub struct CDROM {
    /// Parameter FIFO (up to 16 bytes)
    ///
    /// Parameters for commands are pushed here before the command is executed.
    param_fifo: VecDeque<u8>,

    /// Response FIFO (up to 16 bytes)
    ///
    /// Command responses are placed here for the CPU to read.
    response_fifo: VecDeque<u8>,

    /// Data buffer (2352 bytes per sector)
    ///
    /// Sector data read from disc is stored here for DMA transfer.
    #[allow(dead_code)]
    data_buffer: Vec<u8>,

    /// Current drive state
    state: CDState,

    /// Current read position (minute, second, sector)
    position: CDPosition,

    /// Target seek position
    seek_target: Option<CDPosition>,

    /// Interrupt flag (5 levels: bit 0-4 for INT1-INT5)
    interrupt_flag: u8,

    /// Interrupt enable mask
    interrupt_enable: u8,

    /// Status register
    status: CDStatus,

    /// Loaded disc image (if any)
    disc: Option<DiscImage>,

    /// Current index/status register select
    index: u8,
}

/// CD-ROM drive state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CDState {
    /// Idle - no operation in progress
    Idle,
    /// Reading data sectors
    Reading,
    /// Seeking to target position
    Seeking,
    /// Playing audio CD
    #[allow(dead_code)]
    Playing,
}

/// CD-ROM position in MSF (Minute:Second:Frame) format
///
/// All values are stored as decimal (not BCD).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CDPosition {
    /// Minute (0-99)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Frame/Sector (0-74)
    pub sector: u8,
}

impl CDPosition {
    /// Create a new position
    pub fn new(minute: u8, second: u8, sector: u8) -> Self {
        Self {
            minute,
            second,
            sector,
        }
    }

    /// Convert MSF to logical block address (LBA)
    ///
    /// LBA = (minute * 60 + second) * 75 + sector - 150
    /// (The -150 offset accounts for the 2-second pregap)
    pub fn to_lba(&self) -> i32 {
        ((self.minute as i32 * 60 + self.second as i32) * 75 + self.sector as i32) - 150
    }

    /// Convert logical block address to MSF
    pub fn from_lba(lba: i32) -> Self {
        let total_sectors = lba + 150;
        let minute = (total_sectors / 75 / 60) as u8;
        let second = ((total_sectors / 75) % 60) as u8;
        let sector = (total_sectors % 75) as u8;
        Self::new(minute, second, sector)
    }
}

/// CD-ROM status register
#[derive(Debug, Clone, Default)]
struct CDStatus {
    /// Error occurred
    error: bool,
    /// Motor on
    motor_on: bool,
    /// Seek error
    seek_error: bool,
    /// ID error (disc not recognized)
    id_error: bool,
    /// Shell open (disc tray open)
    #[allow(dead_code)]
    shell_open: bool,
    /// Currently reading data
    reading: bool,
    /// Currently seeking
    seeking: bool,
    /// Currently playing audio
    #[allow(dead_code)]
    playing: bool,
}

/// Disc image (placeholder for future implementation)
///
/// This will be implemented in issue #60.
#[derive(Debug)]
pub struct DiscImage {
    // Placeholder - will be implemented when disc loading is added
}

impl CDROM {
    /// CD-ROM register addresses
    pub const REG_INDEX: u32 = 0x1F801800;
    pub const REG_DATA: u32 = 0x1F801801;
    pub const REG_INT_FLAG: u32 = 0x1F801802;
    pub const REG_INT_ENABLE: u32 = 0x1F801803;

    /// Maximum FIFO size (16 bytes)
    const FIFO_SIZE: usize = 16;

    /// Sector size (2352 bytes - full CD-ROM sector with headers)
    const SECTOR_SIZE: usize = 2352;

    /// Create a new CD-ROM controller
    ///
    /// Initializes the controller in idle state with no disc loaded.
    /// The initial position is set to 00:02:00 (start of data area).
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let cdrom = CDROM::new();
    /// ```
    pub fn new() -> Self {
        Self {
            param_fifo: VecDeque::new(),
            response_fifo: VecDeque::new(),
            data_buffer: vec![0; Self::SECTOR_SIZE],
            state: CDState::Idle,
            position: CDPosition::new(0, 2, 0),
            seek_target: None,
            interrupt_flag: 0,
            interrupt_enable: 0,
            status: CDStatus::default(),
            disc: None,
            index: 0,
        }
    }

    /// Push a parameter byte to the parameter FIFO
    ///
    /// Parameters are pushed before executing a command.
    /// The FIFO has a maximum size of 16 bytes.
    pub fn push_param(&mut self, value: u8) {
        if self.param_fifo.len() < Self::FIFO_SIZE {
            self.param_fifo.push_back(value);
            log::trace!("CD-ROM: Pushed parameter 0x{:02X}", value);
        } else {
            log::warn!("CD-ROM: Parameter FIFO overflow");
        }
    }

    /// Pop a response byte from the response FIFO
    ///
    /// Returns None if the FIFO is empty.
    pub fn pop_response(&mut self) -> Option<u8> {
        let value = self.response_fifo.pop_front();
        if let Some(v) = value {
            log::trace!("CD-ROM: Popped response 0x{:02X}", v);
        }
        value
    }

    /// Check if response FIFO is empty
    pub fn response_empty(&self) -> bool {
        self.response_fifo.is_empty()
    }

    /// Get the response FIFO for testing
    #[cfg(test)]
    pub fn response_fifo(&self) -> &VecDeque<u8> {
        &self.response_fifo
    }

    /// Get the parameter FIFO for testing
    #[cfg(test)]
    pub fn param_fifo_mut(&mut self) -> &mut VecDeque<u8> {
        &mut self.param_fifo
    }

    /// Get current interrupt flag
    pub fn interrupt_flag(&self) -> u8 {
        self.interrupt_flag
    }

    /// Acknowledge interrupt
    ///
    /// Clears the specified interrupt bits.
    pub fn acknowledge_interrupt(&mut self, value: u8) {
        self.interrupt_flag &= !value;
        log::trace!("CD-ROM: Acknowledged interrupts 0x{:02X}", value);
    }

    /// Set interrupt enable mask
    pub fn set_interrupt_enable(&mut self, value: u8) {
        self.interrupt_enable = value & 0x1F;
        log::trace!(
            "CD-ROM: Set interrupt enable 0x{:02X}",
            self.interrupt_enable
        );
    }

    /// Get interrupt enable mask
    pub fn interrupt_enable(&self) -> u8 {
        self.interrupt_enable
    }

    /// Set index register (for register selection)
    pub fn set_index(&mut self, value: u8) {
        self.index = value & 0x3;
    }

    /// Get index register
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Execute CD-ROM command
    ///
    /// Executes the specified command byte, consuming parameters from
    /// the parameter FIFO and generating responses in the response FIFO.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Command byte (0x00-0xFF)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let mut cdrom = CDROM::new();
    /// cdrom.execute_command(0x01); // GetStat
    /// assert!(!cdrom.response_empty());
    /// ```
    pub fn execute_command(&mut self, cmd: u8) {
        log::debug!("CD-ROM command: 0x{:02X}", cmd);

        match cmd {
            0x01 => self.cmd_getstat(),
            0x02 => self.cmd_setloc(),
            0x06 => self.cmd_readn(),
            0x09 => self.cmd_pause(),
            0x0A => self.cmd_init(),
            0x0E => self.cmd_setmode(),
            0x15 => self.cmd_seekl(),
            0x1A => self.cmd_getid(),
            0x1B => self.cmd_reads(),
            0x1E => self.cmd_readtoc(),
            _ => {
                log::warn!("Unknown CD-ROM command: 0x{:02X}", cmd);
                self.error_response();
            }
        }
    }

    /// Command 0x01: GetStat
    ///
    /// Returns the current drive status byte.
    fn cmd_getstat(&mut self) {
        log::trace!("CD-ROM: GetStat");
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x02: SetLoc
    ///
    /// Sets the seek target position from 3 parameter bytes (MM:SS:FF in BCD).
    fn cmd_setloc(&mut self) {
        if self.param_fifo.len() < 3 {
            log::warn!("CD-ROM: SetLoc with insufficient parameters");
            self.error_response();
            return;
        }

        let minute = self.param_fifo.pop_front().unwrap();
        let second = self.param_fifo.pop_front().unwrap();
        let sector = self.param_fifo.pop_front().unwrap();

        self.seek_target = Some(CDPosition::new(
            bcd_to_dec(minute),
            bcd_to_dec(second),
            bcd_to_dec(sector),
        ));

        log::debug!(
            "CD-ROM: SetLoc to {:02}:{:02}:{:02}",
            bcd_to_dec(minute),
            bcd_to_dec(second),
            bcd_to_dec(sector)
        );

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x06: ReadN
    ///
    /// Start reading data sectors at current position.
    fn cmd_readn(&mut self) {
        log::debug!("CD-ROM: ReadN");
        self.state = CDState::Reading;
        self.status.reading = true;

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Actual data will be delivered via DMA
        // For now, we just acknowledge the command
    }

    /// Command 0x09: Pause
    ///
    /// Pause reading or audio playback.
    fn cmd_pause(&mut self) {
        log::debug!("CD-ROM: Pause");

        self.state = CDState::Idle;
        self.status.reading = false;
        self.status.playing = false;

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Second response after pause completes
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(2); // INT2 (complete)
    }

    /// Command 0x0A: Init
    ///
    /// Initialize the drive (motor on, reset state).
    fn cmd_init(&mut self) {
        log::debug!("CD-ROM: Init");

        self.status.motor_on = true;
        self.state = CDState::Idle;
        self.status.reading = false;
        self.status.seeking = false;
        self.status.playing = false;

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Second response after init completes
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(2); // INT2 (complete)
    }

    /// Command 0x0E: SetMode
    ///
    /// Set drive mode (speed, sector size, etc).
    /// Parameters are consumed but not yet implemented.
    fn cmd_setmode(&mut self) {
        if self.param_fifo.is_empty() {
            log::warn!("CD-ROM: SetMode with no parameters");
            self.error_response();
            return;
        }

        let mode = self.param_fifo.pop_front().unwrap();
        log::debug!("CD-ROM: SetMode = 0x{:02X}", mode);

        // TODO: Store and use mode settings
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x15: SeekL
    ///
    /// Seek to target position (data mode).
    fn cmd_seekl(&mut self) {
        log::debug!("CD-ROM: SeekL");

        if let Some(target) = self.seek_target {
            self.state = CDState::Seeking;
            self.status.seeking = true;

            self.response_fifo.push_back(self.get_status_byte());
            self.trigger_interrupt(3); // INT3 (acknowledge)

            // Simulate seek (for now, immediately complete)
            // In a real implementation, this would take time based on distance
            self.position = target;
            self.state = CDState::Idle;
            self.status.seeking = false;

            log::debug!(
                "CD-ROM: Seeked to {:02}:{:02}:{:02}",
                self.position.minute,
                self.position.second,
                self.position.sector
            );

            self.response_fifo.push_back(self.get_status_byte());
            self.trigger_interrupt(2); // INT2 (complete)
        } else {
            log::warn!("CD-ROM: SeekL with no target set");
            self.error_response();
        }
    }

    /// Command 0x1A: GetID
    ///
    /// Get disc identification (region, disc type, etc).
    fn cmd_getid(&mut self) {
        log::debug!("CD-ROM: GetID");

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Second response with disc info
        if self.disc.is_some() {
            self.response_fifo.push_back(self.get_status_byte());
            self.response_fifo.push_back(0x00); // Licensed
            self.response_fifo.push_back(0x20); // Audio+CDROM
            self.response_fifo.push_back(0x00); // SCEx region string (unused)
            self.response_fifo.push_back(b'S'); // SCEx region
            self.response_fifo.push_back(b'C');
            self.response_fifo.push_back(b'E');
            self.response_fifo.push_back(b'A'); // SCEA (America)
            self.trigger_interrupt(2); // INT2 (complete)
        } else {
            // No disc
            self.status.id_error = true;
            self.error_response();
        }
    }

    /// Command 0x1B: ReadS
    ///
    /// Start reading sectors with retry on errors.
    fn cmd_reads(&mut self) {
        log::debug!("CD-ROM: ReadS");

        self.state = CDState::Reading;
        self.status.reading = true;

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x1E: ReadTOC
    ///
    /// Read table of contents from disc.
    fn cmd_readtoc(&mut self) {
        log::debug!("CD-ROM: ReadTOC");

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Second response after TOC read completes
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(2); // INT2 (complete)
    }

    /// Generate status byte from current drive state
    ///
    /// The status byte encodes various drive states:
    /// - Bit 0: Error
    /// - Bit 1: Motor on
    /// - Bit 2: Seek error
    /// - Bit 3: ID error
    /// - Bit 4: Shell open
    /// - Bit 5: Reading
    /// - Bit 6: Seeking
    /// - Bit 7: Playing audio
    fn get_status_byte(&self) -> u8 {
        let mut status = 0u8;

        if self.status.error {
            status |= 1 << 0;
        }
        if self.status.motor_on {
            status |= 1 << 1;
        }
        if self.status.seek_error {
            status |= 1 << 2;
        }
        if self.status.id_error {
            status |= 1 << 3;
        }
        if self.status.shell_open {
            status |= 1 << 4;
        }
        if self.status.reading {
            status |= 1 << 5;
        }
        if self.status.seeking {
            status |= 1 << 6;
        }
        if self.status.playing {
            status |= 1 << 7;
        }

        status
    }

    /// Trigger an interrupt
    ///
    /// Sets the interrupt flag for the specified level (1-5).
    ///
    /// # Interrupt Levels
    ///
    /// - INT1: Reserved (unused)
    /// - INT2: Command complete
    /// - INT3: Command acknowledge (first response)
    /// - INT4: Command error
    /// - INT5: Read error
    fn trigger_interrupt(&mut self, level: u8) {
        if level == 0 || level > 5 {
            log::warn!("CD-ROM: Invalid interrupt level {}", level);
            return;
        }

        self.interrupt_flag |= 1 << (level - 1);
        log::trace!("CD-ROM: Triggered INT{}", level);
    }

    /// Generate an error response
    ///
    /// Sets error status and generates INT5 (error interrupt).
    fn error_response(&mut self) {
        self.status.error = true;
        self.response_fifo.push_back(self.get_status_byte() | 0x01);
        self.response_fifo.push_back(0x80); // Error code: Invalid command
        self.trigger_interrupt(5); // INT5 (error)
    }
}

impl Default for CDROM {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert BCD (Binary-Coded Decimal) to decimal
///
/// BCD format: each nibble (4 bits) represents a decimal digit (0-9).
/// Example: 0x23 (BCD) = 23 (decimal)
///
/// # Arguments
///
/// * `bcd` - BCD-encoded byte
///
/// # Returns
///
/// Decimal value
#[inline]
pub fn bcd_to_dec(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

/// Convert decimal to BCD (Binary-Coded Decimal)
///
/// # Arguments
///
/// * `dec` - Decimal byte (0-99)
///
/// # Returns
///
/// BCD-encoded byte
#[inline]
pub fn dec_to_bcd(dec: u8) -> u8 {
    ((dec / 10) << 4) | (dec % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdrom_initialization() {
        let cdrom = CDROM::new();
        assert_eq!(cdrom.state, CDState::Idle);
        assert_eq!(cdrom.position.minute, 0);
        assert_eq!(cdrom.position.second, 2);
        assert_eq!(cdrom.position.sector, 0);
    }

    #[test]
    fn test_getstat() {
        let mut cdrom = CDROM::new();
        cdrom.execute_command(0x01);

        assert!(!cdrom.response_fifo.is_empty());
        assert_ne!(cdrom.interrupt_flag, 0);

        // Check INT3 was triggered
        assert_eq!(cdrom.interrupt_flag & 0x04, 0x04);
    }

    #[test]
    fn test_setloc() {
        let mut cdrom = CDROM::new();

        // Set parameters: 00:02:00 in BCD
        cdrom.param_fifo.push_back(0x00); // MM
        cdrom.param_fifo.push_back(0x02); // SS
        cdrom.param_fifo.push_back(0x00); // FF

        cdrom.execute_command(0x02);

        assert!(cdrom.seek_target.is_some());
        let target = cdrom.seek_target.unwrap();
        assert_eq!(target.minute, 0);
        assert_eq!(target.second, 2);
        assert_eq!(target.sector, 0);
    }

    #[test]
    fn test_setloc_insufficient_params() {
        let mut cdrom = CDROM::new();

        // Only push 2 parameters (need 3)
        cdrom.param_fifo.push_back(0x00);
        cdrom.param_fifo.push_back(0x02);

        cdrom.execute_command(0x02);

        // Should get error response
        assert_eq!(cdrom.interrupt_flag & 0x10, 0x10); // INT5
    }

    #[test]
    fn test_seekl() {
        let mut cdrom = CDROM::new();

        // Set target first
        cdrom.seek_target = Some(CDPosition::new(0, 10, 30));

        cdrom.execute_command(0x15); // SeekL

        // Position should be updated
        assert_eq!(cdrom.position.minute, 0);
        assert_eq!(cdrom.position.second, 10);
        assert_eq!(cdrom.position.sector, 30);

        // Should have responses and interrupts
        assert!(!cdrom.response_fifo.is_empty());
    }

    #[test]
    fn test_init() {
        let mut cdrom = CDROM::new();
        cdrom.execute_command(0x0A); // Init

        assert!(cdrom.status.motor_on);
        assert_eq!(cdrom.state, CDState::Idle);
        assert!(!cdrom.response_fifo.is_empty());
    }

    #[test]
    fn test_readn() {
        let mut cdrom = CDROM::new();
        cdrom.execute_command(0x06); // ReadN

        assert_eq!(cdrom.state, CDState::Reading);
        assert!(cdrom.status.reading);
        assert!(!cdrom.response_fifo.is_empty());
    }

    #[test]
    fn test_pause() {
        let mut cdrom = CDROM::new();

        // Start reading first
        cdrom.state = CDState::Reading;
        cdrom.status.reading = true;

        cdrom.execute_command(0x09); // Pause

        assert_eq!(cdrom.state, CDState::Idle);
        assert!(!cdrom.status.reading);
    }

    #[test]
    fn test_unknown_command() {
        let mut cdrom = CDROM::new();
        cdrom.execute_command(0xFF); // Invalid command

        // Should trigger error interrupt (INT5)
        assert_eq!(cdrom.interrupt_flag & 0x10, 0x10);
    }

    #[test]
    fn test_bcd_conversion() {
        assert_eq!(bcd_to_dec(0x23), 23);
        assert_eq!(bcd_to_dec(0x00), 0);
        assert_eq!(bcd_to_dec(0x99), 99);

        assert_eq!(dec_to_bcd(23), 0x23);
        assert_eq!(dec_to_bcd(0), 0x00);
        assert_eq!(dec_to_bcd(99), 0x99);
    }

    #[test]
    fn test_msf_to_lba() {
        let pos = CDPosition::new(0, 2, 0);
        assert_eq!(pos.to_lba(), 0); // Start of data (after 2-second pregap)

        let pos = CDPosition::new(0, 3, 0);
        assert_eq!(pos.to_lba(), 75); // 1 second after start
    }

    #[test]
    fn test_lba_to_msf() {
        let pos = CDPosition::from_lba(0);
        assert_eq!(pos.minute, 0);
        assert_eq!(pos.second, 2);
        assert_eq!(pos.sector, 0);

        let pos = CDPosition::from_lba(75);
        assert_eq!(pos.minute, 0);
        assert_eq!(pos.second, 3);
        assert_eq!(pos.sector, 0);
    }

    #[test]
    fn test_status_byte() {
        let mut cdrom = CDROM::new();

        // Initial status
        let status = cdrom.get_status_byte();
        assert_eq!(status, 0);

        // Set motor on
        cdrom.status.motor_on = true;
        let status = cdrom.get_status_byte();
        assert_eq!(status & 0x02, 0x02);

        // Set reading
        cdrom.status.reading = true;
        let status = cdrom.get_status_byte();
        assert_eq!(status & 0x20, 0x20);
    }

    #[test]
    fn test_interrupt_acknowledge() {
        let mut cdrom = CDROM::new();

        cdrom.trigger_interrupt(3);
        assert_eq!(cdrom.interrupt_flag, 0x04);

        cdrom.acknowledge_interrupt(0x04);
        assert_eq!(cdrom.interrupt_flag, 0x00);
    }

    #[test]
    fn test_param_response_fifos() {
        let mut cdrom = CDROM::new();

        // Test parameter FIFO
        cdrom.push_param(0x12);
        cdrom.push_param(0x34);
        assert_eq!(cdrom.param_fifo.len(), 2);

        // Test response FIFO
        cdrom.execute_command(0x01); // GetStat
        assert!(!cdrom.response_empty());

        let response = cdrom.pop_response();
        assert!(response.is_some());
    }
}
