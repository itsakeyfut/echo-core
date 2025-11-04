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
//! - INT1: Data ready (sector read complete)
//! - INT2: Command complete (second response)
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
//! assert!(!cdrom.response_empty());
//! assert_ne!(cdrom.interrupt_flag(), 0);
//! ```

use std::collections::VecDeque;

mod commands;
mod disc;
#[cfg(test)]
mod tests;

pub use disc::{DiscImage, Track, TrackType};

/// CD-ROM drive controller
///
/// Emulates the Sony CXD2510Q CD-ROM controller with command processing,
/// FIFO buffers, and interrupt generation.
pub struct CDROM {
    /// Parameter FIFO (up to 16 bytes)
    ///
    /// Parameters for commands are pushed here before the command is executed.
    pub(super) param_fifo: VecDeque<u8>,

    /// Response FIFO (up to 16 bytes)
    ///
    /// Command responses are placed here for the CPU to read.
    pub(super) response_fifo: VecDeque<u8>,

    /// Data buffer (2352 bytes per sector)
    ///
    /// Sector data read from disc is stored here for DMA transfer.
    pub(super) data_buffer: Vec<u8>,

    /// Current index in data buffer for byte-by-byte reading
    pub(super) data_index: usize,

    /// Cycle counter for sector reading timing
    pub(super) read_ticks: u32,

    /// Cycle counter for seek timing
    pub(super) seek_ticks: u32,

    /// Current drive state
    pub(super) state: CDState,

    /// Current read position (minute, second, sector)
    pub(super) position: CDPosition,

    /// Target seek position
    pub(super) seek_target: Option<CDPosition>,

    /// Interrupt flag (5 levels: bit 0-4 for INT1-INT5)
    pub(super) interrupt_flag: u8,

    /// Interrupt enable mask
    interrupt_enable: u8,

    /// Status register
    pub(super) status: CDStatus,

    /// Loaded disc image (if any)
    pub(super) disc: Option<DiscImage>,

    /// Current index/status register select
    index: u8,
}

/// CD-ROM drive state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CDState {
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
pub(super) struct CDStatus {
    /// Error occurred
    pub(super) error: bool,
    /// Motor on
    pub(super) motor_on: bool,
    /// Seek error
    pub(super) seek_error: bool,
    /// ID error (disc not recognized)
    pub(super) id_error: bool,
    /// Shell open (disc tray open)
    pub(super) shell_open: bool,
    /// Currently reading data
    pub(super) reading: bool,
    /// Currently seeking
    pub(super) seeking: bool,
    /// Currently playing audio
    #[allow(dead_code)]
    pub(super) playing: bool,
}

impl CDROM {
    /// CD-ROM register addresses
    pub const REG_INDEX: u32 = 0x1F801800;
    pub const REG_DATA: u32 = 0x1F801801;
    pub const REG_INT_FLAG: u32 = 0x1F801802;
    pub const REG_INT_ENABLE: u32 = 0x1F801803;

    /// Maximum FIFO size (16 bytes)
    const FIFO_SIZE: usize = 16;

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
            data_buffer: Vec::new(),
            data_index: 0,
            read_ticks: 0,
            seek_ticks: 0,
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
    /// When INT5 is acknowledged, also clears latched error status flags.
    pub fn acknowledge_interrupt(&mut self, value: u8) {
        self.interrupt_flag &= !value;
        if value & 0x10 != 0 {
            self.status.error = false;
            self.status.seek_error = false;
            self.status.id_error = false;
        }
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
    ///
    /// Bits 0-1: Register select (0-3)
    /// Bit 2: Clear parameter FIFO
    /// Bit 3: Clear response FIFO
    pub fn set_index(&mut self, value: u8) {
        if value & 0x04 != 0 {
            self.param_fifo.clear();
        }
        if value & 0x08 != 0 {
            self.response_fifo.clear();
        }
        self.index = value & 0x3;
    }

    /// Get index register
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Read status register (0x1F801800)
    ///
    /// Returns hardware status including FIFO states and busy flags.
    ///
    /// # Status Register Format
    ///
    /// ```text
    /// Bit 0-1: Index (0-3)
    /// Bit 2: ADPBUSY (XA-ADPCM playback active)
    /// Bit 3: Parameter FIFO empty (0=Not Empty, 1=Empty)
    /// Bit 4: Parameter FIFO not full (0=Full, 1=Not Full)
    /// Bit 5: Response FIFO not empty (0=Empty, 1=Not Empty)
    /// Bit 6: Data FIFO not empty (0=Empty, 1=Not Empty)
    /// Bit 7: Busy (0=Ready, 1=Busy)
    /// ```
    pub fn read_status(&self) -> u8 {
        let mut status = self.index & 0x3; // Bits 0-1: current index

        // Bit 2: ADPBUSY (always 0 for minimal stub)
        // status |= 0 << 2;

        // Bit 3: Parameter FIFO empty
        if self.param_fifo.is_empty() {
            status |= 1 << 3;
        }

        // Bit 4: Parameter FIFO not full
        if self.param_fifo.len() < Self::FIFO_SIZE {
            status |= 1 << 4;
        }

        // Bit 5: Response FIFO not empty
        if !self.response_fifo.is_empty() {
            status |= 1 << 5;
        }

        // Bit 6: Data FIFO not empty
        if self.data_index < self.data_buffer.len() {
            status |= 1 << 6;
        }

        // Bit 7: Busy (0=Ready, 1=Busy)
        // Drive is busy when seeking or reading
        if self.state == CDState::Seeking || self.state == CDState::Reading {
            status |= 1 << 7;
        }

        status
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
    pub(super) fn get_status_byte(&self) -> u8 {
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
    /// - INT1: Data ready (sector read complete)
    /// - INT2: Command complete (second response)
    /// - INT3: Command acknowledge (first response)
    /// - INT4: Command error
    /// - INT5: Read error
    pub(super) fn trigger_interrupt(&mut self, level: u8) {
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
    pub(super) fn error_response(&mut self) {
        self.status.error = true;
        self.response_fifo.push_back(self.get_status_byte() | 0x01);
        self.response_fifo.push_back(0x80); // Error code: Invalid command
        self.trigger_interrupt(5); // INT5 (error)
    }

    /// Load a disc image from a .cue file
    ///
    /// Loads the disc image and updates the drive state to reflect
    /// that a disc is present.
    ///
    /// # Arguments
    ///
    /// * `cue_path` - Path to the .cue file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if disc loaded successfully
    /// - `Err(Box<dyn std::error::Error>)` if loading failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let mut cdrom = CDROM::new();
    /// cdrom.load_disc("game.cue").unwrap();
    /// ```
    pub fn load_disc(&mut self, cue_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let disc = DiscImage::load(cue_path)?;
        self.disc = Some(disc);
        self.status.shell_open = false;
        log::info!("Disc loaded successfully");
        Ok(())
    }

    /// Read the current sector from the loaded disc
    ///
    /// Reads sector data at the current position from the disc image.
    ///
    /// # Returns
    ///
    /// - `Some(Vec<u8>)` - Sector data (2352 bytes)
    /// - `None` - No disc loaded or position out of bounds
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let mut cdrom = CDROM::new();
    /// // cdrom.load_disc("game.cue").unwrap();
    /// if let Some(data) = cdrom.read_current_sector() {
    ///     println!("Read {} bytes", data.len());
    /// }
    /// ```
    pub fn read_current_sector(&mut self) -> Option<Vec<u8>> {
        if let Some(ref disc) = self.disc {
            disc.read_sector(&self.position).map(|data| data.to_vec())
        } else {
            None
        }
    }

    /// Check if a disc is loaded
    ///
    /// # Returns
    ///
    /// true if a disc image is loaded, false otherwise
    pub fn has_disc(&self) -> bool {
        self.disc.is_some()
    }

    /// Get the current read position
    ///
    /// # Returns
    ///
    /// Current MSF position
    pub fn position(&self) -> &CDPosition {
        &self.position
    }

    /// Set the current read position
    ///
    /// # Arguments
    ///
    /// * `position` - New MSF position
    pub fn set_position(&mut self, position: CDPosition) {
        self.position = position;
    }

    /// Advance execution by the specified number of CPU cycles
    ///
    /// This method simulates the timing of CD-ROM operations including
    /// sector reading and seeking. It should be called periodically from
    /// the main emulation loop.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles to advance
    ///
    /// # Sector Reading Timing
    ///
    /// At 1x speed (75 sectors/second), each sector takes approximately
    /// 13,300 CPU cycles to read (assuming 33.8688 MHz CPU and ~13.33ms per sector).
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let mut cdrom = CDROM::new();
    /// // Start reading...
    /// cdrom.execute_command(0x06);
    ///
    /// // Simulate time passing
    /// for _ in 0..1000 {
    ///     cdrom.tick(100);
    /// }
    /// ```
    pub fn tick(&mut self, cycles: u32) {
        // Handle sector reading
        if self.state == CDState::Reading {
            self.read_ticks += cycles;

            // Read one sector every ~13,300 cycles (at 1x speed)
            // 75 sectors/second at ~33.8688 MHz CPU = ~451,584 cycles/second / 75 = ~6,021 cycles
            // However, PSX-SPX documents that actual timing is closer to 13,300 cycles
            const CYCLES_PER_SECTOR: u32 = 13_300;

            if self.read_ticks >= CYCLES_PER_SECTOR {
                self.read_ticks -= CYCLES_PER_SECTOR;

                if let Some(data) = self.read_current_sector() {
                    self.data_buffer = data;
                    self.data_index = 0;
                    self.trigger_interrupt(1); // INT1 (data ready)

                    log::trace!(
                        "CD-ROM: Read sector at {:02}:{:02}:{:02}",
                        self.position.minute,
                        self.position.second,
                        self.position.sector
                    );

                    // Advance to next sector
                    self.advance_position();
                }
            }
        }

        // Handle seeking
        if self.state == CDState::Seeking {
            self.seek_ticks += cycles;

            let seek_time = self.calculate_seek_time();
            if self.seek_ticks >= seek_time {
                self.seek_ticks = 0;
                self.state = CDState::Idle;
                self.status.seeking = false;

                if let Some(target) = self.seek_target {
                    self.position = target;

                    log::debug!(
                        "CD-ROM: Seek complete to {:02}:{:02}:{:02}",
                        self.position.minute,
                        self.position.second,
                        self.position.sector
                    );

                    self.response_fifo.push_back(self.get_status_byte());
                    self.trigger_interrupt(2); // INT2 (seek complete)
                }
            }
        }
    }

    /// Advance MSF position by one sector
    ///
    /// Handles wraparound for sectors (75 per second) and seconds (60 per minute).
    fn advance_position(&mut self) {
        self.position.sector += 1;
        if self.position.sector >= 75 {
            self.position.sector = 0;
            self.position.second += 1;
            if self.position.second >= 60 {
                self.position.second = 0;
                self.position.minute += 1;
            }
        }
    }

    /// Calculate seek time in CPU cycles based on seek distance
    ///
    /// # Returns
    ///
    /// Number of CPU cycles for the seek operation
    ///
    /// # Implementation Note
    ///
    /// This is a simplified implementation using a fixed seek time.
    /// Real hardware varies seek time based on distance:
    /// - Short seeks (same track): ~1ms
    /// - Medium seeks (nearby): ~20-50ms
    /// - Long seeks (opposite sides): ~200-500ms
    ///
    /// For now, we use a fixed time of ~3ms (100,000 cycles).
    fn calculate_seek_time(&self) -> u32 {
        // TODO: Calculate actual seek time based on distance
        // For now, use fixed seek time of approximately 3ms
        100_000 // ~3ms at 33.8688 MHz
    }

    /// Read a single byte from the data buffer
    ///
    /// This method is used for DMA transfers and provides byte-by-byte
    /// access to the sector data buffer. Returns 0 if the buffer is exhausted.
    ///
    /// # Returns
    ///
    /// The next byte from the data buffer, or 0 if exhausted
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::CDROM;
    ///
    /// let mut cdrom = CDROM::new();
    /// // ... after reading a sector ...
    /// let byte = cdrom.get_data_byte();
    /// ```
    pub fn get_data_byte(&mut self) -> u8 {
        if self.data_index < self.data_buffer.len() {
            let byte = self.data_buffer[self.data_index];
            self.data_index += 1;
            byte
        } else {
            0
        }
    }

    /// Push a byte to the data buffer (for testing)
    ///
    /// This is a test helper method to populate the CD-ROM data buffer
    /// for DMA transfer testing.
    ///
    /// # Arguments
    ///
    /// * `byte` - The byte to add to the buffer
    #[cfg(test)]
    pub fn push_data_byte(&mut self, byte: u8) {
        self.data_buffer.push(byte);
    }

    /// Read from a CD-ROM register
    ///
    /// The CD-ROM controller has 4 registers (0x1F801800-0x1F801803),
    /// with behavior depending on the current index value (0-3).
    ///
    /// # Arguments
    ///
    /// * `addr` - Register address (0x1F801800-0x1F801803)
    ///
    /// # Returns
    ///
    /// Register value
    ///
    /// # Register Map
    ///
    /// ```text
    /// 0x1F801800: Status Register (all indices)
    /// 0x1F801801: Response FIFO (index 0, 1) / Data Byte (index 2, 3)
    /// 0x1F801802: Data FIFO (index 0, 1) / Interrupt Enable (index 2, 3)
    /// 0x1F801803: Interrupt Enable (index 0) / Interrupt Flag (index 1-3)
    /// ```
    pub fn read_register(&mut self, addr: u32) -> u8 {
        match (addr, self.index) {
            // 0x1F801800: Status register (all indices)
            (Self::REG_INDEX, _) => self.read_status(),

            // 0x1F801801: Response FIFO (index 0, 1)
            (Self::REG_DATA, 0) | (Self::REG_DATA, 1) => {
                self.response_fifo.pop_front().unwrap_or(0)
            }

            // 0x1F801801: Data byte (index 2, 3)
            (Self::REG_DATA, 2) | (Self::REG_DATA, 3) => self.get_data_byte(),

            // 0x1F801802: Data FIFO (index 0, 1) - unused
            (Self::REG_INT_FLAG, 0) | (Self::REG_INT_FLAG, 1) => 0,

            // 0x1F801802: Interrupt Enable (index 2, 3)
            (Self::REG_INT_FLAG, 2) | (Self::REG_INT_FLAG, 3) => self.interrupt_enable,

            // 0x1F801803: Interrupt Enable (index 0)
            (Self::REG_INT_ENABLE, 0) => self.interrupt_enable,

            // 0x1F801803: Interrupt Flag (index 1-3)
            (Self::REG_INT_ENABLE, 1..=3) => 0xE0 | self.interrupt_flag,

            _ => {
                log::warn!("CD-ROM: Invalid register read at 0x{:08X}", addr);
                0
            }
        }
    }

    /// Write to a CD-ROM register
    ///
    /// The CD-ROM controller has 4 registers (0x1F801800-0x1F801803),
    /// with behavior depending on the current index value (0-3).
    ///
    /// # Arguments
    ///
    /// * `addr` - Register address (0x1F801800-0x1F801803)
    /// * `value` - Value to write
    ///
    /// # Register Map
    ///
    /// ```text
    /// 0x1F801800: Index/Status register (all indices)
    /// 0x1F801801: Command register (index 0) / Sound Map Data (index 1-3)
    /// 0x1F801802: Parameter FIFO (index 0) / Interrupt Enable (index 1) / Audio Volume (index 2-3)
    /// 0x1F801803: Request Register (index 0) / Interrupt Flag (index 1) / Audio Volume (index 2-3)
    /// ```
    pub fn write_register(&mut self, addr: u32, value: u8) {
        match (addr, self.index) {
            // 0x1F801800: Index/Status register (all indices)
            (Self::REG_INDEX, _) => self.set_index(value),

            // 0x1F801801: Command register (index 0)
            (Self::REG_DATA, 0) => self.execute_command(value),

            // 0x1F801801: Sound Map Data Out (index 1-3) - not implemented
            (Self::REG_DATA, 1..=3) => {
                log::trace!("CD-ROM: Sound Map Data Out write: 0x{:02X}", value);
            }

            // 0x1F801802: Parameter FIFO (index 0)
            (Self::REG_INT_FLAG, 0) => self.push_param(value),

            // 0x1F801802: Interrupt Enable (index 1)
            (Self::REG_INT_FLAG, 1) => self.set_interrupt_enable(value),

            // 0x1F801802: Audio Volume (index 2-3) - not implemented
            (Self::REG_INT_FLAG, 2) | (Self::REG_INT_FLAG, 3) => {
                log::trace!("CD-ROM: Audio Volume write: 0x{:02X}", value);
            }

            // 0x1F801803: Request Register (index 0) - not implemented
            (Self::REG_INT_ENABLE, 0) => {
                log::trace!("CD-ROM: Request Register write: 0x{:02X}", value);
            }

            // 0x1F801803: Interrupt Flag (index 1)
            (Self::REG_INT_ENABLE, 1) => self.acknowledge_interrupt(value),

            // 0x1F801803: Audio Volume (index 2-3) - not implemented
            (Self::REG_INT_ENABLE, 2) | (Self::REG_INT_ENABLE, 3) => {
                log::trace!("CD-ROM: Audio Volume write: 0x{:02X}", value);
            }

            _ => {
                log::warn!(
                    "CD-ROM: Invalid register write at 0x{:08X} = 0x{:02X}",
                    addr,
                    value
                );
            }
        }
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
