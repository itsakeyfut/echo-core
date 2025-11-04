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
    /// Parameters are consumed but not yet implemented.
    pub(super) fn cmd_setmode(&mut self) {
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
    pub(super) fn cmd_readtoc(&mut self) {
        log::debug!("CD-ROM: ReadTOC");

        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(3); // INT3 (acknowledge)

        // Second response after TOC read completes
        self.response_fifo.push_back(self.get_status_byte());
        self.trigger_interrupt(2); // INT2 (complete)
    }
}
