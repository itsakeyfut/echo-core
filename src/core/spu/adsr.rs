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

//! ADSR (Attack, Decay, Sustain, Release) envelope generator
//!
//! Controls the volume envelope for each voice over time.
//! The envelope has four phases:
//! - Attack: Volume rises from 0 to maximum
//! - Decay: Volume falls from maximum to sustain level
//! - Sustain: Volume holds at sustain level
//! - Release: Volume falls from current level to 0

/// ADSR (Attack, Decay, Sustain, Release) envelope generator
///
/// Controls the volume envelope for each voice over time.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ADSREnvelope {
    pub attack_rate: u8,
    pub attack_mode: AttackMode,
    pub decay_rate: u8,
    pub sustain_level: u8,
    pub sustain_rate: u8,
    pub sustain_mode: SustainMode,
    pub release_rate: u8,
    pub release_mode: ReleaseMode,

    /// Current ADSR phase
    pub phase: ADSRPhase,

    /// Current envelope level (0-32767)
    pub level: i16,
}

/// ADSR envelope phase
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ADSRPhase {
    /// Attack phase: volume rising
    Attack,
    /// Decay phase: volume falling to sustain level
    Decay,
    /// Sustain phase: volume held at sustain level
    Sustain,
    /// Release phase: volume falling to zero
    Release,
    /// Off: voice is silent
    Off,
}

/// Attack mode (linear or exponential)
#[derive(Debug, Clone, Copy)]
pub enum AttackMode {
    Linear,
    Exponential,
}

/// Sustain mode (linear or exponential)
#[derive(Debug, Clone, Copy)]
pub enum SustainMode {
    Linear,
    Exponential,
}

/// Release mode (linear or exponential)
#[derive(Debug, Clone, Copy)]
pub enum ReleaseMode {
    Linear,
    Exponential,
}

#[allow(dead_code)]
impl ADSREnvelope {
    /// Advance the ADSR envelope by one sample
    ///
    /// Updates the current level based on the current phase and configured rates.
    /// Called once per audio sample (44100 Hz).
    pub fn tick(&mut self) {
        match self.phase {
            ADSRPhase::Attack => self.tick_attack(),
            ADSRPhase::Decay => self.tick_decay(),
            ADSRPhase::Sustain => self.tick_sustain(),
            ADSRPhase::Release => self.tick_release(),
            ADSRPhase::Off => {}
        }
    }

    /// Process attack phase
    fn tick_attack(&mut self) {
        let rate = self.attack_rate_to_step();

        match self.attack_mode {
            AttackMode::Linear => {
                self.level = self.level.saturating_add(rate);
            }
            AttackMode::Exponential => {
                // Exponential: increase rate scales with distance from max.
                // When we're very close to max, the computed step can round to 0,
                // which would otherwise leave us stuck in the attack phase.
                let step = ((rate as i32 * (32767 - self.level as i32)) >> 15) as i16;
                if step > 0 {
                    self.level = self.level.saturating_add(step);
                } else {
                    self.level = 32767;
                }
            }
        }

        if self.level == 32767 {
            self.phase = ADSRPhase::Decay;
        }
    }

    /// Process decay phase
    fn tick_decay(&mut self) {
        let rate = self.decay_rate_to_step();
        let sustain_level = ((self.sustain_level as i32 + 1) << 11).min(32767) as i16;

        // Decay is always exponential in hardware
        let step = ((rate as i32 * self.level as i32) >> 15) as i16;
        self.level = self.level.saturating_sub(step);

        if self.level <= sustain_level {
            self.level = sustain_level;
            self.phase = ADSRPhase::Sustain;
        }
    }

    /// Process sustain phase
    fn tick_sustain(&mut self) {
        let rate = self.sustain_rate_to_step();

        match self.sustain_mode {
            SustainMode::Linear => {
                self.level = self.level.saturating_sub(rate);
            }
            SustainMode::Exponential => {
                let step = ((rate as i32 * self.level as i32) >> 15) as i16;
                self.level = self.level.saturating_sub(step);
            }
        }

        if self.level <= 0 {
            self.level = 0;
            self.phase = ADSRPhase::Off;
        }
    }

    /// Process release phase
    fn tick_release(&mut self) {
        let rate = self.release_rate_to_step();

        match self.release_mode {
            ReleaseMode::Linear => {
                self.level = self.level.saturating_sub(rate);
            }
            ReleaseMode::Exponential => {
                let step = ((rate as i32 * self.level as i32) >> 15) as i16;
                self.level = self.level.saturating_sub(step);
            }
        }

        if self.level <= 0 {
            self.level = 0;
            self.phase = ADSRPhase::Off;
        }
    }

    /// Convert attack rate to step value
    ///
    /// # Returns
    ///
    /// Step value to add per sample during attack phase
    fn attack_rate_to_step(&self) -> i16 {
        // PSX attack rate formula: simplified approximation
        // Real hardware uses complex cycle counters
        if self.attack_rate == 0 {
            return 0;
        }

        // Higher rate = faster attack
        // Rate 127 should reach max in ~1ms (44 samples)
        // Rate 0 = infinite attack
        let rate = self.attack_rate as i32;
        ((32767 * rate) / (128 * 50)) as i16
    }

    /// Convert decay rate to step value
    ///
    /// # Returns
    ///
    /// Step value for decay phase
    fn decay_rate_to_step(&self) -> i16 {
        if self.decay_rate == 0 {
            return 0;
        }

        let rate = self.decay_rate as i32;
        ((32767 * rate) / (16 * 200)) as i16
    }

    /// Convert sustain rate to step value
    ///
    /// # Returns
    ///
    /// Step value for sustain phase
    fn sustain_rate_to_step(&self) -> i16 {
        if self.sustain_rate == 0 {
            return 0;
        }

        let rate = self.sustain_rate as i32;
        ((32767 * rate) / (128 * 200)) as i16
    }

    /// Convert release rate to step value
    ///
    /// # Returns
    ///
    /// Step value for release phase
    fn release_rate_to_step(&self) -> i16 {
        if self.release_rate == 0 {
            return 0;
        }

        let rate = self.release_rate as i32;
        ((32767 * rate) / (32 * 200)) as i16
    }

    /// Convert ADSR configuration to register format (word 1)
    ///
    /// # Returns
    ///
    /// Lower 16 bits of ADSR configuration
    ///
    /// # Format
    ///
    /// ```text
    /// Bits  0-3:  Sustain Level
    /// Bit   4:    Decay Rate (bit 0)
    /// Bits  5-7:  Decay Rate (bits 1-3)
    /// Bits  8-14: Attack Rate
    /// Bit   15:   Attack Mode (0=Linear, 1=Exponential)
    /// ```
    pub fn to_word_1(&self) -> u16 {
        let mut value = 0u16;

        value |= (self.sustain_level as u16) & 0xF;
        value |= ((self.decay_rate as u16) & 0xF) << 4;
        value |= ((self.attack_rate as u16) & 0x7F) << 8;
        value |= if matches!(self.attack_mode, AttackMode::Exponential) {
            1 << 15
        } else {
            0
        };

        value
    }

    /// Convert ADSR configuration to register format (word 2)
    ///
    /// # Returns
    ///
    /// Upper 16 bits of ADSR configuration
    ///
    /// # Format
    ///
    /// ```text
    /// Bits  0-4:  Release Rate
    /// Bit   5:    Release Mode (0=Linear, 1=Exponential)
    /// Bits  6-12: Sustain Rate
    /// Bit   13:   (unused, always 0)
    /// Bit   14:   Sustain Direction (0=Increase, 1=Decrease)
    /// Bit   15:   Sustain Mode (0=Linear, 1=Exponential)
    /// ```
    pub fn to_word_2(&self) -> u16 {
        let mut value = 0u16;

        value |= (self.release_rate as u16) & 0x1F;
        value |= if matches!(self.release_mode, ReleaseMode::Exponential) {
            1 << 5
        } else {
            0
        };
        value |= ((self.sustain_rate as u16) & 0x7F) << 6;
        value |= if matches!(self.sustain_mode, SustainMode::Exponential) {
            1 << 15
        } else {
            0
        };

        value
    }

    /// Load ADSR configuration from register format (word 1)
    ///
    /// # Arguments
    ///
    /// * `value` - Lower 16 bits of ADSR configuration
    pub fn set_word_1(&mut self, value: u16) {
        self.sustain_level = (value & 0xF) as u8;
        self.decay_rate = ((value >> 4) & 0xF) as u8;
        self.attack_rate = ((value >> 8) & 0x7F) as u8;
        self.attack_mode = if (value & (1 << 15)) != 0 {
            AttackMode::Exponential
        } else {
            AttackMode::Linear
        };
    }

    /// Load ADSR configuration from register format (word 2)
    ///
    /// # Arguments
    ///
    /// * `value` - Upper 16 bits of ADSR configuration
    pub fn set_word_2(&mut self, value: u16) {
        self.release_rate = (value & 0x1F) as u8;
        self.release_mode = if (value & (1 << 5)) != 0 {
            ReleaseMode::Exponential
        } else {
            ReleaseMode::Linear
        };
        self.sustain_rate = ((value >> 6) & 0x7F) as u8;
        self.sustain_mode = if (value & (1 << 15)) != 0 {
            SustainMode::Exponential
        } else {
            SustainMode::Linear
        };
    }
}

impl Default for ADSREnvelope {
    fn default() -> Self {
        Self {
            attack_rate: 0,
            attack_mode: AttackMode::Linear,
            decay_rate: 0,
            sustain_level: 0,
            sustain_rate: 0,
            sustain_mode: SustainMode::Linear,
            release_rate: 0,
            release_mode: ReleaseMode::Linear,
            phase: ADSRPhase::Off,
            level: 0,
        }
    }
}
