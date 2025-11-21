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

//! CD-DA (Compact Disc Digital Audio) playback module
//!
//! Handles CD audio track playback for music in PSX games.
//! CD audio is 44.1kHz, 16-bit stereo PCM audio stored in 2352-byte sectors.
//! Each sector contains 588 stereo samples (2352 bytes / 4 bytes per sample).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// CD-DA audio player
///
/// Handles playback of CD audio tracks from disc image files.
/// CD audio is stored as raw PCM data in disc sectors.
pub struct CDAudio {
    /// CD audio file handle (.bin file)
    file: Option<File>,

    /// Current playback position (sector)
    current_sector: u32,

    /// Playback start/end sectors
    play_start: u32,
    play_end: u32,

    /// Playing state
    playing: bool,

    /// Loop mode
    looping: bool,

    /// Volume (left/right)
    pub(crate) volume_left: i16,
    pub(crate) volume_right: i16,

    /// Sample buffer (2352 bytes per sector = 588 stereo samples)
    buffer: Vec<i16>,
    buffer_position: usize,
}

impl CDAudio {
    /// Create a new CD-DA audio player
    ///
    /// # Returns
    ///
    /// Initialized CD audio player with default settings
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::cd_audio::CDAudio;
    ///
    /// let cd_audio = CDAudio::new();
    /// ```
    pub fn new() -> Self {
        Self {
            file: None,
            current_sector: 0,
            play_start: 0,
            play_end: 0,
            playing: false,
            looping: false,
            volume_left: 0x80,
            volume_right: 0x80,
            buffer: Vec::new(),
            buffer_position: 0,
        }
    }

    /// Load CD image for audio playback
    ///
    /// Opens the disc image file (.bin) for reading CD audio tracks.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .bin file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if disc loaded successfully
    /// - `Err(std::io::Error)` if loading failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cdrom::cd_audio::CDAudio;
    ///
    /// let mut cd_audio = CDAudio::new();
    /// cd_audio.load_disc("game.bin").unwrap();
    /// ```
    pub fn load_disc(&mut self, path: &str) -> Result<(), std::io::Error> {
        self.file = Some(File::open(path)?);
        log::info!("CD-DA: Loaded disc from {}", path);
        Ok(())
    }

    /// Start CD-DA playback
    ///
    /// Begins playing CD audio from the specified sector range.
    ///
    /// # Arguments
    ///
    /// * `start_sector` - Starting sector number
    /// * `end_sector` - Ending sector number
    /// * `looping` - Whether to loop playback
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::cd_audio::CDAudio;
    ///
    /// let mut cd_audio = CDAudio::new();
    /// cd_audio.play(100, 200, false);
    /// ```
    pub fn play(&mut self, start_sector: u32, end_sector: u32, looping: bool) {
        self.play_start = start_sector;
        self.play_end = end_sector;
        self.current_sector = start_sector;
        self.looping = looping;
        self.playing = true;
        self.buffer.clear();
        self.buffer_position = 0;

        log::debug!(
            "CD-DA play: sectors {}-{}, loop={}",
            start_sector,
            end_sector,
            looping
        );
    }

    /// Stop CD-DA playback
    ///
    /// Stops audio playback and clears buffers.
    pub fn stop(&mut self) {
        self.playing = false;
        self.buffer.clear();
        log::debug!("CD-DA stopped");
    }

    /// Set volume for CD audio
    ///
    /// # Arguments
    ///
    /// * `left` - Left channel volume (0-255)
    /// * `right` - Right channel volume (0-255)
    pub fn set_volume(&mut self, left: u8, right: u8) {
        self.volume_left = left as i16;
        self.volume_right = right as i16;
    }

    /// Check if CD audio is currently playing
    ///
    /// # Returns
    ///
    /// true if playing, false otherwise
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Get next stereo sample
    ///
    /// Returns the next stereo sample from the CD audio stream.
    /// Automatically handles sector reading and looping.
    ///
    /// # Returns
    ///
    /// Stereo sample (left, right) with volume applied
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cdrom::cd_audio::CDAudio;
    ///
    /// let mut cd_audio = CDAudio::new();
    /// let (left, right) = cd_audio.get_sample();
    /// ```
    #[inline(always)]
    pub fn get_sample(&mut self) -> (i16, i16) {
        if !self.playing {
            return (0, 0);
        }

        // Refill buffer if needed
        if self.buffer_position >= self.buffer.len() {
            if let Err(e) = self.read_sector() {
                log::error!("CD-DA read error: {}", e);
                self.stop();
                return (0, 0);
            }
            self.buffer_position = 0;
        }

        // Get stereo sample
        let left = self.buffer[self.buffer_position];
        let right = self.buffer[self.buffer_position + 1];
        self.buffer_position += 2;

        // Apply volume (scale by volume/128)
        let left = (left as i32 * self.volume_left as i32) >> 7;
        let right = (right as i32 * self.volume_right as i32) >> 7;

        // Clamp to i16 range to avoid wrap-around
        let left = left.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let right = right.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        (left, right)
    }

    /// Read a sector from disc and convert to PCM samples
    ///
    /// Reads raw sector data and converts it to 16-bit stereo samples.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if sector read successfully
    /// - `Err(std::io::Error)` if reading fails
    fn read_sector(&mut self) -> Result<(), std::io::Error> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No disc loaded"))?;

        // Seek to sector (2352 bytes per sector)
        let offset = self.current_sector as u64 * 2352;
        file.seek(SeekFrom::Start(offset))?;

        // Read raw sector data
        let mut raw_data = vec![0u8; 2352];
        file.read_exact(&mut raw_data)?;

        // Convert to 16-bit stereo samples
        // CD audio is 44.1kHz, 16-bit stereo = 588 samples/sector
        self.buffer.clear();
        for chunk in raw_data.chunks_exact(4) {
            let left = i16::from_le_bytes([chunk[0], chunk[1]]);
            let right = i16::from_le_bytes([chunk[2], chunk[3]]);
            self.buffer.push(left);
            self.buffer.push(right);
        }

        // Advance sector
        self.current_sector += 1;

        // Check for end
        if self.current_sector > self.play_end {
            if self.looping {
                self.current_sector = self.play_start;
                log::trace!("CD-DA: Looping to sector {}", self.play_start);
            } else {
                self.stop();
            }
        }

        Ok(())
    }
}

impl Default for CDAudio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cd_audio_initialization() {
        let cd_audio = CDAudio::new();
        assert!(!cd_audio.is_playing());
        assert_eq!(cd_audio.volume_left, 0x80);
        assert_eq!(cd_audio.volume_right, 0x80);
    }

    #[test]
    fn test_cd_audio_playback_state() {
        let mut cd_audio = CDAudio::new();

        // Start playback
        cd_audio.play(100, 200, false);
        assert!(cd_audio.is_playing());
        assert_eq!(cd_audio.current_sector, 100);
        assert_eq!(cd_audio.play_start, 100);
        assert_eq!(cd_audio.play_end, 200);
        assert!(!cd_audio.looping);

        // Stop playback
        cd_audio.stop();
        assert!(!cd_audio.is_playing());
    }

    #[test]
    fn test_cd_audio_looping() {
        let mut cd_audio = CDAudio::new();

        // Start looping playback
        cd_audio.play(100, 105, true);
        assert!(cd_audio.is_playing());
        assert!(cd_audio.looping);
    }

    #[test]
    fn test_cd_audio_volume() {
        let mut cd_audio = CDAudio::new();
        cd_audio.set_volume(0x40, 0x40);

        assert_eq!(cd_audio.volume_left, 0x40);
        assert_eq!(cd_audio.volume_right, 0x40);
    }

    #[test]
    fn test_cd_audio_get_sample_when_not_playing() {
        let mut cd_audio = CDAudio::new();
        let (left, right) = cd_audio.get_sample();
        assert_eq!(left, 0);
        assert_eq!(right, 0);
    }
}
