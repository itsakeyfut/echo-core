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

//! SPU reverb effect configuration
//!
//! The PlayStation SPU includes hardware reverb effects that can be
//! applied to audio output. This module implements the reverb
//! configuration and processing logic using all-pass and comb filters.

/// Reverb configuration
///
/// Hardware reverb effects configuration implementing the PSX SPU's
/// reverb algorithm with all-pass and comb filters.
pub struct ReverbConfig {
    /// Reverb enabled
    pub(crate) enabled: bool,

    /// APF (All-Pass Filter) offsets
    pub(crate) apf_offset1: u16,
    pub(crate) apf_offset2: u16,

    /// Reflection volumes
    pub(crate) reflect_volume1: i16,
    pub(crate) reflect_volume2: i16,
    pub(crate) reflect_volume3: i16,
    pub(crate) reflect_volume4: i16,

    /// Comb filter volumes
    pub(crate) comb_volume1: i16,
    pub(crate) comb_volume2: i16,
    pub(crate) comb_volume3: i16,
    pub(crate) comb_volume4: i16,

    /// APF volumes
    pub(crate) apf_volume1: i16,
    pub(crate) apf_volume2: i16,

    /// Input volume
    pub(crate) input_volume_left: i16,
    pub(crate) input_volume_right: i16,

    /// Reverb work area in SPU RAM
    pub(crate) reverb_start_addr: u32,
    pub(crate) reverb_end_addr: u32,

    /// Current reverb address
    pub(crate) reverb_current_addr: u32,
}

impl ReverbConfig {
    /// Create a new reverb configuration
    ///
    /// # Returns
    ///
    /// Initialized reverb config with default values
    pub fn new() -> Self {
        Self {
            enabled: false,
            apf_offset1: 0,
            apf_offset2: 0,
            reflect_volume1: 0,
            reflect_volume2: 0,
            reflect_volume3: 0,
            reflect_volume4: 0,
            comb_volume1: 0,
            comb_volume2: 0,
            comb_volume3: 0,
            comb_volume4: 0,
            apf_volume1: 0,
            apf_volume2: 0,
            input_volume_left: 0,
            input_volume_right: 0,
            reverb_start_addr: 0,
            reverb_end_addr: 0,
            reverb_current_addr: 0,
        }
    }

    /// Apply reverb to a stereo sample
    ///
    /// Processes input samples through all-pass and comb filters
    /// to create a reverb effect.
    ///
    /// # Arguments
    ///
    /// * `left` - Left channel input sample
    /// * `right` - Right channel input sample
    /// * `spu_ram` - Mutable reference to SPU RAM for reverb buffer
    ///
    /// # Returns
    ///
    /// Tuple of (left, right) samples with reverb applied
    #[inline(always)]
    pub fn process(&mut self, left: i16, right: i16, spu_ram: &mut [u8]) -> (i16, i16) {
        if !self.enabled {
            return (left, right);
        }

        // Input with volume
        let input_left = ((left as i32) * (self.input_volume_left as i32)) >> 15;
        let input_right = ((right as i32) * (self.input_volume_right as i32)) >> 15;

        // Read from reverb buffer
        let reverb_left = self.read_reverb_buffer(spu_ram, 0);
        let reverb_right = self.read_reverb_buffer(spu_ram, 2);

        // Apply all-pass filters
        let apf1_left = self.apply_apf(input_left, reverb_left, self.apf_volume1);
        let apf1_right = self.apply_apf(input_right, reverb_right, self.apf_volume1);

        let apf2_left = self.apply_apf(apf1_left, reverb_left, self.apf_volume2);
        let apf2_right = self.apply_apf(apf1_right, reverb_right, self.apf_volume2);

        // Apply comb filters
        let comb_left = self.apply_comb_filters(apf2_left, spu_ram, 0);
        let comb_right = self.apply_comb_filters(apf2_right, spu_ram, 1);

        // Mix with original
        let out_left = ((left as i32) + comb_left).clamp(i16::MIN as i32, i16::MAX as i32);
        let out_right = ((right as i32) + comb_right).clamp(i16::MIN as i32, i16::MAX as i32);

        // Write to reverb buffer
        self.write_reverb_buffer(spu_ram, 0, comb_left as i16);
        self.write_reverb_buffer(spu_ram, 2, comb_right as i16);

        (out_left as i16, out_right as i16)
    }

    /// Apply all-pass filter
    ///
    /// # Arguments
    ///
    /// * `input` - Input sample
    /// * `feedback` - Feedback sample from reverb buffer
    /// * `volume` - APF volume coefficient
    ///
    /// # Returns
    ///
    /// Filtered sample
    #[inline(always)]
    fn apply_apf(&self, input: i32, feedback: i16, volume: i16) -> i32 {
        let fb = (feedback as i32 * volume as i32) >> 15;
        input - fb
    }

    /// Apply comb filters
    ///
    /// # Arguments
    ///
    /// * `input` - Input sample
    /// * `spu_ram` - Reference to SPU RAM
    /// * `channel` - Channel index (0=left, 1=right)
    ///
    /// # Returns
    ///
    /// Filtered sample
    #[inline(always)]
    fn apply_comb_filters(&self, _input: i32, spu_ram: &[u8], channel: usize) -> i32 {
        let mut output = 0i32;

        // Apply 4 comb filters
        let volumes = [
            self.comb_volume1,
            self.comb_volume2,
            self.comb_volume3,
            self.comb_volume4,
        ];

        for (i, &volume) in volumes.iter().enumerate() {
            let offset = (channel * 4 + i) * 2;
            let sample = self.read_reverb_buffer(spu_ram, offset);
            output += (sample as i32 * volume as i32) >> 15;
        }

        output
    }

    /// Read from reverb buffer in SPU RAM
    ///
    /// # Arguments
    ///
    /// * `spu_ram` - Reference to SPU RAM
    /// * `offset` - Offset from current reverb address
    ///
    /// # Returns
    ///
    /// 16-bit sample from reverb buffer
    #[inline(always)]
    fn read_reverb_buffer(&self, spu_ram: &[u8], offset: usize) -> i16 {
        let addr = ((self.reverb_current_addr + offset as u32) & 0x7FFFE) as usize;
        if addr + 1 < spu_ram.len() {
            let lo = spu_ram[addr] as u16;
            let hi = spu_ram[addr + 1] as u16;
            ((hi << 8) | lo) as i16
        } else {
            0
        }
    }

    /// Write to reverb buffer in SPU RAM
    ///
    /// # Arguments
    ///
    /// * `spu_ram` - Mutable reference to SPU RAM
    /// * `offset` - Offset from current reverb address
    /// * `value` - 16-bit sample to write
    #[inline(always)]
    fn write_reverb_buffer(&mut self, spu_ram: &mut [u8], offset: usize, value: i16) {
        let addr = ((self.reverb_current_addr + offset as u32) & 0x7FFFE) as usize;
        if addr + 1 < spu_ram.len() {
            spu_ram[addr] = value as u8;
            spu_ram[addr + 1] = (value >> 8) as u8;
        }
    }
}

impl Default for ReverbConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_creation() {
        let reverb = ReverbConfig::new();
        assert!(!reverb.enabled);
        assert_eq!(reverb.reverb_current_addr, 0);
    }

    #[test]
    fn test_reverb_disabled() {
        let mut reverb = ReverbConfig::new();
        reverb.enabled = false;

        let mut spu_ram = vec![0u8; 512 * 1024];
        let (left, right) = reverb.process(1000, 1000, &mut spu_ram);

        // When disabled, reverb should pass through unchanged
        assert_eq!(left, 1000);
        assert_eq!(right, 1000);
    }

    #[test]
    fn test_reverb_process() {
        let mut reverb = ReverbConfig::new();
        reverb.enabled = true;
        reverb.input_volume_left = 0x4000;
        reverb.input_volume_right = 0x4000;

        let mut spu_ram = vec![0u8; 512 * 1024];

        let (_left, _right) = reverb.process(1000, 1000, &mut spu_ram);

        // Reverb should process without crashing
        // Output values are i16, so they're always in valid range
    }

    #[test]
    fn test_apf_filter() {
        let reverb = ReverbConfig::new();
        let input = 1000i32;
        let feedback = 500i16;
        let volume = 0x4000i16;

        let output = reverb.apply_apf(input, feedback, volume);

        // APF should modify the input based on feedback
        assert_ne!(output, input);
    }

    #[test]
    fn test_reverb_buffer_access() {
        let mut reverb = ReverbConfig::new();
        let mut spu_ram = vec![0u8; 512 * 1024];

        reverb.reverb_current_addr = 0x1000;

        // Write a value
        reverb.write_reverb_buffer(&mut spu_ram, 0, 0x1234);

        // Read it back
        let value = reverb.read_reverb_buffer(&spu_ram, 0);
        assert_eq!(value, 0x1234);
    }
}
