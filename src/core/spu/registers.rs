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

//! SPU register definitions and types

/// SPU control register
///
/// Controls SPU operation including enable/mute, DMA transfer mode,
/// and audio input settings.
pub struct SPUControl {
    pub enabled: bool,
    pub unmute: bool,
    pub noise_clock: u8,
    pub noise_step: u8,
    pub reverb_enabled: bool,
    pub irq_enabled: bool,
    pub transfer_mode: TransferMode,
    pub external_audio_reverb: bool,
    pub cd_audio_reverb: bool,
    pub external_audio_enabled: bool,
    pub cd_audio_enabled: bool,
}

/// SPU status register
///
/// Provides status information about SPU operation including
/// IRQ flags, DMA status, and capture readiness.
#[derive(Default)]
pub struct SPUStatus {
    #[allow(dead_code)]
    pub mode: u16,
    pub irq_flag: bool,
    #[allow(dead_code)]
    pub dma_request: bool,
    pub dma_busy: bool,
    #[allow(dead_code)]
    pub capture_ready: bool,
}

/// SPU data transfer mode
///
/// Specifies how data is transferred to/from SPU RAM.
#[derive(Debug, Clone, Copy)]
pub enum TransferMode {
    /// No transfer
    Stop,
    /// Manual write via FIFO
    ManualWrite,
    /// DMA write to SPU RAM
    DMAWrite,
    /// DMA read from SPU RAM
    DMARead,
}

impl Default for SPUControl {
    fn default() -> Self {
        Self {
            enabled: false,
            unmute: false,
            noise_clock: 0,
            noise_step: 0,
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
