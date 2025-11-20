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

//! SPU noise generator
//!
//! The PlayStation SPU includes a noise generator for creating sound effects
//! like explosions, wind, or other non-tonal sounds. It uses a Linear Feedback
//! Shift Register (LFSR) to generate pseudo-random noise.

/// Noise generator using LFSR
///
/// Generates pseudo-random noise samples using a Galois LFSR
/// with configurable frequency.
pub struct NoiseGenerator {
    /// LFSR (Linear Feedback Shift Register) state
    lfsr: u32,

    /// Noise clock frequency shift (0-15)
    clock_shift: u8,

    /// Noise clock step (0-3)
    clock_step: u8,

    /// Current counter for frequency divider
    counter: u32,
}

impl NoiseGenerator {
    /// Create a new noise generator
    ///
    /// # Returns
    ///
    /// Initialized noise generator with default state
    pub fn new() -> Self {
        Self {
            lfsr: 0x0001,
            clock_shift: 0,
            clock_step: 0,
            counter: 0,
        }
    }

    /// Set noise frequency parameters
    ///
    /// The frequency is determined by: freq = step_value >> shift
    /// where step_value depends on clock_step:
    /// - 0: disabled (outputs silence, no contribution to audio output)
    /// - 1: 0x8000
    /// - 2: 0x10000
    /// - 3: 0x20000
    ///
    /// # Arguments
    ///
    /// * `shift` - Frequency shift value (0-15)
    /// * `step` - Frequency step selector (0-3)
    pub fn set_frequency(&mut self, shift: u8, step: u8) {
        self.clock_shift = shift & 0xF;
        self.clock_step = step & 0x3;
    }

    /// Generate next noise sample
    ///
    /// # Returns
    ///
    /// 16-bit noise sample (either 0x7FFF or -0x8000), or 0 when disabled
    #[inline(always)]
    pub fn generate(&mut self) -> i16 {
        // When clock_step is 0, noise is disabled and outputs silence
        if self.clock_step == 0 {
            return 0;
        }

        // Calculate clock divider
        let freq = match self.clock_step {
            1 => 0x8000 >> self.clock_shift,
            2 => 0x10000 >> self.clock_shift,
            3 => 0x20000 >> self.clock_shift,
            _ => 0,
        };

        self.counter += 1;

        if freq > 0 && self.counter >= freq {
            self.counter = 0;
            self.step_lfsr();
        }

        // Output is based on the bottom bit (where feedback is inserted)
        if (self.lfsr & 0x0001) != 0 {
            0x7FFF
        } else {
            -0x8000
        }
    }

    /// Step the LFSR by one tick
    ///
    /// Uses a Galois LFSR with taps at bits 15, 12, 11, and 10
    /// to generate pseudo-random sequences.
    pub(crate) fn step_lfsr(&mut self) {
        // Galois LFSR with taps at bits 15, 12, 11, and 10
        let feedback =
            ((self.lfsr >> 15) ^ (self.lfsr >> 12) ^ (self.lfsr >> 11) ^ (self.lfsr >> 10)) & 1;
        self.lfsr = (self.lfsr << 1) | feedback;
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_generator_creation() {
        let noise = NoiseGenerator::new();
        assert_eq!(noise.lfsr, 0x0001);
        assert_eq!(noise.clock_shift, 0);
        assert_eq!(noise.clock_step, 0);
    }

    #[test]
    fn test_noise_generator_frequency() {
        let mut noise = NoiseGenerator::new();
        noise.set_frequency(0, 1); // shift=0, step=1, freq=0x8000

        // Generate enough samples to trigger at least one LFSR step
        // With shift=0 and step=1, we need 0x8000 (32768) calls
        let samples: Vec<i16> = (0..40000).map(|_| noise.generate()).collect();

        // Verify noise is not constant
        let all_same = samples.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "Noise should not be constant");
    }

    #[test]
    fn test_lfsr_sequence() {
        let mut noise = NoiseGenerator::new();

        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            noise.step_lfsr();
            seen.insert(noise.lfsr);
        }

        // LFSR should produce varied values
        assert!(
            seen.len() > 100,
            "LFSR should produce many different values"
        );
    }

    #[test]
    fn test_noise_output_values() {
        let mut noise = NoiseGenerator::new();
        noise.set_frequency(0, 1);

        for _ in 0..100 {
            let sample = noise.generate();
            // Output should only be 0x7FFF or -0x8000
            assert!(
                sample == 0x7FFF || sample == -0x8000,
                "Noise output should be max or min"
            );
        }
    }
}
