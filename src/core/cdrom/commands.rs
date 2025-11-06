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

//! CD-ROM command implementations
//!
//! This module contains implementations of all CD-ROM commands
//! (GetStat, SetLoc, ReadN, etc.)

use super::{bcd_to_dec, CDPosition, CDState, CDROM};

impl CDROM {
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
            0x19 => self.cmd_test(),
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
    pub(super) fn cmd_getstat(&mut self) {
        log::trace!("CD-ROM: GetStat");
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x02: SetLoc
    ///
    /// Sets the seek target position from 3 parameter bytes (MM:SS:FF in BCD).
    pub(super) fn cmd_setloc(&mut self) {
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
    pub(super) fn cmd_readn(&mut self) {
        log::debug!("CD-ROM: ReadN");
        self.state = CDState::Reading;
        self.status.reading = true;
        self.read_ticks = 0; // Reset read timer

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Actual sector data will be read by tick() after appropriate timing
        // INT1 interrupts will be triggered when each sector is ready
    }

    /// Command 0x09: Pause
    ///
    /// Pause reading or audio playback.
    pub(super) fn cmd_pause(&mut self) {
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
    pub(super) fn cmd_init(&mut self) {
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
    ///
    /// # Mode byte format (parameter)
    ///
    /// ```text
    /// Bit 0: CD-DA mode (0=Off, 1=On)
    /// Bit 1: Auto Pause (0=Off, 1=On)
    /// Bit 2: Report (0=Off, 1=Report interrupts for all sectors)
    /// Bit 3: XA-Filter (0=Off, 1=Process only XA-ADPCM sectors)
    /// Bit 4: Ignore Bit (0=Off, 1=Ignore sector size and setloc position)
    /// Bit 5: Sector Size (0=2048 bytes, 1=2340 bytes)
    /// Bit 6: XA-ADPCM (0=Off, 1=Send XA-ADPCM to SPU)
    /// Bit 7: Double Speed (0=Off, 1=On, 2x speed)
    /// ```
    pub(super) fn cmd_setmode(&mut self) {
        if self.param_fifo.is_empty() {
            log::warn!("CD-ROM: SetMode with no parameters");
            self.error_response();
            return;
        }

        let mode_byte = self.param_fifo.pop_front().unwrap();
        log::debug!("CD-ROM: SetMode = 0x{:02X}", mode_byte);

        // Parse mode byte and update mode settings
        self.mode.cdda_report = (mode_byte & 0x01) != 0;
        self.mode.auto_pause = (mode_byte & 0x02) != 0;
        self.mode.report_all = (mode_byte & 0x04) != 0;
        self.mode.xa_filter = (mode_byte & 0x08) != 0;
        self.mode.ignore_bit = (mode_byte & 0x10) != 0;
        self.mode.size_2340 = (mode_byte & 0x20) != 0;
        self.mode.xa_adpcm = (mode_byte & 0x40) != 0;
        self.mode.double_speed = (mode_byte & 0x80) != 0;

        log::trace!(
            "CD-ROM: Mode settings - Speed: {}x, Size: {} bytes, XA-ADPCM: {}, Report All: {}",
            if self.mode.double_speed { 2 } else { 1 },
            if self.mode.size_2340 { 2340 } else { 2048 },
            self.mode.xa_adpcm,
            self.mode.report_all
        );

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)
    }

    /// Command 0x15: SeekL
    ///
    /// Seek to target position (data mode).
    pub(super) fn cmd_seekl(&mut self) {
        log::debug!("CD-ROM: SeekL");

        if self.seek_target.is_some() {
            self.state = CDState::Seeking;
            self.status.seeking = true;
            self.seek_ticks = 0; // Reset seek timer

            self.response_fifo.push_back(self.get_status_byte());
            self.trigger_interrupt(3); // INT3 (acknowledge)

        // The actual seek will complete in tick() after the appropriate delay
        // INT2 will be triggered when the seek completes
        } else {
            log::warn!("CD-ROM: SeekL with no target set");
            self.error_response();
        }
    }

    /// Command 0x19: Test
    ///
    /// Test/diagnostic commands with various sub-functions.
    ///
    /// # Sub-functions (first parameter byte)
    ///
    /// - 0x20: Get BIOS date/version (returns 4 bytes: YY, MM, DD, Version in BCD)
    /// - 0x04: Get CD controller chip ID (returns 5 bytes)
    /// - Other sub-functions are hardware diagnostic tests
    ///
    /// # Response
    ///
    /// Varies by sub-function. Most return status byte and test results.
    pub(super) fn cmd_test(&mut self) {
        if self.param_fifo.is_empty() {
            log::warn!("CD-ROM: Test with no parameters");
            self.error_response();
            return;
        }

        let subfunction = self.param_fifo.pop_front().unwrap();
        log::debug!("CD-ROM: Test sub-function 0x{:02X}", subfunction);

        match subfunction {
            0x20 => {
                // Get BIOS date/version
                // Real hardware returns: YY, MM, DD, Version (4 bytes)
                // For emulation, return a fixed date: 1998/08/07 (SCPH-1001 date)
                self.response_fifo.push_back(0x98); // Year (98 = 1998)
                self.response_fifo.push_back(0x08); // Month
                self.response_fifo.push_back(0x07); // Day
                self.response_fifo.push_back(0xC3); // Version byte

                log::trace!("CD-ROM: Test 0x20 - Returned BIOS date 1998/08/07");
                self.trigger_interrupt(3); // INT3 (acknowledge)
            }
            0x04 => {
                // Get CD controller chip ID/version
                // Return actual drive status and fixed chip ID for emulation
                self.response_fifo.push_back(self.get_status_byte());
                self.response_fifo.push_back(0x00); // Chip ID byte 1
                self.response_fifo.push_back(0x00); // Chip ID byte 2
                self.response_fifo.push_back(0x00); // Chip ID byte 3
                self.response_fifo.push_back(0x00); // Chip ID byte 4

                log::trace!("CD-ROM: Test 0x04 - Returned chip ID");
                self.trigger_interrupt(3); // INT3 (acknowledge)
            }
            _ => {
                log::warn!("CD-ROM: Unknown Test sub-function 0x{:02X}", subfunction);
                // For unknown test commands, return status byte
                self.response_fifo.push_back(self.get_status_byte());
                self.trigger_interrupt(3); // INT3 (acknowledge)
            }
        }
    }

    /// Command 0x1A: GetID
    ///
    /// Get disc identification (region, disc type, etc).
    pub(super) fn cmd_getid(&mut self) {
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
    pub(super) fn cmd_reads(&mut self) {
        log::debug!("CD-ROM: ReadS");

        self.state = CDState::Reading;
        self.status.reading = true;
        self.read_ticks = 0; // Reset read timer

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Actual sector data will be read by tick() after appropriate timing
        // INT1 interrupts will be triggered when each sector is ready
    }

    /// Command 0x1E: ReadTOC
    ///
    /// Read table of contents from disc.
    ///
    /// This command reads the disc's TOC (track information) and stores it
    /// internally. The TOC is used by subsequent commands like GetTD (Get Track Duration).
    ///
    /// # Response
    ///
    /// First response (INT3): Status byte
    /// Second response (INT2): Status byte (after TOC read completes)
    ///
    /// # Timing
    ///
    /// The TOC read takes approximately 1 second on real hardware.
    /// For now, we respond immediately.
    pub(super) fn cmd_readtoc(&mut self) {
        log::debug!("CD-ROM: ReadTOC");

        if self.disc.is_none() {
            log::warn!("CD-ROM: ReadTOC with no disc loaded");
            self.status.id_error = true;
            self.error_response();
            return;
        }

        // First response: acknowledge
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Log TOC information for debugging
        if let Some(ref disc) = self.disc {
            let track_count = disc.track_count();
            log::debug!("CD-ROM: ReadTOC - {} tracks on disc", track_count);

            for i in 1..=track_count {
                if let Some(track) = disc.get_track(i as u8) {
                    log::trace!(
                        "CD-ROM: Track {} - Type: {:?}, Start: {:02}:{:02}:{:02}, Length: {} sectors",
                        track.number,
                        track.track_type,
                        track.start_position.minute,
                        track.start_position.second,
                        track.start_position.sector,
                        track.length_sectors
                    );
                }
            }
        }

        // Second response: TOC read complete
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(2); // INT2 (complete)
    }
}
