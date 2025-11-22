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

//! ADSR envelope tests - attack, decay, sustain, release phases

use crate::core::spu::adsr::{ADSREnvelope, ADSRPhase, AttackMode, ReleaseMode};

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
