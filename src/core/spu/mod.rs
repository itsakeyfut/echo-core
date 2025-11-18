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

//! SPU (Sound Processing Unit) implementation
//!
//! The SPU handles all audio processing for the PlayStation, including:
//! - 24 independent hardware voices with ADPCM decoding
//! - 512KB of Sound RAM for storing audio samples
//! - ADSR envelope generation for each voice
//! - Hardware reverb effects
//! - CD audio and external audio mixing
//!
//! # Memory Map
//!
//! | Address Range          | Register               | Access |
//! |------------------------|------------------------|--------|
//! | 0x1F801C00-0x1F801D7F  | Voice registers (24x)  | R/W    |
//! | 0x1F801D80-0x1F801D83  | Main volume L/R        | R/W    |
//! | 0x1F801D84-0x1F801D87  | Reverb volume L/R      | R/W    |
//! | 0x1F801D88-0x1F801D8F  | Voice key on/off       | W      |
//! | 0x1F801DAA             | Control register       | R/W    |
//! | 0x1F801DAE             | Status register        | R      |
//!
//! # Voice Registers (per voice, 16 bytes each)
//!
//! | Offset | Register        | Description                    |
//! |--------|-----------------|--------------------------------|
//! | +0x0   | Volume Left     | Left channel volume            |
//! | +0x2   | Volume Right    | Right channel volume           |
//! | +0x4   | Sample Rate     | Pitch/sample rate              |
//! | +0x6   | Start Address   | Start address in SPU RAM       |
//! | +0x8   | ADSR (low)      | Attack/Decay/Sustain/Release   |
//! | +0xA   | ADSR (high)     | ADSR configuration             |
//! | +0xC   | ADSR Volume     | Current envelope level         |
//! | +0xE   | Repeat Address  | Loop point address             |

/// SPU (Sound Processing Unit)
///
/// The main SPU struct managing all audio processing including voice synthesis,
/// ADPCM decoding, envelope generation, and audio mixing.
pub struct SPU {
    /// Sound RAM (512KB)
    ram: Vec<u8>,

    /// 24 hardware voices
    voices: [Voice; 24],

    /// Main volume (left/right)
    main_volume_left: i16,
    main_volume_right: i16,

    /// Reverb volume
    reverb_volume_left: i16,
    reverb_volume_right: i16,

    /// CD audio volume
    #[allow(dead_code)]
    cd_volume_left: i16,
    #[allow(dead_code)]
    cd_volume_right: i16,

    /// External audio volume
    #[allow(dead_code)]
    ext_volume_left: i16,
    #[allow(dead_code)]
    ext_volume_right: i16,

    /// Reverb configuration
    #[allow(dead_code)]
    reverb: ReverbConfig,

    /// Control register
    control: SPUControl,

    /// Status register
    status: SPUStatus,

    /// Current sample position
    #[allow(dead_code)]
    sample_counter: u32,

    /// Capture buffers
    #[allow(dead_code)]
    capture_buffer: [i16; 2],
}

/// Individual voice channel
///
/// Each voice can play back ADPCM-compressed audio samples with
/// independent volume, pitch, and ADSR envelope control.
pub struct Voice {
    /// Voice number (0-23)
    id: u8,

    /// Current volume (left/right)
    volume_left: i16,
    volume_right: i16,

    /// ADSR state
    adsr: ADSREnvelope,

    /// Current sample rate (pitch)
    sample_rate: u16,

    /// Start address in SPU RAM (multiply by 8 for byte address)
    start_address: u16,

    /// Repeat address (loop point, multiply by 8 for byte address)
    repeat_address: u16,

    /// Current address
    current_address: u32,

    /// ADPCM decoder state
    #[allow(dead_code)]
    adpcm_state: ADPCMState,

    /// Voice enabled
    enabled: bool,

    /// Key on flag
    key_on: bool,

    /// Key off flag
    key_off: bool,
}

/// ADSR (Attack, Decay, Sustain, Release) envelope generator
///
/// Controls the volume envelope for each voice over time.
/// The envelope has four phases:
/// - Attack: Volume rises from 0 to maximum
/// - Decay: Volume falls from maximum to sustain level
/// - Sustain: Volume holds at sustain level
/// - Release: Volume falls from current level to 0
#[derive(Debug, Clone)]
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

/// SPU control register
pub struct SPUControl {
    pub enabled: bool,
    pub unmute: bool,
    pub noise_clock: u8,
    pub reverb_enabled: bool,
    pub irq_enabled: bool,
    pub transfer_mode: TransferMode,
    pub external_audio_reverb: bool,
    pub cd_audio_reverb: bool,
    pub external_audio_enabled: bool,
    pub cd_audio_enabled: bool,
}

/// SPU data transfer mode
#[derive(Debug, Clone, Copy)]
pub enum TransferMode {
    Stop,
    ManualWrite,
    DMAWrite,
    DMARead,
}

/// SPU status register
#[derive(Default)]
pub struct SPUStatus {
    pub mode: u16,
    pub irq_flag: bool,
    pub dma_request: bool,
    pub dma_busy: bool,
    pub capture_ready: bool,
}

/// Reverb configuration
///
/// Hardware reverb effects configuration (placeholder for future implementation)
#[derive(Default)]
pub struct ReverbConfig {
    // TODO: Implement reverb parameters in Phase 4
}

/// ADPCM decoder state
///
/// Maintains state for ADPCM audio decompression (placeholder for future implementation)
#[derive(Default)]
pub struct ADPCMState {
    // TODO: Implement ADPCM decoder state in Phase 4
}

impl SPU {
    /// SPU RAM size (512KB)
    const RAM_SIZE: usize = 512 * 1024;

    /// Create a new SPU instance
    ///
    /// # Returns
    ///
    /// Initialized SPU with 512KB RAM and 24 voices
    pub fn new() -> Self {
        Self {
            ram: vec![0; Self::RAM_SIZE],
            voices: std::array::from_fn(|i| Voice::new(i as u8)),
            main_volume_left: 0,
            main_volume_right: 0,
            reverb_volume_left: 0,
            reverb_volume_right: 0,
            cd_volume_left: 0,
            cd_volume_right: 0,
            ext_volume_left: 0,
            ext_volume_right: 0,
            reverb: ReverbConfig::default(),
            control: SPUControl::default(),
            status: SPUStatus::default(),
            sample_counter: 0,
            capture_buffer: [0; 2],
        }
    }

    /// Read from SPU register
    ///
    /// # Arguments
    ///
    /// * `addr` - Physical address of the register (0x1F801C00-0x1F801FFF)
    ///
    /// # Returns
    ///
    /// 16-bit register value
    pub fn read_register(&self, addr: u32) -> u16 {
        match addr {
            // Voice registers (0x1F801C00-0x1F801D7F)
            // Each voice has 16 bytes (0x10) of registers
            0x1F801C00..=0x1F801D7F => {
                let voice_id = ((addr - 0x1F801C00) / 0x10) as usize;
                let reg = ((addr - 0x1F801C00) % 0x10) as u8;
                self.read_voice_register(voice_id, reg)
            }

            // Main volume
            0x1F801D80 => self.main_volume_left as u16,
            0x1F801D82 => self.main_volume_right as u16,

            // Reverb volume
            0x1F801D84 => self.reverb_volume_left as u16,
            0x1F801D86 => self.reverb_volume_right as u16,

            // Voice key on/off (write-only, read returns 0)
            0x1F801D88 => 0, // VOICE_KEY_ON (lower)
            0x1F801D8A => 0, // VOICE_KEY_ON (upper)
            0x1F801D8C => 0, // VOICE_KEY_OFF (lower)
            0x1F801D8E => 0, // VOICE_KEY_OFF (upper)

            // Control/Status
            0x1F801DAA => self.read_control(),
            0x1F801DAE => self.read_status(),

            _ => {
                log::warn!("SPU read from unknown register: 0x{:08X}", addr);
                0
            }
        }
    }

    /// Write to SPU register
    ///
    /// # Arguments
    ///
    /// * `addr` - Physical address of the register (0x1F801C00-0x1F801FFF)
    /// * `value` - 16-bit value to write
    pub fn write_register(&mut self, addr: u32, value: u16) {
        match addr {
            // Voice registers (0x1F801C00-0x1F801D7F)
            0x1F801C00..=0x1F801D7F => {
                let voice_id = ((addr - 0x1F801C00) / 0x10) as usize;
                let reg = ((addr - 0x1F801C00) % 0x10) as u8;
                self.write_voice_register(voice_id, reg, value);
            }

            // Main volume
            0x1F801D80 => self.main_volume_left = value as i16,
            0x1F801D82 => self.main_volume_right = value as i16,

            // Reverb volume
            0x1F801D84 => self.reverb_volume_left = value as i16,
            0x1F801D86 => self.reverb_volume_right = value as i16,

            // Voice key on (lower 16 voices, bits 0-15)
            0x1F801D88 => self.key_on_voices(value as u32),
            // Voice key on (upper 8 voices, bits 16-23)
            0x1F801D8A => self.key_on_voices((value as u32) << 16),

            // Voice key off (lower 16 voices, bits 0-15)
            0x1F801D8C => self.key_off_voices(value as u32),
            // Voice key off (upper 8 voices, bits 16-23)
            0x1F801D8E => self.key_off_voices((value as u32) << 16),

            // Control
            0x1F801DAA => self.write_control(value),

            _ => {
                log::warn!(
                    "SPU write to unknown register: 0x{:08X} = 0x{:04X}",
                    addr,
                    value
                );
            }
        }
    }

    /// Read from a voice register
    ///
    /// # Arguments
    ///
    /// * `voice_id` - Voice number (0-23)
    /// * `reg` - Register offset within voice (0-15)
    ///
    /// # Returns
    ///
    /// 16-bit register value
    fn read_voice_register(&self, voice_id: usize, reg: u8) -> u16 {
        if voice_id >= 24 {
            return 0;
        }

        let voice = &self.voices[voice_id];

        match reg {
            0x0 => voice.volume_left as u16,
            0x2 => voice.volume_right as u16,
            0x4 => voice.sample_rate,
            0x6 => voice.start_address,
            0x8 => voice.adsr.to_word_1(),
            0xA => voice.adsr.to_word_2(),
            0xC => voice.adsr.level as u16,
            0xE => voice.repeat_address,
            _ => 0,
        }
    }

    /// Write to a voice register
    ///
    /// # Arguments
    ///
    /// * `voice_id` - Voice number (0-23)
    /// * `reg` - Register offset within voice (0-15)
    /// * `value` - 16-bit value to write
    fn write_voice_register(&mut self, voice_id: usize, reg: u8, value: u16) {
        if voice_id >= 24 {
            return;
        }

        let voice = &mut self.voices[voice_id];

        match reg {
            0x0 => voice.volume_left = value as i16,
            0x2 => voice.volume_right = value as i16,
            0x4 => voice.sample_rate = value,
            0x6 => voice.start_address = value,
            0x8 => voice.adsr.set_word_1(value),
            0xA => voice.adsr.set_word_2(value),
            0xE => voice.repeat_address = value,
            _ => {}
        }
    }

    /// Trigger key-on for voices specified by bitmask
    ///
    /// # Arguments
    ///
    /// * `mask` - 24-bit mask where each bit represents a voice (bit 0 = voice 0, etc.)
    fn key_on_voices(&mut self, mask: u32) {
        for i in 0..24 {
            if (mask & (1 << i)) != 0 {
                self.voices[i].key_on();
            }
        }
    }

    /// Trigger key-off for voices specified by bitmask
    ///
    /// # Arguments
    ///
    /// * `mask` - 24-bit mask where each bit represents a voice
    fn key_off_voices(&mut self, mask: u32) {
        for i in 0..24 {
            if (mask & (1 << i)) != 0 {
                self.voices[i].key_off();
            }
        }
    }

    /// Read SPU control register
    ///
    /// # Returns
    ///
    /// 16-bit control register value
    fn read_control(&self) -> u16 {
        let mut value = 0u16;

        if self.control.enabled {
            value |= 1 << 15;
        }
        if self.control.unmute {
            value |= 1 << 14;
        }
        value |= (self.control.noise_clock as u16) << 10;
        if self.control.reverb_enabled {
            value |= 1 << 7;
        }
        if self.control.irq_enabled {
            value |= 1 << 6;
        }
        value |= (self.control.transfer_mode as u16) << 4;
        if self.control.cd_audio_enabled {
            value |= 1 << 0;
        }

        value
    }

    /// Write SPU control register
    ///
    /// # Arguments
    ///
    /// * `value` - 16-bit control register value
    fn write_control(&mut self, value: u16) {
        self.control.enabled = (value & (1 << 15)) != 0;
        self.control.unmute = (value & (1 << 14)) != 0;
        self.control.noise_clock = ((value >> 10) & 0xF) as u8;
        self.control.reverb_enabled = (value & (1 << 7)) != 0;
        self.control.irq_enabled = (value & (1 << 6)) != 0;
        self.control.cd_audio_enabled = (value & (1 << 0)) != 0;

        log::debug!(
            "SPU control: enabled={} unmute={}",
            self.control.enabled,
            self.control.unmute
        );
    }

    /// Read SPU status register
    ///
    /// # Returns
    ///
    /// 16-bit status register value
    fn read_status(&self) -> u16 {
        let mut value = 0u16;

        if self.status.irq_flag {
            value |= 1 << 6;
        }
        if self.status.dma_busy {
            value |= 1 << 10;
        }

        value
    }

    /// Read from SPU RAM
    ///
    /// # Arguments
    ///
    /// * `addr` - Address in SPU RAM (0-0x7FFFF)
    ///
    /// # Returns
    ///
    /// Byte value from SPU RAM
    pub fn read_ram(&self, addr: u32) -> u8 {
        let addr = (addr as usize) & (Self::RAM_SIZE - 1);
        self.ram[addr]
    }

    /// Write to SPU RAM
    ///
    /// # Arguments
    ///
    /// * `addr` - Address in SPU RAM (0-0x7FFFF)
    /// * `value` - Byte value to write
    pub fn write_ram(&mut self, addr: u32, value: u8) {
        let addr = (addr as usize) & (Self::RAM_SIZE - 1);
        self.ram[addr] = value;
    }
}

impl Voice {
    /// Create a new voice instance
    ///
    /// # Arguments
    ///
    /// * `id` - Voice number (0-23)
    ///
    /// # Returns
    ///
    /// Initialized voice
    pub fn new(id: u8) -> Self {
        Self {
            id,
            volume_left: 0,
            volume_right: 0,
            adsr: ADSREnvelope::default(),
            sample_rate: 0,
            start_address: 0,
            repeat_address: 0,
            current_address: 0,
            adpcm_state: ADPCMState::default(),
            enabled: false,
            key_on: false,
            key_off: false,
        }
    }

    /// Trigger key-on for this voice
    ///
    /// Starts playback from the start address and begins the attack phase
    /// of the ADSR envelope.
    pub fn key_on(&mut self) {
        self.enabled = true;
        self.key_on = true;
        self.current_address = (self.start_address as u32) * 8;
        self.adsr.phase = ADSRPhase::Attack;
        self.adsr.level = 0;

        log::trace!("Voice {} key on", self.id);
    }

    /// Trigger key-off for this voice
    ///
    /// Begins the release phase of the ADSR envelope.
    pub fn key_off(&mut self) {
        self.key_off = true;
        self.adsr.phase = ADSRPhase::Release;

        log::trace!("Voice {} key off", self.id);
    }
}

impl ADSREnvelope {
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

impl Default for SPUControl {
    fn default() -> Self {
        Self {
            enabled: false,
            unmute: false,
            noise_clock: 0,
            reverb_enabled: false,
            irq_enabled: false,
            transfer_mode: TransferMode::Stop,
            external_audio_reverb: false,
            cd_audio_reverb: false,
            external_audio_enabled: false,
            cd_audio_enabled: false,
        }
    }
}

impl Default for SPU {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
