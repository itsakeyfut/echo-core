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

//! Unit tests for SPU module

use super::adpcm::ADPCMState;
use super::adsr::{ADSREnvelope, ADSRPhase, AttackMode, ReleaseMode};
use super::noise::NoiseGenerator;
use super::registers::TransferMode;
use super::reverb::ReverbConfig;
use super::voice::Voice;
use super::SPU;

#[test]
fn test_spu_initialization() {
    let spu = SPU::new();
    assert_eq!(spu.ram.len(), 512 * 1024);
    assert_eq!(spu.voices.len(), 24);
}

#[test]
fn test_voice_key_on() {
    let mut spu = SPU::new();
    spu.key_on_voices(0x00000001); // Key on voice 0

    assert!(spu.voices[0].enabled);
    assert_eq!(spu.voices[0].adsr.phase, ADSRPhase::Attack);
}

#[test]
fn test_spu_ram_access() {
    let mut spu = SPU::new();

    spu.write_ram(0x1000, 0xAB);
    assert_eq!(spu.read_ram(0x1000), 0xAB);
}

#[test]
fn test_register_mapping() {
    let mut spu = SPU::new();

    // Write main volume
    spu.write_register(0x1F801D80, 0x3FFF);
    assert_eq!(spu.main_volume_left, 0x3FFF);

    // Read back
    assert_eq!(spu.read_register(0x1F801D80), 0x3FFF);
}

#[test]
fn test_voice_registers() {
    let mut spu = SPU::new();

    // Voice 0 volume left (0x1F801C00)
    spu.write_register(0x1F801C00, 0x1234);
    assert_eq!(spu.voices[0].volume_left, 0x1234);
    assert_eq!(spu.read_register(0x1F801C00), 0x1234);

    // Voice 0 sample rate (0x1F801C04)
    spu.write_register(0x1F801C04, 0x1000);
    assert_eq!(spu.voices[0].sample_rate, 0x1000);

    // Voice 1 volume right (0x1F801C12)
    spu.write_register(0x1F801C12, 0x5678);
    assert_eq!(spu.voices[1].volume_right, 0x5678);
}

#[test]
fn test_key_on_multiple_voices() {
    let mut spu = SPU::new();

    // Key on voices 0, 1, and 15 (bits 0, 1, 15)
    spu.write_register(0x1F801D88, 0x8003);

    assert!(spu.voices[0].enabled);
    assert!(spu.voices[1].enabled);
    assert!(!spu.voices[2].enabled);
    assert!(spu.voices[15].enabled);
}

#[test]
fn test_key_on_upper_voices() {
    let mut spu = SPU::new();

    // Key on voices 16-23 (upper register, bits 0-7)
    spu.write_register(0x1F801D8A, 0x00FF);

    for i in 16..24 {
        assert!(spu.voices[i].enabled);
    }
    for i in 0..16 {
        assert!(!spu.voices[i].enabled);
    }
}

#[test]
fn test_control_register() {
    let mut spu = SPU::new();

    // Enable SPU, unmute, enable reverb
    spu.write_register(0x1F801DAA, 0xC080);

    assert!(spu.control.enabled);
    assert!(spu.control.unmute);
    assert!(spu.control.reverb_enabled);

    // Read back
    let control = spu.read_register(0x1F801DAA);
    assert_eq!(control & 0xC080, 0xC080);
}

#[test]
fn test_control_register_round_trip() {
    let mut spu = SPU::new();

    // Test transfer modes and audio flags round-trip
    // Bits: enabled(15), unmute(14), reverb(7), irq(6),
    //       DMAWrite mode(5-4=10b), ext_reverb(3), cd_reverb(2), ext_en(1), cd_en(0)
    let test_value = 0xC0E0 | (2 << 4) | 0x0F; // DMAWrite + all audio flags
    spu.write_register(0x1F801DAA, test_value);

    assert!(spu.control.enabled);
    assert!(spu.control.unmute);
    assert!(spu.control.reverb_enabled);
    assert!(spu.control.irq_enabled);
    assert!(matches!(spu.control.transfer_mode, TransferMode::DMAWrite));
    assert!(spu.control.external_audio_reverb);
    assert!(spu.control.cd_audio_reverb);
    assert!(spu.control.external_audio_enabled);
    assert!(spu.control.cd_audio_enabled);

    // Read back and verify exact match
    let read_back = spu.read_register(0x1F801DAA);
    assert_eq!(read_back, test_value);
}

#[test]
fn test_adsr_encoding() {
    let adsr = ADSREnvelope {
        attack_rate: 0x7F,
        attack_mode: AttackMode::Exponential,
        decay_rate: 0x0F,
        sustain_level: 0x0D,
        ..Default::default()
    };

    let word1 = adsr.to_word_1();
    assert_eq!(word1 & 0xF, 0x0D); // Sustain level
    assert_eq!((word1 >> 4) & 0xF, 0x0F); // Decay rate
    assert_eq!((word1 >> 8) & 0x7F, 0x7F); // Attack rate
    assert_eq!(word1 & 0x8000, 0x8000); // Exponential mode
}

#[test]
fn test_adsr_decoding() {
    let mut adsr = ADSREnvelope::default();

    // Set ADSR word 1: sustain_level=5, decay=7, attack=64, exponential
    adsr.set_word_1(0xFF75);

    assert_eq!(adsr.sustain_level, 0x5);
    assert_eq!(adsr.decay_rate, 0x7);
    assert_eq!(adsr.attack_rate, 0x7F);
    assert!(matches!(adsr.attack_mode, AttackMode::Exponential));
}

#[test]
fn test_ram_wrapping() {
    let mut spu = SPU::new();

    // Write beyond RAM size (should wrap)
    spu.write_ram(0x80000, 0xCD); // Wraps to 0x0
    assert_eq!(spu.read_ram(0x0), 0xCD);

    spu.write_ram(0x80001, 0xEF); // Wraps to 0x1
    assert_eq!(spu.read_ram(0x1), 0xEF);
}

#[test]
fn test_adpcm_decode_filter_0() {
    let mut state = ADPCMState::default();

    // Create a test block with filter 0 (no filtering)
    let mut block = vec![0u8; 16];
    block[0] = 0x00; // Shift=0, Filter=0
    block[1] = 0x00; // No flags

    // Set some test nibbles
    block[2] = 0x12; // Nibbles: 2, 1
    block[3] = 0x34; // Nibbles: 4, 3

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // With shift=0 and filter=0, samples should be nibbles << 12
    assert_eq!(samples[0], 0x2000); // nibble 2 << 12
    assert_eq!(samples[1], 0x1000); // nibble 1 << 12
}

#[test]
fn test_adpcm_decode_with_shift() {
    let mut state = ADPCMState::default();

    // Create a test block with shift
    let mut block = vec![0u8; 16];
    block[0] = 0x04; // Shift=4, Filter=0
    block[1] = 0x00;
    block[2] = 0xFF; // Nibbles: F (-1), F (-1)

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // Nibble F = -1 (sign extended)
    // (-1 << 12) >> 4 = -4096 >> 4 = -256
    assert_eq!(samples[0], -256);
    assert_eq!(samples[1], -256);
}

#[test]
fn test_adpcm_decode_filter_1() {
    let mut state = ADPCMState::default();

    // Set up previous samples
    state.prev_samples[0] = 100;
    state.prev_samples[1] = 50;

    // Create a test block with filter 1
    let mut block = vec![0u8; 16];
    block[0] = 0x10; // Shift=0, Filter=1
    block[1] = 0x00;
    block[2] = 0x00; // Nibbles: 0, 0

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // Filter 1: sample + old[0] + (-old[0] >> 1)
    // 0 + 100 + (-100 >> 1) = 0 + 100 - 50 = 50
    assert_eq!(samples[0], 50);
}

#[test]
fn test_adpcm_empty_block() {
    let mut state = ADPCMState::default();

    // Try to decode empty block
    let block: Vec<u8> = Vec::new();
    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 0);

    // Try with too-short block
    let block = vec![0u8; 10];
    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 0);
}

#[test]
fn test_adsr_attack_linear() {
    let mut adsr = ADSREnvelope {
        attack_rate: 0x7F,
        attack_mode: AttackMode::Linear,
        phase: ADSRPhase::Attack,
        level: 0,
        ..Default::default()
    };

    // Tick several times
    for _ in 0..100 {
        adsr.tick();
    }

    // Level should have increased
    assert!(adsr.level > 0);
}

#[test]
fn test_adsr_attack_exponential() {
    let mut adsr = ADSREnvelope {
        attack_rate: 0x7F,
        attack_mode: AttackMode::Exponential,
        phase: ADSRPhase::Attack,
        level: 0,
        ..Default::default()
    };

    // Tick several times
    for _ in 0..100 {
        adsr.tick();
    }

    // Level should have increased
    assert!(adsr.level > 0);
}

#[test]
fn test_adsr_attack_to_decay() {
    let mut adsr = ADSREnvelope {
        attack_rate: 0x7F,
        attack_mode: AttackMode::Linear,
        decay_rate: 0x0F,
        sustain_level: 0x08,
        phase: ADSRPhase::Attack,
        level: 0,
        ..Default::default()
    };

    // Tick until we reach max level
    for _ in 0..10000 {
        adsr.tick();
        if adsr.phase == ADSRPhase::Decay {
            break;
        }
    }

    // Should transition to decay phase
    assert_eq!(adsr.phase, ADSRPhase::Decay);
    assert_eq!(adsr.level, 32767);
}

#[test]
fn test_adsr_decay_to_sustain() {
    let mut adsr = ADSREnvelope {
        decay_rate: 0x0F,
        sustain_level: 0x08, // ~50% level
        phase: ADSRPhase::Decay,
        level: 32767,
        ..Default::default()
    };

    // Tick until we reach sustain level
    for _ in 0..10000 {
        adsr.tick();
        if adsr.phase == ADSRPhase::Sustain {
            break;
        }
    }

    // Should transition to sustain phase
    assert_eq!(adsr.phase, ADSRPhase::Sustain);
    // Level should be at sustain level
    let expected_sustain = ((0x08 + 1) << 11) as i16;
    assert_eq!(adsr.level, expected_sustain);
}

#[test]
fn test_adsr_release() {
    let mut adsr = ADSREnvelope {
        release_rate: 0x1F,
        release_mode: ReleaseMode::Linear,
        phase: ADSRPhase::Release,
        level: 16000,
        ..Default::default()
    };

    // Tick several times
    for _ in 0..100 {
        adsr.tick();
    }

    // Level should have decreased
    assert!(adsr.level < 16000);
}

#[test]
fn test_adsr_release_to_off() {
    let mut adsr = ADSREnvelope {
        release_rate: 0x1F,
        release_mode: ReleaseMode::Linear,
        phase: ADSRPhase::Release,
        level: 100,
        ..Default::default()
    };

    // Tick until off
    for _ in 0..10000 {
        adsr.tick();
        if adsr.phase == ADSRPhase::Off {
            break;
        }
    }

    assert_eq!(adsr.phase, ADSRPhase::Off);
    assert_eq!(adsr.level, 0);
}

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

// Audio Output Integration Tests

#[test]
fn test_spu_tick_disabled() {
    let mut spu = SPU::new();
    // SPU is disabled by default
    assert!(!spu.control.enabled);

    let samples = spu.tick(100);
    assert_eq!(samples.len(), 0, "Disabled SPU should generate no samples");
}

#[test]
fn test_spu_tick_enabled() {
    let mut spu = SPU::new();

    // Enable SPU
    spu.control.enabled = true;
    spu.control.unmute = true;

    // Tick for 100 CPU cycles
    let samples = spu.tick(100);

    // Calculate expected number of samples
    // CPU: 33.8688 MHz, SPU: 44.1 kHz
    // samples = cycles * (44100 / 33868800) ≈ cycles * 0.001302
    // For 100 cycles: ~0 samples (too few cycles)
    assert!(samples.len() <= 1);
}

#[test]
fn test_spu_tick_sample_generation() {
    let mut spu = SPU::new();

    // Enable SPU
    spu.control.enabled = true;
    spu.control.unmute = true;

    // Set main volume
    spu.main_volume_left = 0x3FFF;
    spu.main_volume_right = 0x3FFF;

    // Tick for one frame worth of cycles (564,480 cycles)
    let samples = spu.tick(564_480);

    // At 44.1 kHz, one frame (1/60 second) should generate:
    // 44100 / 60 = 735 samples
    assert!(
        samples.len() >= 730 && samples.len() <= 740,
        "Expected ~735 samples, got {}",
        samples.len()
    );

    // All samples should be (0, 0) since no voices are playing
    for (left, right) in &samples {
        assert_eq!(*left, 0);
        assert_eq!(*right, 0);
    }
}

#[test]
fn test_spu_tick_with_volume() {
    let mut spu = SPU::new();

    // Enable SPU
    spu.control.enabled = true;
    spu.control.unmute = true;

    // Set main volume to 50%
    spu.main_volume_left = 0x4000; // 0x4000 / 0x8000 = 0.5
    spu.main_volume_right = 0x4000;

    // Generate samples (no voices active, so output should still be silence)
    let samples = spu.tick(10000);

    assert!(!samples.is_empty());

    // All samples should be (0, 0) since no voices are playing
    for (left, right) in &samples {
        assert_eq!(*left, 0);
        assert_eq!(*right, 0);
    }
}

#[test]
fn test_spu_generate_sample_mixing() {
    let mut spu = SPU::new();

    // Enable SPU
    spu.control.enabled = true;
    spu.control.unmute = true;

    // Set main volume to max
    spu.main_volume_left = 0x7FFF;
    spu.main_volume_right = 0x7FFF;

    // Generate a single sample (no voices, should be silence)
    let sample = spu.generate_sample();
    assert_eq!(sample, (0, 0));
}

#[test]
fn test_spu_tick_accurate_sample_count() {
    let mut spu = SPU::new();
    spu.control.enabled = true;

    // Test various cycle counts
    let test_cases = vec![
        (1000, 0),      // Very small, might round to 0
        (10000, 0),     // Still small
        (100000, 2),    // Should generate ~2-3 samples
        (564_480, 735), // One frame: ~735 samples
    ];

    for (cycles, expected_min) in test_cases {
        let samples = spu.tick(cycles);
        assert!(
            samples.len() >= expected_min,
            "For {} cycles, expected at least {} samples, got {}",
            cycles,
            expected_min,
            samples.len()
        );
    }
}

// Noise Generator Tests

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

// Reverb Tests

#[test]
fn test_reverb_creation() {
    let reverb = ReverbConfig::new();
    assert!(!reverb.enabled);
    assert_eq!(reverb.reverb_current_addr, 0);
}

#[test]
fn test_reverb_disabled_passthrough() {
    let mut reverb = ReverbConfig::new();
    reverb.enabled = false;

    let mut spu_ram = vec![0u8; 512 * 1024];
    let (left, right) = reverb.process(1000, 1000, &mut spu_ram);

    // When disabled, reverb should pass through unchanged
    assert_eq!(left, 1000);
    assert_eq!(right, 1000);
}

#[test]
fn test_reverb_enabled_processing() {
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
fn test_reverb_register_writes() {
    let mut spu = SPU::new();

    // Write reverb configuration
    spu.write_register(0x1F801DC0, 0x1234); // APF offset 1
    spu.write_register(0x1F801DC2, 0x5678); // APF offset 2
    spu.write_register(0x1F801DD2, 0x4000); // Input volume left
    spu.write_register(0x1F801DD4, 0x3000); // Input volume right

    assert_eq!(spu.reverb.apf_offset1, 0x1234);
    assert_eq!(spu.reverb.apf_offset2, 0x5678);
    assert_eq!(spu.reverb.input_volume_left, 0x4000);
    assert_eq!(spu.reverb.input_volume_right, 0x3000);
}

#[test]
fn test_spu_with_reverb_enabled() {
    let mut spu = SPU::new();

    // Enable SPU and reverb
    spu.control.enabled = true;
    spu.control.unmute = true;
    spu.write_register(0x1F801DAA, 0xC080); // Enable + unmute + reverb

    assert!(spu.control.reverb_enabled);
    assert!(spu.reverb.enabled);

    // Generate a sample
    let _sample = spu.generate_sample();

    // Should generate without crashing
    // Output values are i16, so they're always in valid range
}

#[test]
fn test_noise_clock_configuration() {
    let mut spu = SPU::new();

    // Write control with noise clock settings (bits 8-13)
    // Bits 10-13: shift=5, Bits 8-9: step=2
    spu.write_register(0x1F801DAA, 0x1600);

    assert_eq!(spu.control.noise_clock, 5);
}

// DMA Tests

#[test]
fn test_spu_dma_write() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write data via DMA
    for _ in 0..100 {
        spu.dma_write(0x12345678);
    }

    spu.flush_dma_fifo();

    // Verify data was written
    assert_eq!(spu.read_ram_word(0x1000 * 8), 0x5678);
    assert_eq!(spu.read_ram_word(0x1000 * 8 + 2), 0x1234);
}

#[test]
fn test_spu_dma_read() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write test data
    spu.write_ram_word(0x1000 * 8, 0xABCD);
    spu.write_ram_word(0x1000 * 8 + 2, 0x1234);

    // Read via DMA
    let value = spu.dma_read();
    assert_eq!(value, 0x1234ABCD);
}

#[test]
fn test_spu_dma_transfer_address() {
    let mut spu = SPU::new();

    // Set transfer address (in 8-byte units)
    spu.set_transfer_address(0x2000);

    // Verify address was set correctly (multiplied by 8)
    assert_eq!(spu.transfer_addr, 0x2000 * 8);
}

#[test]
fn test_spu_dma_ready() {
    let spu = SPU::new();
    // SPU should always be ready for DMA
    assert!(spu.dma_ready());
}

#[test]
fn test_spu_dma_fifo_flush() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write enough data to trigger auto-flush (16 words)
    for _ in 0..8 {
        spu.dma_write(0xAABBCCDD);
    }

    // FIFO should have been flushed automatically
    assert_eq!(spu.dma_fifo.len(), 0);

    // Verify data was written to RAM
    let first_word = spu.read_ram_word(0x1000 * 8);
    assert_eq!(first_word, 0xCCDD);
}

#[test]
fn test_spu_dma_address_wrapping() {
    let mut spu = SPU::new();

    // Set address near end of SPU RAM
    spu.set_transfer_address(0xFFFE);

    // Write data that would go past the end
    spu.dma_write(0x11223344);
    spu.dma_write(0x55667788);
    spu.flush_dma_fifo();

    // Address should have wrapped around
    // After 4 writes (8 bytes), addr should be (0xFFFE * 8 + 8) & 0x7FFFE
    assert!(spu.transfer_addr < 0x80000);
}

#[test]
fn test_spu_dma_register_read_write() {
    let mut spu = SPU::new();

    // Write transfer address via register
    spu.write_register(0x1F801DA6, 0x5000);
    assert_eq!(spu.transfer_addr, 0x5000 * 8);

    // Read transfer address via register (should return value in 8-byte units)
    let read_value = spu.read_register(0x1F801DA6);
    assert_eq!(read_value, 0x5000);
}

#[test]
fn test_spu_dma_manual_write() {
    let mut spu = SPU::new();

    // Set transfer address
    spu.write_register(0x1F801DA6, 0x1000);

    // Write data manually via register
    spu.write_register(0x1F801DA8, 0xABCD);
    spu.write_register(0x1F801DA8, 0x1234);

    // Verify data was written and address auto-incremented
    assert_eq!(spu.read_ram_word(0x1000 * 8), 0xABCD);
    assert_eq!(spu.read_ram_word(0x1000 * 8 + 2), 0x1234);
}

#[test]
fn test_spu_read_ram_word() {
    let mut spu = SPU::new();

    // Write bytes directly to RAM
    spu.write_ram(0x1000, 0xCD);
    spu.write_ram(0x1001, 0xAB);

    // Read as word (little-endian)
    let word = spu.read_ram_word(0x1000);
    assert_eq!(word, 0xABCD);
}

#[test]
fn test_spu_write_ram_word() {
    let mut spu = SPU::new();

    // Write word (little-endian)
    spu.write_ram_word(0x1000, 0xABCD);

    // Read bytes
    assert_eq!(spu.read_ram(0x1000), 0xCD);
    assert_eq!(spu.read_ram(0x1001), 0xAB);
}
