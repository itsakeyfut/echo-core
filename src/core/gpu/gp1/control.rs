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

//! GP1 control commands
//!
//! Implements GPU control operations including reset, interrupt, and DMA.

use super::super::GPU;

impl GPU {
    /// GP1(0x00): Reset GPU
    ///
    /// Resets the GPU to its initial state without clearing VRAM.
    /// Per PSX-SPX specification, VRAM contents are preserved.
    pub(in crate::core::gpu) fn gp1_reset_gpu(&mut self) {
        // Reset GPU state without clearing VRAM (per PSX-SPX spec)
        self.reset_state_preserving_vram();
        self.display_mode.display_disabled = true;
        self.status.display_disabled = true;

        log::debug!("GPU reset");
    }

    /// GP1(0x01): Reset Command Buffer
    ///
    /// Clears the GP0 command FIFO and cancels any ongoing commands.
    /// This is useful for recovering from command processing errors.
    pub(in crate::core::gpu) fn gp1_reset_command_buffer(&mut self) {
        // Clear pending commands
        self.command_fifo.clear();

        // Cancel any ongoing VRAM transfer
        self.vram_transfer = None;

        log::debug!("Command buffer reset");
    }

    /// GP1(0x02): Acknowledge GPU Interrupt
    ///
    /// Clears the GPU interrupt request flag. The GPU can generate
    /// interrupts for certain operations, though this is rarely used.
    pub(in crate::core::gpu) fn gp1_acknowledge_interrupt(&mut self) {
        self.status.interrupt_request = false;
        log::debug!("GPU interrupt acknowledged");
    }

    /// GP1(0x04): DMA Direction
    ///
    /// Sets the DMA transfer direction/mode.
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-1: Direction (0=Off, 1=FIFO, 2=CPUtoGP0, 3=GPUREADtoCPU)
    pub(in crate::core::gpu) fn gp1_dma_direction(&mut self, value: u32) {
        let direction = (value & 3) as u8;
        self.status.dma_direction = direction;

        match direction {
            0 => log::debug!("DMA off"),
            1 => log::debug!("DMA FIFO"),
            2 => log::debug!("DMA CPU→GP0"),
            3 => log::debug!("DMA GPUREAD→CPU"),
            _ => unreachable!(),
        }
    }

    /// GP1(0x10): GPU Info
    ///
    /// Requests GPU information to be returned via the GPUREAD register.
    /// Different info types return different GPU state information.
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-7: Info type
    ///   - 0x02: Texture window settings
    ///   - 0x03: Draw area top left
    ///   - 0x04: Draw area bottom right
    ///   - 0x05: Draw offset
    ///   - 0x07: GPU version (returns 2 for PSX)
    pub(in crate::core::gpu) fn gp1_get_gpu_info(&mut self, value: u32) {
        let info_type = value & 0xFF;

        log::debug!("GPU info request: type {}", info_type);

        // TODO: Implement proper GPU info responses via GPUREAD register
        // Info types:
        // 0x02 - Texture window
        // 0x03 - Draw area top left
        // 0x04 - Draw area bottom right
        // 0x05 - Draw offset
        // 0x07 - GPU version (2 for PSX)
    }
}
