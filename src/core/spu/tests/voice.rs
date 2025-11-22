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

//! Voice channel tests - voice rendering, interpolation, and playback

use crate::core::spu::adsr::ADSRPhase;
use crate::core::spu::noise::NoiseGenerator;
use crate::core::spu::voice::Voice;

#[test]
fn test_voice_render_sample_disabled() {
    let mut voice = Voice::new(0);
    let spu_ram = vec![0u8; 512 * 1024];
    let mut noise = NoiseGenerator::new();

    voice.enabled = false;
    let (left, right) = voice.render_sample(&spu_ram, &mut noise);

    assert_eq!(left, 0);
    assert_eq!(right, 0);
}

#[test]
fn test_voice_render_sample_with_volume() {
    let mut voice = Voice::new(0);
    let mut spu_ram = vec![0u8; 512 * 1024];
    let mut noise = NoiseGenerator::new();

    // Create a simple ADPCM block with known output
    spu_ram[0] = 0x00; // Shift=0, Filter=0
    spu_ram[1] = 0x00; // No loop flags
    spu_ram[2] = 0x11; // Nibbles: 1, 1

    voice.enabled = true;
    voice.adsr.phase = ADSRPhase::Attack;
    voice.adsr.level = 32767; // Max envelope
    voice.volume_left = 16384; // 50% volume
    voice.volume_right = 16384;
    voice.sample_rate = 4096; // Normal pitch (1.0x)
    voice.start_address = 0;
    voice.current_address = 0;

    let (left, right) = voice.render_sample(&spu_ram, &mut noise);

    // Should have some output
    // Exact value depends on ADPCM decoding and interpolation
    // Just verify it's not zero
    assert!(left != 0 || right != 0);
}

#[test]
fn test_voice_interpolation() {
    let mut voice = Voice::new(0);

    // Manually set up decoded samples
    voice.decoded_samples = vec![0, 1000, 2000, 3000];
    voice.adpcm_state.position = 1.5; // Halfway between index 1 and 2

    let sample = voice.interpolate_sample();

    // Should interpolate between 1000 and 2000
    // 1000 + (2000 - 1000) * 0.5 = 1500
    assert_eq!(sample, 1500);
}

#[test]
fn test_voice_advance_position() {
    let mut voice = Voice::new(0);

    voice.sample_rate = 4096; // 1.0x speed
    voice.adpcm_state.position = 0.0;
    voice.current_address = 0;

    voice.advance_position();

    // Position should have advanced by 1.0
    assert_eq!(voice.adpcm_state.position, 1.0);

    // Set position near end of block
    voice.adpcm_state.position = 27.5;
    voice.advance_position();

    // Should advance past block boundary
    assert!(voice.adpcm_state.position >= 28.0);
    assert_eq!(voice.current_address, 16); // Moved to next block
}

#[test]
fn test_voice_decode_block_with_loop() {
    let mut voice = Voice::new(0);
    let mut spu_ram = vec![0u8; 512 * 1024];

    // Create an ADPCM block with loop flags
    spu_ram[0] = 0x00; // Shift=0, Filter=0
    spu_ram[1] = 0x03; // Loop end + loop repeat
    spu_ram[2] = 0x11;

    voice.current_address = 0;
    voice.repeat_address = 10; // Loop back to address 80 (10 * 8)
    voice.enabled = true;

    voice.decode_block(&spu_ram);

    // Should have decoded samples
    assert_eq!(voice.decoded_samples.len(), 28);

    // Loop flag should be set
    assert!(voice.loop_flag);

    // Current address should be set to repeat address
    assert_eq!(voice.current_address, 80);
}

#[test]
fn test_voice_decode_block_end_without_loop() {
    let mut voice = Voice::new(0);
    let mut spu_ram = vec![0u8; 512 * 1024];

    // Create an ADPCM block with end flag only
    spu_ram[0] = 0x00;
    spu_ram[1] = 0x01; // Loop end only (no repeat)

    voice.current_address = 0;
    voice.enabled = true;
    voice.adsr.phase = ADSRPhase::Sustain;

    voice.decode_block(&spu_ram);

    // Voice should still be enabled (final block is not consumed yet)
    assert!(voice.enabled);
    assert_eq!(voice.adsr.phase, ADSRPhase::Sustain);
    // Final block flag should be set
    assert!(voice.final_block);

    // Simulate consuming all 28 samples by advancing position past the block
    voice.adpcm_state.position = 28.0;
    voice.advance_position();

    // Now voice should be disabled
    assert!(!voice.enabled);
    assert_eq!(voice.adsr.phase, ADSRPhase::Off);
}

#[test]
fn test_voice_noise_mode() {
    let mut voice = Voice::new(0);
    let spu_ram = vec![0u8; 512 * 1024];
    let mut noise = NoiseGenerator::new();

    voice.enabled = true;
    voice.noise_enabled = true;
    voice.adsr.phase = ADSRPhase::Attack;
    voice.adsr.level = 32767;
    voice.volume_left = 16384;
    voice.volume_right = 16384;

    noise.set_frequency(0, 1); // Low frequency for testing

    let (left, right) = voice.render_sample(&spu_ram, &mut noise);

    // Should produce noise output (either max positive or max negative)
    // The noise generator outputs 0x7FFF or -0x8000
    assert!(left != 0 || right != 0);
}
