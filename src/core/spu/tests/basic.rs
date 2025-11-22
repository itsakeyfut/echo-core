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

//! Basic SPU functionality tests - initialization, reset, and registers

use crate::core::spu::adsr::ADSRPhase;
use crate::core::spu::registers::TransferMode;
use crate::core::spu::SPU;

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
fn test_ram_wrapping() {
    let mut spu = SPU::new();

    // Write beyond RAM size (should wrap)
    spu.write_ram(0x80000, 0xCD); // Wraps to 0x0
    assert_eq!(spu.read_ram(0x0), 0xCD);

    spu.write_ram(0x80001, 0xEF); // Wraps to 0x1
    assert_eq!(spu.read_ram(0x1), 0xEF);
}

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

#[test]
fn test_noise_clock_configuration() {
    let mut spu = SPU::new();

    // Write control with noise clock settings (bits 8-13)
    // Bits 10-13: shift=5, Bits 8-9: step=2
    spu.write_register(0x1F801DAA, 0x1600);

    assert_eq!(spu.control.noise_clock, 5);
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
