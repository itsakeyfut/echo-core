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

//! ADPCM (Adaptive Differential Pulse Code Modulation) decoder
//!
//! Implements the PlayStation's ADPCM audio decompression format.
//! ADPCM compresses 16-bit PCM audio to 4 bits per sample using
//! adaptive prediction filters.

/// ADPCM decoder state
///
/// Maintains state for ADPCM audio decompression including previous samples
/// for filter interpolation and current decode position.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ADPCMState {
    /// Previous samples for interpolation (history for filters)
    pub(crate) prev_samples: [i16; 2],

    /// Decoder position within the current block (0.0-28.0)
    pub(crate) position: f32,
}

impl Default for ADPCMState {
    fn default() -> Self {
        Self {
            prev_samples: [0; 2],
            position: 0.0,
        }
    }
}

#[allow(dead_code)]
impl ADPCMState {
    /// Decode a single ADPCM block
    ///
    /// # Arguments
    ///
    /// * `block` - 16-byte ADPCM block to decode
    ///
    /// # Returns
    ///
    /// Vector of 28 decoded 16-bit PCM samples
    ///
    /// # ADPCM Block Format
    ///
    /// ```text
    /// Byte 0: Shift (bits 0-3) | Filter (bits 4-7)
    /// Byte 1: Flags (loop end, loop repeat, etc.)
    /// Bytes 2-15: 14 bytes of nibble pairs (28 samples total)
    /// ```
    pub fn decode_block(&mut self, block: &[u8]) -> Vec<i16> {
        if block.len() < 16 {
            return Vec::new();
        }

        let mut samples = Vec::with_capacity(28);

        // Block header
        let shift = block[0] & 0xF;
        let filter = (block[0] >> 4) & 0x3;
        // Flags in block[1] are for loop control, handled elsewhere

        // Decode 28 samples from 14 bytes (2 samples per byte)
        for i in 0..14 {
            let byte = block[2 + i];

            // Extract two 4-bit samples per byte
            let nibble1 = (byte & 0xF) as i8;
            let nibble2 = ((byte >> 4) & 0xF) as i8;

            // Sign extend 4-bit nibbles to 8-bit
            let nibble1_signed = (nibble1 << 4) >> 4;
            let nibble2_signed = (nibble2 << 4) >> 4;

            // Apply shift (scale up, then shift right)
            let sample1 = ((nibble1_signed as i16) << 12) >> shift;
            let sample2 = ((nibble2_signed as i16) << 12) >> shift;

            // Apply filter
            let decoded1 = self.apply_filter(sample1, filter);
            let decoded2 = self.apply_filter(sample2, filter);

            samples.push(decoded1);
            samples.push(decoded2);
        }

        samples
    }

    /// Apply ADPCM filter to a sample
    ///
    /// # Arguments
    ///
    /// * `sample` - Input sample after shift
    /// * `filter` - Filter mode (0-3)
    ///
    /// # Returns
    ///
    /// Filtered and clamped sample
    ///
    /// # Filter Modes
    ///
    /// - Filter 0: No filtering, pass through
    /// - Filter 1: Simple first-order prediction
    /// - Filter 2: Second-order prediction with both previous samples
    /// - Filter 3: Alternative second-order prediction
    #[inline(always)]
    fn apply_filter(&mut self, sample: i16, filter: u8) -> i16 {
        let result = match filter {
            0 => sample as i32,
            1 => {
                // Filter 1: s + old[0] + (-old[0] >> 1)
                sample as i32 + self.prev_samples[0] as i32 + (-(self.prev_samples[0] as i32) >> 1)
            }
            2 => {
                // Filter 2: s + old[0]*2 + (-old[0]*3 >> 1) - old[1] + (old[1] >> 1)
                sample as i32
                    + (self.prev_samples[0] as i32 * 2)
                    + ((-(self.prev_samples[0] as i32) * 3) >> 1)
                    - self.prev_samples[1] as i32
                    + (self.prev_samples[1] as i32 >> 1)
            }
            3 => {
                // Filter 3: s + old[0]*2 - (old[0]*5 >> 2) + old[1]*2 - (old[1] >> 1)
                sample as i32 + (self.prev_samples[0] as i32 * 2)
                    - ((self.prev_samples[0] as i32 * 5) >> 2)
                    + (self.prev_samples[1] as i32 * 2)
                    - (self.prev_samples[1] as i32 >> 1)
            }
            _ => sample as i32,
        };

        // Clamp to i16 range
        let clamped = result.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        // Update history (shift old samples)
        self.prev_samples[1] = self.prev_samples[0];
        self.prev_samples[0] = clamped;

        clamped
    }
}
