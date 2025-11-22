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

//! Audio integration tests

#[cfg(feature = "audio")]
use super::super::*;

#[test]
#[cfg(feature = "audio")]
fn test_audio_backend_optional() {
    let system = System::new();
    // Audio backend may or may not be initialized depending on system capabilities
    // This test just ensures the system can be created regardless
    assert_eq!(system.cycles(), 0);
}

#[test]
#[cfg(feature = "audio")]
fn test_spu_audio_integration_via_step() {
    let mut system = System::new();

    // Skip test if no audio backend available
    if system.audio.is_none() {
        return;
    }

    // Create an infinite loop in BIOS
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Enable SPU via control register write
    system.bus.write16(0x1F801DAA, 0x8000).unwrap(); // Enable bit

    // Note: Individual step() calls with 1 cycle each won't generate samples
    // because SPU::tick(1) returns 0 samples (truncates to 0).
    // This test verifies the integration doesn't crash.
    // For actual sample generation, see test_run_frame_generates_audio.
    for _ in 0..100 {
        system.step().unwrap();
    }

    // Verify system continues to work with audio enabled
    // (actual sample generation requires larger cycle batches)
    assert!(system.cycles() >= 100);
}

#[test]
#[cfg(feature = "audio")]
fn test_run_frame_generates_audio() {
    let mut system = System::new();

    // Skip test if no audio backend available
    if system.audio.is_none() {
        return;
    }

    // Create an infinite loop in BIOS
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Enable SPU
    system.bus.write16(0x1F801DAA, 0x8000).unwrap();

    // Run one frame - should generate ~735 samples at 44.1 kHz
    system.run_frame().unwrap();

    // Verify audio samples were generated and queued
    if let Some(ref audio) = system.audio {
        let buffer_level = audio.buffer_level();
        // One frame should generate approximately 735 samples
        assert!(
            (730..=740).contains(&buffer_level),
            "Expected ~735 samples per frame, got {}",
            buffer_level
        );
    }
}
