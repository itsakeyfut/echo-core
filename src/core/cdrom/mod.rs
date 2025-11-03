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
//! assert!(!cdrom.response_empty());
//! assert_ne!(cdrom.interrupt_flag(), 0);
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
    shell_open: bool,
    /// Currently reading data
    reading: bool,
    /// Currently seeking
    seeking: bool,
    /// Currently playing audio
    #[allow(dead_code)]
    playing: bool,
}

/// Disc image loaded from .bin/.cue files
///
/// Represents a CD-ROM disc image with tracks and raw sector data.
/// Supports reading sectors in MSF format.
///
/// # Example
///
/// ```no_run
/// use psrx::core::cdrom::DiscImage;
///
/// let disc = DiscImage::load("game.cue").unwrap();
/// let position = psrx::core::cdrom::CDPosition::new(0, 2, 0);
/// let sector_data = disc.read_sector(&position);
/// ```
#[derive(Debug)]
pub struct DiscImage {
    /// Tracks on the disc
    tracks: Vec<Track>,

    /// Raw sector data from .bin file
    data: Vec<u8>,
}

/// CD-ROM track information
///
/// Represents a single track on a CD-ROM disc, including its type,
/// position, and location in the .bin file.
#[derive(Debug, Clone)]
pub struct Track {
    /// Track number (1-99)
    pub number: u8,

    /// Track type (Mode1/2352, Mode2/2352, Audio)
    pub track_type: TrackType,

    /// Start position (MSF)
    pub start_position: CDPosition,

    /// Length in sectors
    pub length_sectors: u32,

    /// Byte offset in .bin file
    pub file_offset: u64,
}

/// CD-ROM track type
///
/// Specifies the format of data stored in a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    /// Data track, 2352 bytes per sector (Mode 1)
    Mode1_2352,
    /// XA track, 2352 bytes per sector (Mode 2)
    Mode2_2352,
    /// CD-DA audio, 2352 bytes per sector
    Audio,
}

impl DiscImage {
    /// Load a disc image from a .cue file
    ///
    /// Parses the .cue file to extract track information and loads
    /// the corresponding .bin file containing raw sector data.
    ///
    /// # Arguments
    ///
    /// * `cue_path` - Path to the .cue file
    ///
    /// # Returns
    ///
    /// - `Ok(DiscImage)` if loading succeeded
    /// - `Err(Box<dyn std::error::Error>)` if loading failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cdrom::DiscImage;
    ///
    /// let disc = DiscImage::load("game.cue").unwrap();
    /// ```
    pub fn load(cue_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cue_data = std::fs::read_to_string(cue_path)?;
        let bin_path = Self::get_bin_path_from_cue(cue_path, &cue_data)?;

        let mut tracks = Self::parse_cue(&cue_data)?;
        let data = std::fs::read(bin_path)?;

        // Calculate track lengths based on file size and positions
        Self::calculate_track_lengths(&mut tracks, data.len());

        log::info!(
            "Loaded disc image: {} tracks, {} MB",
            tracks.len(),
            data.len() / 1024 / 1024
        );

        Ok(Self { tracks, data })
    }

    /// Extract .bin file path from .cue file path and content
    ///
    /// Searches for FILE directive in .cue content to determine .bin filename.
    ///
    /// # Arguments
    ///
    /// * `cue_path` - Path to the .cue file
    /// * `cue_data` - Content of the .cue file
    ///
    /// # Returns
    ///
    /// Full path to the .bin file
    fn get_bin_path_from_cue(
        cue_path: &str,
        cue_data: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Find FILE directive
        for line in cue_data.lines() {
            let line = line.trim();
            if line.starts_with("FILE") {
                // Extract filename from quotes
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let bin_filename = &line[start + 1..start + 1 + end];

                        // Construct full path by replacing .cue filename with .bin filename
                        let cue_path_obj = std::path::Path::new(cue_path);
                        let bin_path = if let Some(parent) = cue_path_obj.parent() {
                            parent.join(bin_filename)
                        } else {
                            std::path::PathBuf::from(bin_filename)
                        };

                        return Ok(bin_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Err("No FILE directive found in .cue file".into())
    }

    /// Parse .cue file content to extract track information
    ///
    /// # Arguments
    ///
    /// * `cue_data` - Content of the .cue file
    ///
    /// # Returns
    ///
    /// Vector of tracks parsed from the .cue file
    fn parse_cue(cue_data: &str) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
        let mut tracks = Vec::new();
        let mut current_track: Option<Track> = None;

        for line in cue_data.lines() {
            let line = line.trim();

            if line.starts_with("TRACK") {
                // Save previous track
                if let Some(track) = current_track.take() {
                    tracks.push(track);
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                let track_num = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                let track_type_str = parts.get(2).unwrap_or(&"MODE2/2352");

                current_track = Some(Track {
                    number: track_num,
                    track_type: Self::parse_track_type(track_type_str),
                    start_position: CDPosition::new(0, 0, 0),
                    length_sectors: 0,
                    file_offset: 0,
                });
            } else if line.starts_with("INDEX 01") {
                if let Some(ref mut track) = current_track {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(time_str) = parts.get(2) {
                        track.start_position = Self::parse_msf(time_str)?;
                        // Calculate file offset from MSF position
                        track.file_offset =
                            Self::msf_to_sector(&track.start_position) as u64 * 2352;
                    }
                }
            }
        }

        // Save last track
        if let Some(track) = current_track {
            tracks.push(track);
        }

        Ok(tracks)
    }

    /// Parse MSF time string (MM:SS:FF)
    ///
    /// # Arguments
    ///
    /// * `msf` - MSF string in format "MM:SS:FF"
    ///
    /// # Returns
    ///
    /// CDPosition parsed from the string
    fn parse_msf(msf: &str) -> Result<CDPosition, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = msf.split(':').collect();
        if parts.len() != 3 {
            return Err("Invalid MSF format".into());
        }

        Ok(CDPosition {
            minute: parts[0].parse()?,
            second: parts[1].parse()?,
            sector: parts[2].parse()?,
        })
    }

    /// Parse track type string from .cue file
    ///
    /// # Arguments
    ///
    /// * `s` - Track type string (e.g., "MODE1/2352", "AUDIO")
    ///
    /// # Returns
    ///
    /// Corresponding TrackType enum value
    fn parse_track_type(s: &str) -> TrackType {
        match s {
            "MODE1/2352" => TrackType::Mode1_2352,
            "MODE2/2352" => TrackType::Mode2_2352,
            "AUDIO" => TrackType::Audio,
            _ => TrackType::Mode2_2352, // Default to Mode2
        }
    }

    /// Calculate track lengths based on file size and start positions
    ///
    /// # Arguments
    ///
    /// * `tracks` - Mutable vector of tracks to update
    /// * `file_size` - Total size of the .bin file in bytes
    fn calculate_track_lengths(tracks: &mut [Track], file_size: usize) {
        for i in 0..tracks.len() {
            if i + 1 < tracks.len() {
                // Calculate length as difference between this track and next track
                let next_offset = tracks[i + 1].file_offset;
                let this_offset = tracks[i].file_offset;
                tracks[i].length_sectors = ((next_offset - this_offset) / 2352) as u32;
            } else {
                // Last track: calculate from remaining file size
                let this_offset = tracks[i].file_offset;
                tracks[i].length_sectors = ((file_size as u64 - this_offset) / 2352) as u32;
            }
        }
    }

    /// Read a sector from the disc at the specified MSF position
    ///
    /// # Arguments
    ///
    /// * `position` - MSF position to read from
    ///
    /// # Returns
    ///
    /// - `Some(&[u8])` - Sector data (2352 bytes)
    /// - `None` - Position out of bounds
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::cdrom::{DiscImage, CDPosition};
    /// # let disc = DiscImage::load("game.cue").unwrap();
    /// let position = CDPosition::new(0, 2, 0);
    /// if let Some(data) = disc.read_sector(&position) {
    ///     println!("Read {} bytes", data.len());
    /// }
    /// ```
    pub fn read_sector(&self, position: &CDPosition) -> Option<&[u8]> {
        let sector_num = Self::msf_to_sector(position);
        let offset = sector_num * 2352;

        if offset + 2352 <= self.data.len() {
            Some(&self.data[offset..offset + 2352])
        } else {
            None
        }
    }

    /// Convert MSF position to sector number
    ///
    /// # Arguments
    ///
    /// * `pos` - MSF position
    ///
    /// # Returns
    ///
    /// Sector number (0-based)
    fn msf_to_sector(pos: &CDPosition) -> usize {
        (pos.minute as usize * 60 * 75) + (pos.second as usize * 75) + (pos.sector as usize)
    }

    /// Get the number of tracks on the disc
    ///
    /// # Returns
    ///
    /// Number of tracks
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Get track information by track number
    ///
    /// # Arguments
    ///
    /// * `track_num` - Track number (1-99)
    ///
    /// # Returns
    ///
    /// Optional reference to track information
    pub fn get_track(&self, track_num: u8) -> Option<&Track> {
        self.tracks.iter().find(|t| t.number == track_num)
    }
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

        // Bit 6: Data FIFO not empty (always 0 for minimal stub)
        // status |= 0 << 6;

        // Bit 7: Busy (0=Ready, 1=Busy)
        // For minimal stub, always ready unless actively seeking/reading
        if self.state == CDState::Seeking {
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

    #[test]
    fn test_status_register() {
        let mut cdrom = CDROM::new();

        // Initial status: parameter FIFO empty and not full
        let status = cdrom.read_status();
        assert_eq!(status & 0x08, 0x08); // Bit 3: Parameter FIFO empty
        assert_eq!(status & 0x10, 0x10); // Bit 4: Parameter FIFO not full
        assert_eq!(status & 0x20, 0x00); // Bit 5: Response FIFO empty
        assert_eq!(status & 0x80, 0x00); // Bit 7: Not busy

        // Push parameter - FIFO should no longer be empty
        cdrom.push_param(0x12);
        let status = cdrom.read_status();
        assert_eq!(status & 0x08, 0x00); // Bit 3: Parameter FIFO not empty
        assert_eq!(status & 0x10, 0x10); // Bit 4: Parameter FIFO still not full

        // Execute command - response FIFO should have data
        cdrom.execute_command(0x01); // GetStat
        let status = cdrom.read_status();
        assert_eq!(status & 0x20, 0x20); // Bit 5: Response FIFO not empty

        // Set seeking state - should show busy
        cdrom.state = CDState::Seeking;
        let status = cdrom.read_status();
        assert_eq!(status & 0x80, 0x80); // Bit 7: Busy
    }

    #[test]
    fn test_status_register_ready_state() {
        let cdrom = CDROM::new();

        // On initialization, CDROM should report ready state (0x18)
        // Bit 3 (Parameter FIFO empty) = 1
        // Bit 4 (Parameter FIFO not full) = 1
        let status = cdrom.read_status();
        assert_eq!(status & 0x18, 0x18); // Ready state bits
    }

    // Disc Image Loading Tests

    #[test]
    fn test_cue_parsing() {
        let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#;

        let tracks = DiscImage::parse_cue(cue_data).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].number, 1);
        assert_eq!(tracks[0].track_type, TrackType::Mode2_2352);
        assert_eq!(tracks[0].start_position.minute, 0);
        assert_eq!(tracks[0].start_position.second, 0);
        assert_eq!(tracks[0].start_position.sector, 0);
    }

    #[test]
    fn test_cue_parsing_multiple_tracks() {
        let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 10:30:15
  TRACK 03 MODE1/2352
    INDEX 01 25:45:20
"#;

        let tracks = DiscImage::parse_cue(cue_data).unwrap();
        assert_eq!(tracks.len(), 3);

        // Track 1
        assert_eq!(tracks[0].number, 1);
        assert_eq!(tracks[0].track_type, TrackType::Mode2_2352);
        assert_eq!(tracks[0].start_position.minute, 0);

        // Track 2
        assert_eq!(tracks[1].number, 2);
        assert_eq!(tracks[1].track_type, TrackType::Audio);
        assert_eq!(tracks[1].start_position.minute, 10);
        assert_eq!(tracks[1].start_position.second, 30);
        assert_eq!(tracks[1].start_position.sector, 15);

        // Track 3
        assert_eq!(tracks[2].number, 3);
        assert_eq!(tracks[2].track_type, TrackType::Mode1_2352);
        assert_eq!(tracks[2].start_position.minute, 25);
        assert_eq!(tracks[2].start_position.second, 45);
        assert_eq!(tracks[2].start_position.sector, 20);
    }

    #[test]
    fn test_msf_to_sector_conversion() {
        let pos = CDPosition {
            minute: 0,
            second: 2,
            sector: 16,
        };
        let sector = DiscImage::msf_to_sector(&pos);
        assert_eq!(sector, 2 * 75 + 16); // 166

        let pos = CDPosition {
            minute: 1,
            second: 0,
            sector: 0,
        };
        let sector = DiscImage::msf_to_sector(&pos);
        assert_eq!(sector, 60 * 75); // 4500
    }

    #[test]
    fn test_parse_msf() {
        let pos = DiscImage::parse_msf("10:30:15").unwrap();
        assert_eq!(pos.minute, 10);
        assert_eq!(pos.second, 30);
        assert_eq!(pos.sector, 15);

        let pos = DiscImage::parse_msf("00:00:00").unwrap();
        assert_eq!(pos.minute, 0);
        assert_eq!(pos.second, 0);
        assert_eq!(pos.sector, 0);
    }

    #[test]
    fn test_parse_msf_invalid() {
        // Invalid format - only 2 components
        assert!(DiscImage::parse_msf("10:30").is_err());

        // Invalid format - 4 components
        assert!(DiscImage::parse_msf("10:30:15:00").is_err());

        // Invalid numbers
        assert!(DiscImage::parse_msf("abc:def:ghi").is_err());
    }

    #[test]
    fn test_parse_track_type() {
        assert_eq!(
            DiscImage::parse_track_type("MODE1/2352"),
            TrackType::Mode1_2352
        );
        assert_eq!(
            DiscImage::parse_track_type("MODE2/2352"),
            TrackType::Mode2_2352
        );
        assert_eq!(DiscImage::parse_track_type("AUDIO"), TrackType::Audio);

        // Unknown type defaults to Mode2
        assert_eq!(
            DiscImage::parse_track_type("UNKNOWN"),
            TrackType::Mode2_2352
        );
    }

    #[test]
    fn test_disc_image_with_mock_data() {
        // Create a temporary directory for test files
        let temp_dir = std::env::temp_dir();
        let cue_path = temp_dir.join("test_disc.cue");
        let bin_path = temp_dir.join("test_disc.bin");

        // Create a mock .cue file
        let cue_content = r#"FILE "test_disc.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#;
        std::fs::write(&cue_path, cue_content).unwrap();

        // Create a mock .bin file (10 sectors = 23520 bytes)
        let sector_data = vec![0xAB; 2352];
        let mut bin_data = Vec::new();
        for _ in 0..10 {
            bin_data.extend_from_slice(&sector_data);
        }
        std::fs::write(&bin_path, &bin_data).unwrap();

        // Load the disc image
        let disc = DiscImage::load(cue_path.to_str().unwrap()).unwrap();

        // Verify track count
        assert_eq!(disc.track_count(), 1);

        // Verify track info
        let track = disc.get_track(1).unwrap();
        assert_eq!(track.number, 1);
        assert_eq!(track.track_type, TrackType::Mode2_2352);
        assert_eq!(track.length_sectors, 10);

        // Read a sector
        let pos = CDPosition::new(0, 0, 0);
        let sector = disc.read_sector(&pos).unwrap();
        assert_eq!(sector.len(), 2352);
        assert_eq!(sector[0], 0xAB);

        // Read last sector
        let pos = CDPosition::new(0, 0, 9);
        let sector = disc.read_sector(&pos).unwrap();
        assert_eq!(sector.len(), 2352);

        // Read out of bounds
        let pos = CDPosition::new(0, 0, 10);
        assert!(disc.read_sector(&pos).is_none());

        // Clean up
        let _ = std::fs::remove_file(&cue_path);
        let _ = std::fs::remove_file(&bin_path);
    }

    #[test]
    fn test_cdrom_load_disc() {
        // Create a temporary directory for test files
        let temp_dir = std::env::temp_dir();
        let cue_path = temp_dir.join("test_load.cue");
        let bin_path = temp_dir.join("test_load.bin");

        // Create mock files
        let cue_content = r#"FILE "test_load.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#;
        std::fs::write(&cue_path, cue_content).unwrap();

        let bin_data = vec![0x00; 2352 * 5]; // 5 sectors
        std::fs::write(&bin_path, &bin_data).unwrap();

        // Load disc into CDROM
        let mut cdrom = CDROM::new();
        assert!(!cdrom.has_disc());

        cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

        assert!(cdrom.has_disc());
        assert!(!cdrom.status.shell_open);

        // Clean up
        let _ = std::fs::remove_file(&cue_path);
        let _ = std::fs::remove_file(&bin_path);
    }

    #[test]
    fn test_cdrom_read_current_sector() {
        // Create a temporary directory for test files
        let temp_dir = std::env::temp_dir();
        let cue_path = temp_dir.join("test_read.cue");
        let bin_path = temp_dir.join("test_read.bin");

        // Create mock files with recognizable pattern
        let cue_content = r#"FILE "test_read.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
"#;
        std::fs::write(&cue_path, cue_content).unwrap();

        let mut bin_data = Vec::new();
        for i in 0..5 {
            let mut sector = vec![i as u8; 2352];
            bin_data.append(&mut sector);
        }
        std::fs::write(&bin_path, &bin_data).unwrap();

        // Load and read
        let mut cdrom = CDROM::new();
        cdrom.load_disc(cue_path.to_str().unwrap()).unwrap();

        // Read sector at position 00:00:00
        cdrom.set_position(CDPosition::new(0, 0, 0));
        let sector = cdrom.read_current_sector().unwrap();
        assert_eq!(sector.len(), 2352);
        assert_eq!(sector[0], 0); // First sector filled with 0

        // Read sector at position 00:00:03
        cdrom.set_position(CDPosition::new(0, 0, 3));
        let sector = cdrom.read_current_sector().unwrap();
        assert_eq!(sector[0], 3); // Fourth sector filled with 3

        // Clean up
        let _ = std::fs::remove_file(&cue_path);
        let _ = std::fs::remove_file(&bin_path);
    }

    #[test]
    fn test_cdrom_read_without_disc() {
        let mut cdrom = CDROM::new();
        assert!(cdrom.read_current_sector().is_none());
    }

    #[test]
    fn test_cdrom_position_accessors() {
        let mut cdrom = CDROM::new();

        // Check initial position
        let pos = cdrom.position();
        assert_eq!(pos.minute, 0);
        assert_eq!(pos.second, 2);
        assert_eq!(pos.sector, 0);

        // Set new position
        cdrom.set_position(CDPosition::new(10, 30, 15));
        let pos = cdrom.position();
        assert_eq!(pos.minute, 10);
        assert_eq!(pos.second, 30);
        assert_eq!(pos.sector, 15);
    }

    #[test]
    fn test_track_length_calculation_realistic() {
        let cue_data = r#"
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 00:01:00
"#;

        let mut tracks = DiscImage::parse_cue(cue_data).unwrap();

        // Track 1 starts at 0, track 2 starts at 1 second = 75 sectors
        // Total file size: 150 sectors
        DiscImage::calculate_track_lengths(&mut tracks, 2352 * 150);

        // Track 1: 75 sectors (from 0 to 75)
        assert_eq!(tracks[0].length_sectors, 75);

        // Track 2: 75 sectors (from 75 to 150)
        assert_eq!(tracks[1].length_sectors, 75);
    }

    #[test]
    fn test_get_track() {
        let temp_dir = std::env::temp_dir();
        let cue_path = temp_dir.join("test_get_track.cue");
        let bin_path = temp_dir.join("test_get_track.bin");

        let cue_content = r#"FILE "test_get_track.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 00:01:00
"#;
        std::fs::write(&cue_path, cue_content).unwrap();

        let bin_data = vec![0x00; 2352 * 150];
        std::fs::write(&bin_path, &bin_data).unwrap();

        let disc = DiscImage::load(cue_path.to_str().unwrap()).unwrap();

        // Get track 1
        let track1 = disc.get_track(1);
        assert!(track1.is_some());
        assert_eq!(track1.unwrap().number, 1);
        assert_eq!(track1.unwrap().track_type, TrackType::Mode2_2352);

        // Get track 2
        let track2 = disc.get_track(2);
        assert!(track2.is_some());
        assert_eq!(track2.unwrap().number, 2);
        assert_eq!(track2.unwrap().track_type, TrackType::Audio);

        // Get non-existent track
        let track99 = disc.get_track(99);
        assert!(track99.is_none());

        // Clean up
        let _ = std::fs::remove_file(&cue_path);
        let _ = std::fs::remove_file(&bin_path);
    }
}
