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

//! Noise generator tests - noise generation and frequency control

use crate::core::spu::noise::NoiseGenerator;

#[test]
fn test_noise_generator_creation() {
    let mut noise = NoiseGenerator::new();
    // Verify initial state (step=0 means disabled, outputs 0)
    let sample = noise.generate();
    assert_eq!(sample, 0, "Disabled noise generator should output 0");

    // Enable noise and verify it outputs non-zero
    noise.set_frequency(0, 1);
    let sample = noise.generate();
    assert!(
        sample == 0x7FFF || sample == -0x8000,
        "Enabled noise should output max or min"
    );
}

#[test]
fn test_noise_generator_frequency_setting() {
    let mut noise = NoiseGenerator::new();
    noise.set_frequency(0, 1); // shift=0, step=1, freq=0x8000

    // Generate enough samples to trigger at least one LFSR step
    // With shift=0 and step=1, we need 0x8000 (32768) calls
    let samples: Vec<i16> = (0..40000).map(|_| noise.generate()).collect();

    // Verify noise is not constant
    let all_same = samples.windows(2).all(|w| w[0] == w[1]);
    assert!(!all_same, "Noise should not be constant");
}
