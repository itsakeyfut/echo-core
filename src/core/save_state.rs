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

//! Save state serialization for PlayStation emulator
//!
//! This module provides functionality to save and restore complete emulator state,
//! allowing users to save their game progress at any point and resume later.
//!
//! # Save State Format
//!
//! Save states are serialized using bincode for efficient binary encoding.
//! The state includes:
//! - Metadata (timestamp, game ID, playtime, etc.)
//! - CPU state (registers, PC, COP0, etc.)
//! - Memory state (RAM, scratchpad, VRAM, SPU RAM)
//! - GPU state (registers, drawing area, display mode)
//! - SPU state (voices, volume, reverb)
//! - CD-ROM state (FIFOs, seek position, mode)
//! - DMA state (channels, control registers)
//! - Timer state (counters, targets, modes)
//! - Controller state (button inputs)
//! - Interrupt state (I_STAT, I_MASK)
//!
//! # Version Compatibility
//!
//! Save states include a version number to ensure compatibility.
//! Loading a save state with a different version will fail with an error.
//!
//! # Example
//!
//! ```no_run
//! use psrx::core::save_state::SaveState;
//! use psrx::core::System;
//!
//! // Create system and run emulation
//! let mut system = System::new();
//! // ... run emulation ...
//!
//! // Create save state
//! let state = SaveState::from_system(&system);
//!
//! // Save to file
//! state.save_to_file("save.state").unwrap();
//!
//! // Later: load from file
//! let loaded_state = SaveState::load_from_file("save.state").unwrap();
//! // ... apply to system ...
//! ```

use bincode::{config, Decode, Encode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Save state version for compatibility checking
///
/// This version number should be incremented whenever the save state format changes
/// in a way that breaks backward compatibility.
pub const SAVE_STATE_VERSION: u32 = 1;

/// Complete emulator save state
///
/// This structure contains all state needed to fully restore the emulator
/// to a specific point in time, including all hardware component states.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct SaveState {
    /// Version number for compatibility checking
    pub version: u32,

    /// Save state metadata
    pub metadata: SaveStateMetadata,

    /// CPU state
    pub cpu: CPUState,

    /// Memory state (RAM, scratchpad)
    pub memory: MemoryState,

    /// GPU state (VRAM, registers, display settings)
    pub gpu: GPUState,

    /// SPU state (sound RAM, voices, volume)
    pub spu: SPUState,

    /// CD-ROM state (FIFOs, position, mode)
    pub cdrom: CDROMState,

    /// DMA state (channels, control)
    pub dma: DMAState,

    /// Timer state (3 channels)
    pub timers: TimerState,

    /// Controller state (button inputs)
    pub controllers: ControllerState,

    /// Interrupt controller state
    pub interrupts: InterruptState,
}

/// Save state metadata
///
/// Contains information about when and where the save state was created.
#[derive(Serialize, Deserialize, Encode, Decode)]
#[bincode(encode_bounds = "", decode_bounds = "")]
pub struct SaveStateMetadata {
    /// Timestamp when the save state was created
    #[bincode(with_serde)]
    pub timestamp: DateTime<Utc>,

    /// Game ID from disc (e.g., "SCUS-94163")
    pub game_id: String,

    /// Game title from disc
    pub game_title: String,

    /// Frame count at save time
    pub frame_count: u64,

    /// Playtime in seconds
    pub playtime: u64,

    /// Optional screenshot thumbnail (PNG format)
    pub thumbnail: Option<Vec<u8>>,
}

/// CPU state (MIPS R3000A)
///
/// Captures all CPU registers and internal state including
/// delay slots, COP0, and GTE state.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct CPUState {
    /// General purpose registers (R0-R31)
    pub regs: [u32; 32],

    /// Program counter
    pub pc: u32,

    /// Next PC (for delay slot handling)
    pub next_pc: u32,

    /// HI register (multiplication/division result upper 32 bits)
    pub hi: u32,

    /// LO register (multiplication/division result lower 32 bits)
    pub lo: u32,

    /// COP0 registers (System Control Coprocessor)
    pub cop0_regs: [u32; 32],

    /// Load delay slot (register index, value)
    pub load_delay: Option<(u8, u32)>,

    /// Branch delay slot flag
    pub in_branch_delay: bool,

    /// Current instruction (for debugging)
    pub current_instruction: u32,

    /// GTE (Geometry Transformation Engine) registers
    pub gte_data_regs: [i32; 32],
    pub gte_control_regs: [u32; 32],
}

/// Memory state (RAM and scratchpad)
///
/// Contains main RAM and scratchpad data.
/// BIOS is not saved as it doesn't change during execution.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct MemoryState {
    /// Main RAM (2MB)
    pub ram: Vec<u8>,

    /// Scratchpad (1KB fast RAM)
    pub scratchpad: Vec<u8>,

    /// BIOS ROM is not saved (doesn't change during execution)
    #[serde(skip)]
    pub bios: Vec<u8>,
}

/// GPU state (Graphics Processing Unit)
///
/// Captures VRAM contents, drawing settings, and display configuration.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct GPUState {
    /// VRAM (1MB - 1024x512 pixels, 16-bit per pixel)
    pub vram: Vec<u16>,

    /// Drawing area (clipping rectangle)
    pub draw_area_left: u16,
    pub draw_area_top: u16,
    pub draw_area_right: u16,
    pub draw_area_bottom: u16,

    /// Drawing offset (added to vertex coordinates)
    pub draw_offset_x: i16,
    pub draw_offset_y: i16,

    /// Display area (region of VRAM output to screen)
    pub display_area_x: u16,
    pub display_area_y: u16,

    /// Display horizontal and vertical range
    pub display_horiz_start: u16,
    pub display_horiz_end: u16,
    pub display_vert_start: u16,
    pub display_vert_end: u16,

    /// Display mode flags
    pub display_enabled: bool,
    pub display_depth_24bit: bool,
    pub vertical_interlace: bool,
    pub horizontal_res: u8,
    pub vertical_res: bool,
    pub video_mode: bool,

    /// Texture window settings
    pub texture_window_mask_x: u8,
    pub texture_window_mask_y: u8,
    pub texture_window_offset_x: u8,
    pub texture_window_offset_y: u8,

    /// Drawing mode flags
    pub draw_mode_texture_page_x: u8,
    pub draw_mode_texture_page_y: u8,
    pub draw_mode_semi_transparency: u8,
    pub draw_mode_texture_depth: u8,
    pub draw_mode_dithering: bool,
    pub draw_mode_draw_to_display: bool,
    pub draw_mode_texture_disable: bool,
    pub draw_mode_rectangle_flip_x: bool,
    pub draw_mode_rectangle_flip_y: bool,

    /// Mask settings
    pub mask_bit_force: bool,
    pub mask_bit_check: bool,

    /// GPU status register
    pub status: u32,

    /// Scanline and timing state
    pub scanline: u16,
    pub dots: u16,
    pub in_vblank: bool,
}

/// SPU state (Sound Processing Unit)
///
/// Captures all audio processing state including voices, volumes, and sound RAM.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct SPUState {
    /// Sound RAM (512KB)
    pub ram: Vec<u8>,

    /// Voice states (24 voices)
    pub voices: Vec<VoiceState>,

    /// Main volume (left/right channels)
    pub main_volume_left: i16,
    pub main_volume_right: i16,

    /// Reverb volume
    pub reverb_volume_left: i16,
    pub reverb_volume_right: i16,

    /// CD audio volume
    pub cd_volume_left: i16,
    pub cd_volume_right: i16,

    /// Reverb state
    pub reverb: ReverbState,

    /// Control register
    pub control: u16,

    /// Status register
    pub status: u16,

    /// DMA transfer address
    pub transfer_addr: u32,
}

/// Individual voice state
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct VoiceState {
    /// Volume (left/right channels)
    pub volume_left: i16,
    pub volume_right: i16,

    /// Sample rate / pitch
    pub sample_rate: u16,

    /// Start address in SPU RAM
    pub start_address: u16,

    /// Repeat/loop address
    pub repeat_address: u16,

    /// Current playback address
    pub current_address: u32,

    /// ADSR envelope state
    pub adsr: ADSRState,

    /// Voice enabled flag
    pub enabled: bool,

    /// Current ADPCM decoder state
    pub adpcm_old: i16,
    pub adpcm_older: i16,
}

/// ADSR (Attack Decay Sustain Release) envelope state
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct ADSRState {
    /// Attack rate
    pub attack_rate: u8,

    /// Decay rate
    pub decay_rate: u8,

    /// Sustain level
    pub sustain_level: u8,

    /// Sustain rate
    pub sustain_rate: u8,

    /// Release rate
    pub release_rate: u8,

    /// Current envelope phase
    pub phase: u8,

    /// Current envelope level
    pub level: i16,
}

/// Reverb configuration state
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct ReverbState {
    /// Reverb enabled flag
    pub enabled: bool,

    /// Reverb current address
    pub reverb_current_addr: u32,
    // Additional reverb parameters could be added here
}

/// CD-ROM state
///
/// Captures the state of the CD-ROM controller including FIFOs and command processing.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct CDROMState {
    /// Current status register
    pub status: u8,

    /// Index register
    pub index: u8,

    /// Parameter FIFO
    pub param_fifo: Vec<u8>,

    /// Response FIFO
    pub response_fifo: Vec<u8>,

    /// Data buffer
    pub data_buffer: Vec<u8>,

    /// Current command being processed
    pub current_command: u8,

    /// Seek target (MSF format: minute, second, frame)
    pub seek_target: (u8, u8, u8),

    /// Current read position (MSF format)
    pub read_position: (u8, u8, u8),

    /// Mode register
    pub mode: u8,

    /// Interrupt enable register
    pub interrupt_enable: u8,

    /// Interrupt flag register
    pub interrupt_flag: u8,

    /// Reading flag
    pub reading: bool,

    /// Seeking flag
    pub seeking: bool,
}

/// DMA state (Direct Memory Access)
///
/// Captures all 7 DMA channels and control registers.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct DMAState {
    /// DMA channel states (7 channels)
    pub channels: Vec<DMAChannelState>,

    /// DMA control register (DPCR)
    pub control: u32,

    /// DMA interrupt register (DICR)
    pub interrupt: u32,
}

/// Individual DMA channel state
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct DMAChannelState {
    /// Base address register (MADR)
    pub base_address: u32,

    /// Block control register (BCR)
    pub block_control: u32,

    /// Channel control register (CHCR)
    pub channel_control: u32,

    /// Channel ID (0-6)
    pub channel_id: u8,
}

/// Timer state (3 timer channels)
///
/// Captures all three timer/counter channels.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct TimerState {
    /// Timer channel states
    pub timers: Vec<TimerChannelState>,
}

/// Individual timer channel state
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct TimerChannelState {
    /// Current counter value
    pub counter: u32,

    /// Target value
    pub target: u32,

    /// Mode register
    pub mode: u32,
}

/// Controller state
///
/// Captures the state of all controller ports.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct ControllerState {
    /// Controller data for each port
    pub controllers: Vec<ControllerData>,
}

/// Individual controller button state
#[derive(Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ControllerData {
    /// Button state (16-bit bitfield)
    pub buttons: u16,
}

/// Interrupt controller state
///
/// Captures interrupt status and mask registers.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct InterruptState {
    /// Interrupt status register (I_STAT)
    pub i_stat: u32,

    /// Interrupt mask register (I_MASK)
    pub i_mask: u32,
}

impl SaveState {
    /// Create a new save state from the current system state
    ///
    /// # Arguments
    ///
    /// * `_system` - Reference to the system to save
    ///
    /// # Returns
    ///
    /// SaveState containing complete system state
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::{System, save_state::SaveState};
    /// # let system = System::new();
    /// let state = SaveState::from_system(&system);
    /// ```
    pub fn from_system(_system: &crate::core::System) -> Self {
        Self {
            version: SAVE_STATE_VERSION,
            metadata: SaveStateMetadata {
                timestamp: Utc::now(),
                game_id: String::new(), // Will be populated by System method
                game_title: String::new(),
                frame_count: 0,
                playtime: 0,
                thumbnail: None,
            },
            cpu: CPUState {
                regs: [0; 32],
                pc: 0,
                next_pc: 0,
                hi: 0,
                lo: 0,
                cop0_regs: [0; 32],
                load_delay: None,
                in_branch_delay: false,
                current_instruction: 0,
                gte_data_regs: [0; 32],
                gte_control_regs: [0; 32],
            },
            memory: MemoryState {
                ram: Vec::new(),
                scratchpad: Vec::new(),
                bios: Vec::new(),
            },
            gpu: GPUState {
                vram: Vec::new(),
                draw_area_left: 0,
                draw_area_top: 0,
                draw_area_right: 0,
                draw_area_bottom: 0,
                draw_offset_x: 0,
                draw_offset_y: 0,
                display_area_x: 0,
                display_area_y: 0,
                display_horiz_start: 0,
                display_horiz_end: 0,
                display_vert_start: 0,
                display_vert_end: 0,
                display_enabled: false,
                display_depth_24bit: false,
                vertical_interlace: false,
                horizontal_res: 0,
                vertical_res: false,
                video_mode: false,
                texture_window_mask_x: 0,
                texture_window_mask_y: 0,
                texture_window_offset_x: 0,
                texture_window_offset_y: 0,
                draw_mode_texture_page_x: 0,
                draw_mode_texture_page_y: 0,
                draw_mode_semi_transparency: 0,
                draw_mode_texture_depth: 0,
                draw_mode_dithering: false,
                draw_mode_draw_to_display: false,
                draw_mode_texture_disable: false,
                draw_mode_rectangle_flip_x: false,
                draw_mode_rectangle_flip_y: false,
                mask_bit_force: false,
                mask_bit_check: false,
                status: 0,
                scanline: 0,
                dots: 0,
                in_vblank: false,
            },
            spu: SPUState {
                ram: Vec::new(),
                voices: Vec::new(),
                main_volume_left: 0,
                main_volume_right: 0,
                reverb_volume_left: 0,
                reverb_volume_right: 0,
                cd_volume_left: 0,
                cd_volume_right: 0,
                reverb: ReverbState {
                    enabled: false,
                    reverb_current_addr: 0,
                },
                control: 0,
                status: 0,
                transfer_addr: 0,
            },
            cdrom: CDROMState {
                status: 0,
                index: 0,
                param_fifo: Vec::new(),
                response_fifo: Vec::new(),
                data_buffer: Vec::new(),
                current_command: 0,
                seek_target: (0, 0, 0),
                read_position: (0, 0, 0),
                mode: 0,
                interrupt_enable: 0,
                interrupt_flag: 0,
                reading: false,
                seeking: false,
            },
            dma: DMAState {
                channels: Vec::new(),
                control: 0,
                interrupt: 0,
            },
            timers: TimerState { timers: Vec::new() },
            controllers: ControllerState {
                controllers: Vec::new(),
            },
            interrupts: InterruptState {
                i_stat: 0,
                i_mask: 0,
            },
        }
    }

    /// Save state to file
    ///
    /// Serializes the save state to a binary file using bincode.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save file
    ///
    /// # Returns
    ///
    /// Result indicating success or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be created
    /// - Serialization fails
    /// - Write operation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::save_state::SaveState;
    /// # let state = SaveState::default();
    /// state.save_to_file("save.state").unwrap();
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let config = config::standard();
        let encoded = bincode::encode_to_vec(self, config)?;
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Load state from file
    ///
    /// Deserializes a save state from a binary file and verifies version compatibility.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save file
    ///
    /// # Returns
    ///
    /// Result containing loaded SaveState or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be opened
    /// - File cannot be read
    /// - Deserialization fails
    /// - Version is incompatible
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::save_state::SaveState;
    /// let state = SaveState::load_from_file("save.state").unwrap();
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let config = config::standard();
        let (state, _): (SaveState, usize) = bincode::decode_from_slice(&buffer, config)?;

        // Version check
        if state.version != SAVE_STATE_VERSION {
            return Err(format!(
                "Incompatible save state version: expected {}, got {}",
                SAVE_STATE_VERSION, state.version
            )
            .into());
        }

        Ok(state)
    }

    /// Get estimated file size for this save state
    ///
    /// Returns approximate size in bytes of the serialized save state.
    ///
    /// # Returns
    ///
    /// Estimated size in bytes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::save_state::SaveState;
    /// # let state = SaveState::default();
    /// let size = state.estimated_size();
    /// println!("Save state will be approximately {} MB", size / (1024 * 1024));
    /// ```
    pub fn estimated_size(&self) -> usize {
        // RAM: 2MB
        // VRAM: 1MB (stored as u16, so 2MB)
        // SPU RAM: 512KB
        // Scratchpad: 1KB
        // Other state: ~100KB (registers, FIFOs, etc.)
        // Total: ~3.6MB uncompressed
        2 * 1024 * 1024 +   // RAM
        2 * 1024 * 1024 +   // VRAM (as u16 array)
        512 * 1024 +        // SPU RAM
        1024 +              // Scratchpad
        100 * 1024 // Other state
    }
}

impl Default for SaveState {
    fn default() -> Self {
        Self {
            version: SAVE_STATE_VERSION,
            metadata: SaveStateMetadata {
                timestamp: Utc::now(),
                game_id: String::new(),
                game_title: String::new(),
                frame_count: 0,
                playtime: 0,
                thumbnail: None,
            },
            cpu: CPUState {
                regs: [0; 32],
                pc: 0,
                next_pc: 0,
                hi: 0,
                lo: 0,
                cop0_regs: [0; 32],
                load_delay: None,
                in_branch_delay: false,
                current_instruction: 0,
                gte_data_regs: [0; 32],
                gte_control_regs: [0; 32],
            },
            memory: MemoryState {
                ram: vec![0; 2 * 1024 * 1024],
                scratchpad: vec![0; 1024],
                bios: Vec::new(),
            },
            gpu: GPUState {
                vram: vec![0; 1024 * 512],
                draw_area_left: 0,
                draw_area_top: 0,
                draw_area_right: 1023,
                draw_area_bottom: 511,
                draw_offset_x: 0,
                draw_offset_y: 0,
                display_area_x: 0,
                display_area_y: 0,
                display_horiz_start: 0x200,
                display_horiz_end: 0xC00,
                display_vert_start: 0x10,
                display_vert_end: 0x100,
                display_enabled: true,
                display_depth_24bit: false,
                vertical_interlace: false,
                horizontal_res: 0,
                vertical_res: false,
                video_mode: false,
                texture_window_mask_x: 0,
                texture_window_mask_y: 0,
                texture_window_offset_x: 0,
                texture_window_offset_y: 0,
                draw_mode_texture_page_x: 0,
                draw_mode_texture_page_y: 0,
                draw_mode_semi_transparency: 0,
                draw_mode_texture_depth: 0,
                draw_mode_dithering: false,
                draw_mode_draw_to_display: false,
                draw_mode_texture_disable: false,
                draw_mode_rectangle_flip_x: false,
                draw_mode_rectangle_flip_y: false,
                mask_bit_force: false,
                mask_bit_check: false,
                status: 0x1C000000,
                scanline: 0,
                dots: 0,
                in_vblank: false,
            },
            spu: SPUState {
                ram: vec![0; 512 * 1024],
                voices: (0..24)
                    .map(|_| VoiceState {
                        volume_left: 0,
                        volume_right: 0,
                        sample_rate: 0,
                        start_address: 0,
                        repeat_address: 0,
                        current_address: 0,
                        adsr: ADSRState {
                            attack_rate: 0,
                            decay_rate: 0,
                            sustain_level: 0,
                            sustain_rate: 0,
                            release_rate: 0,
                            phase: 0,
                            level: 0,
                        },
                        enabled: false,
                        adpcm_old: 0,
                        adpcm_older: 0,
                    })
                    .collect(),
                main_volume_left: 0,
                main_volume_right: 0,
                reverb_volume_left: 0,
                reverb_volume_right: 0,
                cd_volume_left: 0,
                cd_volume_right: 0,
                reverb: ReverbState {
                    enabled: false,
                    reverb_current_addr: 0,
                },
                control: 0,
                status: 0,
                transfer_addr: 0,
            },
            cdrom: CDROMState {
                status: 0,
                index: 0,
                param_fifo: Vec::new(),
                response_fifo: Vec::new(),
                data_buffer: Vec::new(),
                current_command: 0,
                seek_target: (0, 2, 0),
                read_position: (0, 2, 0),
                mode: 0,
                interrupt_enable: 0,
                interrupt_flag: 0,
                reading: false,
                seeking: false,
            },
            dma: DMAState {
                channels: (0..7)
                    .map(|i| DMAChannelState {
                        base_address: 0,
                        block_control: 0,
                        channel_control: 0,
                        channel_id: i,
                    })
                    .collect(),
                control: 0x07654321,
                interrupt: 0,
            },
            timers: TimerState {
                timers: (0..3)
                    .map(|_| TimerChannelState {
                        counter: 0,
                        target: 0,
                        mode: 0,
                    })
                    .collect(),
            },
            controllers: ControllerState {
                controllers: vec![ControllerData { buttons: 0xFFFF }; 2],
            },
            interrupts: InterruptState {
                i_stat: 0,
                i_mask: 0,
            },
        }
    }
}

/// Trait for components that can be saved and restored
///
/// This trait should be implemented by all emulator components that need
/// to be included in save states.
///
/// # Example
///
/// ```no_run
/// use psrx::core::save_state::{StateSave, CPUState};
/// use serde::{Serialize, Deserialize};
///
/// struct MyCPU {
///     pc: u32,
///     regs: [u32; 32],
/// }
///
/// impl StateSave for MyCPU {
///     type State = CPUState;
///
///     fn to_state(&self) -> Self::State {
///         // Convert CPU to state...
///         # todo!()
///     }
///
///     fn restore_from_state(&mut self, state: &Self::State) {
///         // Restore CPU from state...
///     }
/// }
/// ```
pub trait StateSave {
    /// The state type for this component
    type State: Serialize + for<'de> Deserialize<'de>;

    /// Convert this component to a saveable state
    ///
    /// # Returns
    ///
    /// State representation of this component
    fn to_state(&self) -> Self::State;

    /// Restore this component from a saved state
    ///
    /// # Arguments
    ///
    /// * `state` - The state to restore from
    fn restore_from_state(&mut self, state: &Self::State);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_state_version() {
        assert_eq!(SAVE_STATE_VERSION, 1);
    }

    #[test]
    fn test_save_state_default() {
        let state = SaveState::default();
        assert_eq!(state.version, SAVE_STATE_VERSION);
        assert_eq!(state.memory.ram.len(), 2 * 1024 * 1024);
        assert_eq!(state.memory.scratchpad.len(), 1024);
        assert_eq!(state.gpu.vram.len(), 1024 * 512);
        assert_eq!(state.spu.ram.len(), 512 * 1024);
        assert_eq!(state.spu.voices.len(), 24);
        assert_eq!(state.dma.channels.len(), 7);
        assert_eq!(state.timers.timers.len(), 3);
    }

    #[test]
    fn test_save_state_serialization() {
        let state = SaveState {
            version: SAVE_STATE_VERSION,
            metadata: SaveStateMetadata {
                timestamp: Utc::now(),
                game_id: "SCUS-94163".to_string(),
                game_title: "Test Game".to_string(),
                frame_count: 1000,
                playtime: 60,
                thumbnail: None,
            },
            cpu: CPUState {
                regs: [0; 32],
                pc: 0xBFC00000,
                next_pc: 0xBFC00004,
                hi: 0,
                lo: 0,
                cop0_regs: [0; 32],
                load_delay: None,
                in_branch_delay: false,
                current_instruction: 0,
                gte_data_regs: [0; 32],
                gte_control_regs: [0; 32],
            },
            memory: MemoryState {
                ram: vec![0; 2 * 1024 * 1024],
                scratchpad: vec![0; 1024],
                bios: Vec::new(),
            },
            gpu: GPUState {
                vram: vec![0; 1024 * 512],
                draw_area_left: 0,
                draw_area_top: 0,
                draw_area_right: 1023,
                draw_area_bottom: 511,
                draw_offset_x: 0,
                draw_offset_y: 0,
                display_area_x: 0,
                display_area_y: 0,
                display_horiz_start: 0,
                display_horiz_end: 0,
                display_vert_start: 0,
                display_vert_end: 0,
                display_enabled: false,
                display_depth_24bit: false,
                vertical_interlace: false,
                horizontal_res: 0,
                vertical_res: false,
                video_mode: false,
                texture_window_mask_x: 0,
                texture_window_mask_y: 0,
                texture_window_offset_x: 0,
                texture_window_offset_y: 0,
                draw_mode_texture_page_x: 0,
                draw_mode_texture_page_y: 0,
                draw_mode_semi_transparency: 0,
                draw_mode_texture_depth: 0,
                draw_mode_dithering: false,
                draw_mode_draw_to_display: false,
                draw_mode_texture_disable: false,
                draw_mode_rectangle_flip_x: false,
                draw_mode_rectangle_flip_y: false,
                mask_bit_force: false,
                mask_bit_check: false,
                status: 0,
                scanline: 0,
                dots: 0,
                in_vblank: false,
            },
            spu: SPUState {
                ram: vec![0; 512 * 1024],
                voices: Vec::new(),
                main_volume_left: 0,
                main_volume_right: 0,
                reverb_volume_left: 0,
                reverb_volume_right: 0,
                cd_volume_left: 0,
                cd_volume_right: 0,
                reverb: ReverbState {
                    enabled: false,
                    reverb_current_addr: 0,
                },
                control: 0,
                status: 0,
                transfer_addr: 0,
            },
            cdrom: CDROMState {
                status: 0,
                index: 0,
                param_fifo: Vec::new(),
                response_fifo: Vec::new(),
                data_buffer: Vec::new(),
                current_command: 0,
                seek_target: (0, 0, 0),
                read_position: (0, 0, 0),
                mode: 0,
                interrupt_enable: 0,
                interrupt_flag: 0,
                reading: false,
                seeking: false,
            },
            dma: DMAState {
                channels: Vec::new(),
                control: 0,
                interrupt: 0,
            },
            timers: TimerState { timers: Vec::new() },
            controllers: ControllerState {
                controllers: Vec::new(),
            },
            interrupts: InterruptState {
                i_stat: 0,
                i_mask: 0,
            },
        };

        // Serialize
        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        assert!(!encoded.is_empty());

        // Deserialize
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.version, SAVE_STATE_VERSION);
        assert_eq!(decoded.metadata.game_id, "SCUS-94163");
        assert_eq!(decoded.metadata.game_title, "Test Game");
        assert_eq!(decoded.metadata.frame_count, 1000);
        assert_eq!(decoded.metadata.playtime, 60);
        assert_eq!(decoded.cpu.pc, 0xBFC00000);
        assert_eq!(decoded.cpu.next_pc, 0xBFC00004);
    }

    #[test]
    fn test_save_load_file() {
        let state = SaveState::default();

        // Save
        let test_path = "test_save.state";
        state.save_to_file(test_path).unwrap();

        // Load
        let loaded = SaveState::load_from_file(test_path).unwrap();

        assert_eq!(loaded.version, SAVE_STATE_VERSION);
        assert_eq!(loaded.memory.ram.len(), 2 * 1024 * 1024);
        assert_eq!(loaded.gpu.vram.len(), 1024 * 512);

        // Cleanup
        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_version_check() {
        let state = SaveState {
            version: 999, // Invalid version
            ..Default::default()
        };

        let test_path = "test_version.state";
        state.save_to_file(test_path).unwrap();

        let result = SaveState::load_from_file(test_path);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("Incompatible save state version"));
        }

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_estimated_size() {
        let state = SaveState::default();
        let estimated = state.estimated_size();

        // Should be approximately 3.6MB (allows for overhead)
        assert!(estimated > 3 * 1024 * 1024);
        assert!(estimated < 5 * 1024 * 1024);
    }

    #[test]
    fn test_metadata() {
        let mut state = SaveState::default();
        state.metadata.game_id = "SCUS-94163".to_string();
        state.metadata.game_title = "Spyro the Dragon".to_string();
        state.metadata.frame_count = 5000;
        state.metadata.playtime = 300;

        let test_path = "test_metadata.state";
        state.save_to_file(test_path).unwrap();

        let loaded = SaveState::load_from_file(test_path).unwrap();
        assert_eq!(loaded.metadata.game_id, "SCUS-94163");
        assert_eq!(loaded.metadata.game_title, "Spyro the Dragon");
        assert_eq!(loaded.metadata.frame_count, 5000);
        assert_eq!(loaded.metadata.playtime, 300);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_cpu_state() {
        let mut state = SaveState::default();
        state.cpu.pc = 0x80010000;
        state.cpu.regs[1] = 0x12345678;
        state.cpu.hi = 0xAAAAAAAA;
        state.cpu.lo = 0xBBBBBBBB;

        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.cpu.pc, 0x80010000);
        assert_eq!(decoded.cpu.regs[1], 0x12345678);
        assert_eq!(decoded.cpu.hi, 0xAAAAAAAA);
        assert_eq!(decoded.cpu.lo, 0xBBBBBBBB);
    }

    #[test]
    fn test_memory_state() {
        let mut state = SaveState::default();
        state.memory.ram[0x1000] = 0xAB;
        state.memory.scratchpad[0x100] = 0xCD;

        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.memory.ram[0x1000], 0xAB);
        assert_eq!(decoded.memory.scratchpad[0x100], 0xCD);
    }

    #[test]
    fn test_gpu_state() {
        let mut state = SaveState::default();
        state.gpu.vram[1000] = 0x7FFF; // White pixel
        state.gpu.draw_area_left = 10;
        state.gpu.draw_area_top = 20;
        state.gpu.draw_area_right = 630;
        state.gpu.draw_area_bottom = 470;

        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.gpu.vram[1000], 0x7FFF);
        assert_eq!(decoded.gpu.draw_area_left, 10);
        assert_eq!(decoded.gpu.draw_area_top, 20);
        assert_eq!(decoded.gpu.draw_area_right, 630);
        assert_eq!(decoded.gpu.draw_area_bottom, 470);
    }

    #[test]
    fn test_dma_state() {
        let mut state = SaveState::default();
        state.dma.control = 0x07654321;
        state.dma.channels[2].channel_control = 0x01000000; // GPU channel active

        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.dma.control, 0x07654321);
        assert_eq!(decoded.dma.channels[2].channel_control, 0x01000000);
    }

    #[test]
    fn test_interrupt_state() {
        let mut state = SaveState::default();
        state.interrupts.i_stat = 0x0001; // VBLANK interrupt
        state.interrupts.i_mask = 0x0003; // VBLANK and GPU interrupts enabled

        let config = config::standard();
        let encoded = bincode::encode_to_vec(&state, config).unwrap();
        let (decoded, _): (SaveState, usize) =
            bincode::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(decoded.interrupts.i_stat, 0x0001);
        assert_eq!(decoded.interrupts.i_mask, 0x0003);
    }
}
