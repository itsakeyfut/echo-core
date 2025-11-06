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

//! Disc image loading and management
//!
//! This module handles loading CD-ROM disc images from .cue/.bin files
//! and provides sector reading functionality.

use super::CDPosition;
use crate::core::error::CdRomError;

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
    /// - `Err(CdRomError)` if loading failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::cdrom::DiscImage;
    ///
    /// let disc = DiscImage::load("game.cue").unwrap();
    /// ```
    pub fn load(cue_path: &str) -> Result<Self, CdRomError> {
        let cue_data = std::fs::read_to_string(cue_path)?;
        let bin_path = Self::get_bin_path_from_cue(cue_path, &cue_data)?;

        let mut tracks = Self::parse_cue(&cue_data)?;
        let data = std::fs::read(&bin_path).map_err(|e| {
            CdRomError::DiscLoadError(format!("Failed to read bin file '{}': {}", bin_path, e))
        })?;

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
    fn get_bin_path_from_cue(cue_path: &str, cue_data: &str) -> Result<String, CdRomError> {
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

        Err(CdRomError::DiscLoadError(
            "No FILE directive found in .cue file".to_string(),
        ))
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
    pub(super) fn parse_cue(cue_data: &str) -> Result<Vec<Track>, CdRomError> {
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
    pub(super) fn parse_msf(msf: &str) -> Result<CDPosition, CdRomError> {
        let parts: Vec<&str> = msf.split(':').collect();
        if parts.len() != 3 {
            return Err(CdRomError::DiscLoadError(format!(
                "Invalid MSF format: '{}'",
                msf
            )));
        }

        let minute = parts[0]
            .parse()
            .map_err(|_| CdRomError::DiscLoadError(format!("Invalid minute in MSF: '{}'", msf)))?;
        let second = parts[1]
            .parse()
            .map_err(|_| CdRomError::DiscLoadError(format!("Invalid second in MSF: '{}'", msf)))?;
        let sector = parts[2]
            .parse()
            .map_err(|_| CdRomError::DiscLoadError(format!("Invalid sector in MSF: '{}'", msf)))?;

        Ok(CDPosition {
            minute,
            second,
            sector,
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
    pub(super) fn parse_track_type(s: &str) -> TrackType {
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
    pub(super) fn calculate_track_lengths(tracks: &mut [Track], file_size: usize) {
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
    /// Sector number (0-based, accounting for 2-second pregap)
    pub(super) fn msf_to_sector(pos: &CDPosition) -> usize {
        let total = (pos.minute as u32 * 60 * 75) + (pos.second as u32 * 75) + pos.sector as u32;
        total.saturating_sub(150) as usize
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
