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

mod adpcm;
mod adsr;
mod noise;
mod registers;
mod reverb;
mod voice;

#[cfg(test)]
mod tests;

use noise::NoiseGenerator;
use registers::{SPUControl, SPUStatus, TransferMode};
use reverb::ReverbConfig;
use std::collections::VecDeque;
use voice::Voice;

/// SPU (Sound Processing Unit)
///
/// The main SPU struct managing all audio processing including voice synthesis,
/// ADPCM decoding, envelope generation, and audio mixing.
pub struct SPU {
    /// Sound RAM (512KB)
    pub(crate) ram: Vec<u8>,

    /// 24 hardware voices
    pub(crate) voices: [Voice; 24],

    /// Main volume (left/right)
    pub(crate) main_volume_left: i16,
    pub(crate) main_volume_right: i16,

    /// Reverb volume
    reverb_volume_left: i16,
    reverb_volume_right: i16,

    /// CD audio volume
    pub(crate) cd_volume_left: i16,
    pub(crate) cd_volume_right: i16,

    /// External audio volume
    #[allow(dead_code)]
    ext_volume_left: i16,
    #[allow(dead_code)]
    ext_volume_right: i16,

    /// Reverb configuration
    reverb: ReverbConfig,

    /// Noise generator
    noise: NoiseGenerator,

    /// Control register
    pub(crate) control: SPUControl,

    /// Status register
    status: SPUStatus,

    /// Current sample position
    #[allow(dead_code)]
    sample_counter: u32,

    /// Capture buffers
    #[allow(dead_code)]
    capture_buffer: [i16; 2],

    /// DMA transfer address (in 8-byte units)
    transfer_addr: u32,

    /// DMA FIFO for buffered writes
    dma_fifo: VecDeque<u16>,
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
            noise: NoiseGenerator::new(),
            control: SPUControl::default(),
            status: SPUStatus::default(),
            sample_counter: 0,
            capture_buffer: [0; 2],
            transfer_addr: 0,
            dma_fifo: VecDeque::new(),
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

            // DMA Transfer Address (0x1F801DA6)
            // Returns address in 8-byte units
            0x1F801DA6 => (self.transfer_addr / 8) as u16,

            // DMA Data Register (0x1F801DA8) - write-only, reads return 0
            0x1F801DA8 => 0,

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

            // DMA Transfer Address (0x1F801DA6)
            // Address is in 8-byte units
            0x1F801DA6 => self.set_transfer_address(value as u32),

            // DMA Data Register (0x1F801DA8)
            // Manual write to SPU RAM, auto-increment address
            0x1F801DA8 => {
                self.write_ram_word(self.transfer_addr, value);
                self.transfer_addr = (self.transfer_addr + 2) & 0x7FFFE;
            }

            // Reverb registers (0x1F801DC0-0x1F801DFF)
            0x1F801DC0..=0x1F801DFF => self.write_reverb_register(addr, value),

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
        value |= (self.control.noise_step as u16) << 8;
        if self.control.reverb_enabled {
            value |= 1 << 7;
        }
        if self.control.irq_enabled {
            value |= 1 << 6;
        }
        value |= (self.control.transfer_mode as u16) << 4;
        if self.control.external_audio_reverb {
            value |= 1 << 3;
        }
        if self.control.cd_audio_reverb {
            value |= 1 << 2;
        }
        if self.control.external_audio_enabled {
            value |= 1 << 1;
        }
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
        // Bits 10-13: noise clock (shift)
        self.control.noise_clock = ((value >> 10) & 0xF) as u8;
        // Bits 8-9: noise frequency step
        self.control.noise_step = ((value >> 8) & 0x3) as u8;
        self.control.reverb_enabled = (value & (1 << 7)) != 0;
        self.control.irq_enabled = (value & (1 << 6)) != 0;
        // Bits 5-4: transfer mode
        self.control.transfer_mode = match (value >> 4) & 0x3 {
            1 => TransferMode::ManualWrite,
            2 => TransferMode::DMAWrite,
            3 => TransferMode::DMARead,
            _ => TransferMode::Stop,
        };
        // Bits 3-1: external/CD audio flags
        self.control.external_audio_reverb = (value & (1 << 3)) != 0;
        self.control.cd_audio_reverb = (value & (1 << 2)) != 0;
        self.control.external_audio_enabled = (value & (1 << 1)) != 0;
        self.control.cd_audio_enabled = (value & (1 << 0)) != 0;

        // Update noise generator frequency
        self.noise
            .set_frequency(self.control.noise_clock, self.control.noise_step);

        // Update reverb enabled state
        self.reverb.enabled = self.control.reverb_enabled;

        log::debug!(
            "SPU control: enabled={} unmute={}",
            self.control.enabled,
            self.control.unmute
        );
    }

    /// Write reverb configuration register
    ///
    /// # Arguments
    ///
    /// * `addr` - Register address
    /// * `value` - 16-bit value to write
    fn write_reverb_register(&mut self, addr: u32, value: u16) {
        match addr {
            0x1F801DC0 => self.reverb.apf_offset1 = value,
            0x1F801DC2 => self.reverb.apf_offset2 = value,
            0x1F801DC4 => self.reverb.reflect_volume1 = value as i16,
            0x1F801DC6 => self.reverb.comb_volume1 = value as i16,
            0x1F801DC8 => self.reverb.comb_volume2 = value as i16,
            0x1F801DCA => self.reverb.comb_volume3 = value as i16,
            0x1F801DCC => self.reverb.comb_volume4 = value as i16,
            0x1F801DCE => self.reverb.apf_volume1 = value as i16,
            0x1F801DD0 => self.reverb.apf_volume2 = value as i16,
            0x1F801DD2 => self.reverb.input_volume_left = value as i16,
            0x1F801DD4 => self.reverb.input_volume_right = value as i16,
            0x1F801DD6 => self.reverb.reflect_volume2 = value as i16,
            0x1F801DD8 => self.reverb.reflect_volume3 = value as i16,
            0x1F801DDA => self.reverb.reflect_volume4 = value as i16,
            // Reverb work area address
            0x1F801DDC => self.reverb.reverb_start_addr = (value as u32) * 8,
            0x1F801DDE => self.reverb.reverb_end_addr = (value as u32) * 8,
            _ => {
                log::debug!(
                    "Unknown reverb register write: 0x{:08X} = 0x{:04X}",
                    addr,
                    value
                );
            }
        }
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

    /// Tick SPU to generate audio samples
    ///
    /// Generates audio samples based on the number of CPU cycles elapsed.
    /// The SPU runs at 44.1 kHz while the CPU runs at ~33.8688 MHz.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles elapsed
    ///
    /// # Returns
    ///
    /// Vector of stereo samples (left, right) generated during this tick
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    ///
    /// let mut spu = SPU::new();
    /// let samples = spu.tick(100); // Generate samples for 100 CPU cycles
    /// ```
    pub fn tick(&mut self, cycles: u32) -> Vec<(i16, i16)> {
        // Check if SPU is enabled
        if !self.control.enabled {
            return Vec::new();
        }

        // Calculate number of samples to generate
        // CPU frequency: 33.8688 MHz (33_868_800 Hz)
        // SPU frequency: 44.1 kHz (44_100 Hz)
        // Ratio: 44100 / 33868800 ≈ 0.001302
        const CPU_FREQ: f32 = 33_868_800.0;
        const SPU_FREQ: f32 = 44_100.0;

        let samples_to_generate = (cycles as f32 * SPU_FREQ / CPU_FREQ) as usize;

        let mut output = Vec::with_capacity(samples_to_generate);

        for _ in 0..samples_to_generate {
            let sample = self.generate_sample();
            output.push(sample);
        }

        output
    }

    /// Tick SPU with CD audio mixing
    ///
    /// Generates audio samples with CD-DA audio mixed in.
    /// The SPU runs at 44.1 kHz while the CPU runs at ~33.8688 MHz.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles elapsed
    /// * `cd_audio` - CD audio player for mixing CD-DA
    ///
    /// # Returns
    ///
    /// Vector of stereo samples (left, right) generated during this tick
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    /// use psrx::core::cdrom::CDAudio;
    ///
    /// let mut spu = SPU::new();
    /// let mut cd_audio = CDAudio::new();
    /// let samples = spu.tick_with_cd(100, &mut cd_audio);
    /// ```
    pub fn tick_with_cd(
        &mut self,
        cycles: u32,
        cd_audio: &mut crate::core::cdrom::CDAudio,
    ) -> Vec<(i16, i16)> {
        // Check if SPU is enabled
        if !self.control.enabled {
            return Vec::new();
        }

        // Calculate number of samples to generate
        const CPU_FREQ: f32 = 33_868_800.0;
        const SPU_FREQ: f32 = 44_100.0;

        let samples_to_generate = (cycles as f32 * SPU_FREQ / CPU_FREQ) as usize;

        let mut output = Vec::with_capacity(samples_to_generate);

        for _ in 0..samples_to_generate {
            let sample = self.generate_sample_with_cd(cd_audio);
            output.push(sample);
        }

        output
    }

    /// Generate a single stereo sample
    ///
    /// Mixes all 24 voices, applies main volume, and processes reverb.
    ///
    /// # Returns
    ///
    /// Stereo sample (left, right)
    #[inline(always)]
    fn generate_sample(&mut self) -> (i16, i16) {
        // Use i64 to avoid overflow when mixing 24 voices at high volume
        let mut left: i64 = 0;
        let mut right: i64 = 0;

        // Mix all 24 voices
        for voice in &mut self.voices {
            let (v_left, v_right) = voice.render_sample(&self.ram, &mut self.noise);
            left += v_left as i64;
            right += v_right as i64;
        }

        // Apply main volume (fixed-point multiply with 15-bit fraction)
        left = (left * self.main_volume_left as i64) >> 15;
        right = (right * self.main_volume_right as i64) >> 15;

        // Clamp to i16 range
        left = left.clamp(i16::MIN as i64, i16::MAX as i64);
        right = right.clamp(i16::MIN as i64, i16::MAX as i64);

        // Apply reverb
        self.reverb
            .process(left as i16, right as i16, &mut self.ram)
    }

    /// Generate a single stereo sample with CD audio mixing
    ///
    /// Mixes all 24 voices, CD audio, applies main volume, and processes reverb.
    ///
    /// # Arguments
    ///
    /// * `cd_audio` - CD audio player for CD-DA mixing
    ///
    /// # Returns
    ///
    /// Stereo sample (left, right) with CD audio mixed in
    #[inline(always)]
    fn generate_sample_with_cd(
        &mut self,
        cd_audio: &mut crate::core::cdrom::CDAudio,
    ) -> (i16, i16) {
        // Use i64 to avoid overflow when mixing 24 voices at high volume
        let mut left: i64 = 0;
        let mut right: i64 = 0;

        // Mix all 24 voices
        for voice in &mut self.voices {
            let (v_left, v_right) = voice.render_sample(&self.ram, &mut self.noise);
            left += v_left as i64;
            right += v_right as i64;
        }

        // Apply main volume (fixed-point multiply with 15-bit fraction)
        left = (left * self.main_volume_left as i64) >> 15;
        right = (right * self.main_volume_right as i64) >> 15;

        // Mix CD audio if enabled
        if self.control.cd_audio_enabled {
            let (cd_left, cd_right) = cd_audio.get_sample();

            // Apply CD volume (fixed-point multiply with 15-bit fraction)
            let cd_left = (cd_left as i64 * self.cd_volume_left as i64) >> 15;
            let cd_right = (cd_right as i64 * self.cd_volume_right as i64) >> 15;

            left += cd_left;
            right += cd_right;
        }

        // Clamp to i16 range
        left = left.clamp(i16::MIN as i64, i16::MAX as i64);
        right = right.clamp(i16::MIN as i64, i16::MAX as i64);

        // Apply reverb
        self.reverb
            .process(left as i16, right as i16, &mut self.ram)
    }

    // DMA Interface Methods

    /// Set DMA transfer address
    ///
    /// The address is specified in 8-byte units and is automatically
    /// multiplied by 8 to get the actual byte address in SPU RAM.
    ///
    /// # Arguments
    ///
    /// * `addr` - Transfer address in 8-byte units
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    ///
    /// let mut spu = SPU::new();
    /// spu.set_transfer_address(0x1000); // Sets address to 0x8000 bytes
    /// ```
    pub fn set_transfer_address(&mut self, addr: u32) {
        // Address is in 8-byte units, mask to 19 bits (0-0x7FFFF)
        self.transfer_addr = (addr & 0x3FFFF) * 8;
        log::debug!("SPU DMA address: 0x{:08X}", self.transfer_addr);
    }

    /// Write to DMA FIFO
    ///
    /// Writes a 32-bit value to the DMA FIFO as two 16-bit words.
    /// When the FIFO reaches 16 entries, it is automatically flushed to SPU RAM.
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit value to write (split into two 16-bit words)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    ///
    /// let mut spu = SPU::new();
    /// spu.set_transfer_address(0x1000);
    /// spu.dma_write(0x12345678); // Writes 0x5678, then 0x1234
    /// ```
    pub fn dma_write(&mut self, value: u32) {
        let lo = (value & 0xFFFF) as u16;
        let hi = (value >> 16) as u16;

        self.dma_fifo.push_back(lo);
        self.dma_fifo.push_back(hi);

        // Flush to RAM if FIFO is full
        if self.dma_fifo.len() >= 16 {
            self.flush_dma_fifo();
        }
    }

    /// Read from DMA
    ///
    /// Reads two consecutive 16-bit words from SPU RAM starting at the
    /// current transfer address and returns them as a 32-bit value.
    /// The transfer address is automatically incremented after each read.
    ///
    /// # Returns
    ///
    /// 32-bit value composed of two 16-bit words from SPU RAM
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    ///
    /// let mut spu = SPU::new();
    /// spu.set_transfer_address(0x1000);
    /// let value = spu.dma_read();
    /// ```
    pub fn dma_read(&mut self) -> u32 {
        let lo = self.read_ram_word(self.transfer_addr);
        self.transfer_addr = (self.transfer_addr + 2) & 0x7FFFE;

        let hi = self.read_ram_word(self.transfer_addr);
        self.transfer_addr = (self.transfer_addr + 2) & 0x7FFFE;

        ((hi as u32) << 16) | (lo as u32)
    }

    /// Flush DMA FIFO to SPU RAM
    ///
    /// Writes all pending data in the DMA FIFO to SPU RAM,
    /// starting at the current transfer address.
    fn flush_dma_fifo(&mut self) {
        while let Some(value) = self.dma_fifo.pop_front() {
            self.write_ram_word(self.transfer_addr, value);
            self.transfer_addr = (self.transfer_addr + 2) & 0x7FFFE;
        }
    }

    /// Read 16-bit word from SPU RAM
    ///
    /// Reads a 16-bit word from SPU RAM in little-endian format.
    ///
    /// # Arguments
    ///
    /// * `addr` - Address in SPU RAM (masked to 19 bits)
    ///
    /// # Returns
    ///
    /// 16-bit value from SPU RAM
    #[inline(always)]
    fn read_ram_word(&self, addr: u32) -> u16 {
        let addr = (addr as usize) & 0x7FFFE;
        let lo = self.ram[addr] as u16;
        let hi = self.ram[addr + 1] as u16;
        (hi << 8) | lo
    }

    /// Write 16-bit word to SPU RAM
    ///
    /// Writes a 16-bit word to SPU RAM in little-endian format.
    ///
    /// # Arguments
    ///
    /// * `addr` - Address in SPU RAM (masked to 19 bits)
    /// * `value` - 16-bit value to write
    #[inline(always)]
    pub(crate) fn write_ram_word(&mut self, addr: u32, value: u16) {
        let addr = (addr as usize) & 0x7FFFE;
        self.ram[addr] = value as u8;
        self.ram[addr + 1] = (value >> 8) as u8;
    }

    /// Check if DMA is ready
    ///
    /// The SPU is always ready for DMA transfers.
    ///
    /// # Returns
    ///
    /// Always returns `true`
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::SPU;
    ///
    /// let spu = SPU::new();
    /// assert!(spu.dma_ready());
    /// ```
    pub fn dma_ready(&self) -> bool {
        // SPU is always ready for DMA
        true
    }
}

impl Default for SPU {
    fn default() -> Self {
        Self::new()
    }
}
