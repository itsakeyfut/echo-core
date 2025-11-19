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

//! SPU voice (audio channel) implementation
//!
//! Each voice can play back ADPCM-compressed audio samples with
//! independent volume, pitch, and ADSR envelope control.

use super::adpcm::ADPCMState;
use super::adsr::{ADSREnvelope, ADSRPhase};

/// Individual voice channel
///
/// Each voice can play back ADPCM-compressed audio samples with
/// independent volume, pitch, and ADSR envelope control.
#[allow(dead_code)]
pub struct Voice {
    /// Voice number (0-23)
    id: u8,

    /// Current volume (left/right)
    pub(crate) volume_left: i16,
    pub(crate) volume_right: i16,

    /// ADSR state
    pub(crate) adsr: ADSREnvelope,

    /// Current sample rate (pitch)
    pub(crate) sample_rate: u16,

    /// Start address in SPU RAM (multiply by 8 for byte address)
    pub(crate) start_address: u16,

    /// Repeat address (loop point, multiply by 8 for byte address)
    pub(crate) repeat_address: u16,

    /// Current address
    pub(crate) current_address: u32,

    /// ADPCM decoder state
    pub(crate) adpcm_state: ADPCMState,

    /// Decoded samples buffer (28 samples per ADPCM block)
    pub(crate) decoded_samples: Vec<i16>,

    /// Voice enabled
    pub(crate) enabled: bool,

    /// Key on flag
    key_on: bool,

    /// Key off flag
    key_off: bool,

    /// Loop flag (set when loop end flag is encountered)
    pub(crate) loop_flag: bool,

    /// Final block flag (set when a non-repeating end block is encountered)
    /// Playback will stop after this block is fully consumed
    pub(crate) final_block: bool,
}

#[allow(dead_code)]
impl Voice {
    /// Create a new voice instance
    ///
    /// # Arguments
    ///
    /// * `id` - Voice number (0-23)
    ///
    /// # Returns
    ///
    /// Initialized voice
    pub fn new(id: u8) -> Self {
        Self {
            id,
            volume_left: 0,
            volume_right: 0,
            adsr: ADSREnvelope::default(),
            sample_rate: 0,
            start_address: 0,
            repeat_address: 0,
            current_address: 0,
            adpcm_state: ADPCMState::default(),
            decoded_samples: Vec::new(),
            enabled: false,
            key_on: false,
            key_off: false,
            loop_flag: false,
            final_block: false,
        }
    }

    /// Trigger key-on for this voice
    ///
    /// Starts playback from the start address and begins the attack phase
    /// of the ADSR envelope.
    pub fn key_on(&mut self) {
        self.enabled = true;
        self.key_on = true;
        self.current_address = (self.start_address as u32) * 8;
        self.adpcm_state = ADPCMState::default();
        self.decoded_samples.clear();
        self.loop_flag = false;
        self.final_block = false;
        self.key_off = false;
        self.adsr.phase = ADSRPhase::Attack;
        self.adsr.level = 0;

        log::trace!("Voice {} key on", self.id);
    }

    /// Trigger key-off for this voice
    ///
    /// Begins the release phase of the ADSR envelope.
    pub fn key_off(&mut self) {
        self.key_off = true;
        self.adsr.phase = ADSRPhase::Release;

        log::trace!("Voice {} key off", self.id);
    }

    /// Render a single stereo sample from this voice
    ///
    /// # Arguments
    ///
    /// * `spu_ram` - Reference to SPU RAM for ADPCM data access
    ///
    /// # Returns
    ///
    /// Tuple of (left, right) 16-bit audio samples
    #[inline(always)]
    pub fn render_sample(&mut self, spu_ram: &[u8]) -> (i16, i16) {
        if !self.enabled || self.adsr.phase == ADSRPhase::Off {
            return (0, 0);
        }

        // Check if we need to decode a new ADPCM block
        if self.needs_decode() {
            self.decode_block(spu_ram);
        }

        // Get interpolated sample at current position
        let sample = self.interpolate_sample();

        // Apply ADSR envelope
        let enveloped = self.apply_envelope(sample);

        // Apply volume (fixed-point multiply with 15-bit fraction)
        let left = ((enveloped as i32 * self.volume_left as i32) >> 15) as i16;
        let right = ((enveloped as i32 * self.volume_right as i32) >> 15) as i16;

        // Advance playback position
        self.advance_position();

        (left, right)
    }

    /// Check if a new ADPCM block needs to be decoded
    ///
    /// # Returns
    ///
    /// True if the decoded samples buffer is empty or the read position has
    /// advanced past the current 28-sample ADPCM block
    fn needs_decode(&self) -> bool {
        self.decoded_samples.is_empty() || self.adpcm_state.position >= 28.0
    }

    /// Decode the current ADPCM block from SPU RAM
    ///
    /// # Arguments
    ///
    /// * `spu_ram` - Reference to SPU RAM for ADPCM data access
    pub(crate) fn decode_block(&mut self, spu_ram: &[u8]) {
        // Calculate block address (each block is 16 bytes)
        let block_addr = (self.current_address as usize) & (spu_ram.len() - 1);

        // Ensure we have a full block available
        if block_addr + 16 > spu_ram.len() {
            self.decoded_samples.clear();
            return;
        }

        let block = &spu_ram[block_addr..block_addr + 16];

        // Check loop flags in block header
        let flags = block[1];
        let loop_end = (flags & 0x01) != 0;
        let loop_repeat = (flags & 0x02) != 0;

        // Decode the block
        self.decoded_samples = self.adpcm_state.decode_block(block);

        // Handle loop flags
        if loop_end {
            // Remember that this block had a loop-end flag
            self.loop_flag = true;
            if loop_repeat {
                // Next block will start from repeat address; we must not
                // auto-increment current_address again when we finish this
                // block, or we'd skip the first loop block
                self.current_address = (self.repeat_address as u32) * 8;
            } else {
                // Mark that this is the final block; playback will stop after
                // this block is fully consumed.
                // (Actual disabling is deferred until advance_position detects
                // position >= 28.0 on a final_block)
                self.final_block = true;
            }
        } else {
            self.loop_flag = false;
        }

        // Reset position to start of new block
        self.adpcm_state.position = 0.0;
    }

    /// Get interpolated sample at current position
    ///
    /// Uses simple linear interpolation for smooth pitch shifting.
    ///
    /// # Returns
    ///
    /// Interpolated 16-bit sample
    #[inline(always)]
    pub(crate) fn interpolate_sample(&self) -> i16 {
        if self.decoded_samples.is_empty() {
            return 0;
        }

        let pos = self.adpcm_state.position;
        let index = pos as usize;

        // Simple linear interpolation (Gaussian would be more accurate but slower)
        if index + 1 < self.decoded_samples.len() {
            let s0 = self.decoded_samples[index] as f32;
            let s1 = self.decoded_samples[index + 1] as f32;
            let frac = pos - index as f32;
            (s0 + (s1 - s0) * frac) as i16
        } else if index < self.decoded_samples.len() {
            self.decoded_samples[index]
        } else {
            0
        }
    }

    /// Apply ADSR envelope to a sample
    ///
    /// # Arguments
    ///
    /// * `sample` - Input sample
    ///
    /// # Returns
    ///
    /// Sample with envelope applied
    #[inline(always)]
    fn apply_envelope(&mut self, sample: i16) -> i16 {
        // Update ADSR envelope
        self.adsr.tick();

        // Apply envelope level (fixed-point multiply)
        ((sample as i32 * self.adsr.level as i32) >> 15) as i16
    }

    /// Advance the playback position
    ///
    /// Updates position based on sample rate and handles block transitions.
    pub(crate) fn advance_position(&mut self) {
        // Calculate step based on sample rate
        // Sample rate is in 4.12 fixed point format
        // Base sample rate is 44100 Hz
        let step = (self.sample_rate as f32) / 4096.0;

        self.adpcm_state.position += step;

        // Check if we've advanced past the current block
        if self.adpcm_state.position >= 28.0 {
            // If this was a non-repeating end block, stop playback now
            // that all samples have been consumed
            if self.final_block {
                self.enabled = false;
                self.adsr.phase = ADSRPhase::Off;
                self.final_block = false;
            }

            // Move to next block (16 bytes per block) unless we've just
            // processed a loop-end block that already updated current_address
            if !self.loop_flag {
                self.current_address += 16;
            }

            // Position will be reset when decode_block is called.
            // Loop end/repeat behavior is handled via loop_flag above.
        }
    }
}
