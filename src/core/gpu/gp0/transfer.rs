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

//! GP0 VRAM transfer commands
//!
//! Implements CPU↔VRAM and VRAM↔VRAM transfer operations.

use super::super::registers::{VRAMTransfer, VRAMTransferDirection};
use super::super::GPU;

impl GPU {
    /// GP0(0xA0): CPU→VRAM Transfer
    ///
    /// Initiates a transfer from CPU to VRAM. The transfer requires 3 command words:
    /// - Word 0: Command (0xA0000000)
    /// - Word 1: Destination coordinates (X in bits 0-15, Y in bits 16-31)
    /// - Word 2: Size (Width in bits 0-15, Height in bits 16-31)
    ///
    /// After this command, subsequent GP0 writes are treated as VRAM data.
    pub(crate) fn gp0_cpu_to_vram_transfer(&mut self) {
        if self.command_fifo.len() < 3 {
            return; // Need more words
        }

        // Extract command words
        let _ = self.command_fifo.pop_front().unwrap();
        let coords = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let x = (coords & 0xFFFF) as u16;
        let y = ((coords >> 16) & 0xFFFF) as u16;
        let width = (size & 0xFFFF) as u16;
        let height = ((size >> 16) & 0xFFFF) as u16;

        // Align to boundaries and apply hardware limits
        let x = x & 0x3FF; // 10-bit (0-1023)
        let y = y & 0x1FF; // 9-bit (0-511)
        let width = (width.wrapping_sub(1) & 0x03FF).wrapping_add(1);
        let height = (height.wrapping_sub(1) & 0x01FF).wrapping_add(1);

        log::debug!(
            "CPU→VRAM transfer: ({}, {}) size {}×{}",
            x,
            y,
            width,
            height
        );

        // Start VRAM transfer
        self.vram_transfer = Some(VRAMTransfer {
            x,
            y,
            width,
            height,
            current_x: 0,
            current_y: 0,
            direction: VRAMTransferDirection::CpuToVram,
        });
    }

    /// Process incoming VRAM write data during CPU→VRAM transfer
    ///
    /// Each word contains two 16-bit pixels. Pixels are written sequentially
    /// left-to-right, top-to-bottom within the transfer rectangle.
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit word containing two 16-bit pixels
    pub(crate) fn process_vram_write(&mut self, value: u32) {
        // Extract transfer state to avoid borrowing issues
        let mut transfer = match self.vram_transfer.take() {
            Some(t) => t,
            None => return,
        };

        // Each u32 contains two 16-bit pixels
        let pixel1 = (value & 0xFFFF) as u16;
        let pixel2 = ((value >> 16) & 0xFFFF) as u16;

        // Write first pixel
        let vram_x = (transfer.x + transfer.current_x) & 0x3FF;
        let vram_y = (transfer.y + transfer.current_y) & 0x1FF;
        self.write_vram(vram_x, vram_y, pixel1);

        transfer.current_x += 1;
        if transfer.current_x >= transfer.width {
            transfer.current_x = 0;
            transfer.current_y += 1;
        }

        // Write second pixel if transfer not complete
        if transfer.current_y < transfer.height {
            let vram_x = (transfer.x + transfer.current_x) & 0x3FF;
            let vram_y = (transfer.y + transfer.current_y) & 0x1FF;
            self.write_vram(vram_x, vram_y, pixel2);

            transfer.current_x += 1;
            if transfer.current_x >= transfer.width {
                transfer.current_x = 0;
                transfer.current_y += 1;
            }
        }

        // Check if transfer is complete
        if transfer.current_y >= transfer.height {
            log::debug!("CPU→VRAM transfer complete");
            // Transfer is complete, don't restore it
        } else {
            // Restore transfer state for next write
            self.vram_transfer = Some(transfer);
        }
    }

    /// GP0(0xC0): VRAM→CPU Transfer
    ///
    /// Initiates a transfer from VRAM to CPU. The transfer requires 3 command words:
    /// - Word 0: Command (0xC0000000)
    /// - Word 1: Source coordinates (X in bits 0-15, Y in bits 16-31)
    /// - Word 2: Size (Width in bits 0-15, Height in bits 16-31)
    ///
    /// After this command, the CPU can read pixel data via GPUREAD register.
    pub(crate) fn gp0_vram_to_cpu_transfer(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let _ = self.command_fifo.pop_front().unwrap();
        let coords = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let x = (coords & 0xFFFF) as u16 & 0x3FF;
        let y = ((coords >> 16) & 0xFFFF) as u16 & 0x1FF;
        let width = (((size & 0xFFFF) as u16).wrapping_sub(1) & 0x03FF).wrapping_add(1);
        let height = ((((size >> 16) & 0xFFFF) as u16).wrapping_sub(1) & 0x01FF).wrapping_add(1);

        log::debug!(
            "VRAM→CPU transfer: ({}, {}) size {}×{}",
            x,
            y,
            width,
            height
        );

        // Set up for reading
        self.vram_transfer = Some(VRAMTransfer {
            x,
            y,
            width,
            height,
            current_x: 0,
            current_y: 0,
            direction: VRAMTransferDirection::VramToCpu,
        });

        // Update status to indicate data is ready
        self.status.ready_to_send_vram = true;
    }

    /// GP0(0x80): VRAM→VRAM Transfer
    ///
    /// Copies a rectangle within VRAM. The transfer requires 4 command words:
    /// - Word 0: Command (0x80000000)
    /// - Word 1: Source coordinates (X in bits 0-15, Y in bits 16-31)
    /// - Word 2: Destination coordinates (X in bits 0-15, Y in bits 16-31)
    /// - Word 3: Size (Width in bits 0-15, Height in bits 16-31)
    ///
    /// The copy handles overlapping regions correctly by using a temporary buffer.
    pub(crate) fn gp0_vram_to_vram_transfer(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let _ = self.command_fifo.pop_front().unwrap();
        let src_coords = self.command_fifo.pop_front().unwrap();
        let dst_coords = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let src_x = (src_coords & 0xFFFF) as u16 & 0x3FF;
        let src_y = ((src_coords >> 16) & 0xFFFF) as u16 & 0x1FF;
        let dst_x = (dst_coords & 0xFFFF) as u16 & 0x3FF;
        let dst_y = ((dst_coords >> 16) & 0xFFFF) as u16 & 0x1FF;
        let width = (((size & 0xFFFF) as u16).wrapping_sub(1) & 0x03FF).wrapping_add(1);
        let height = ((((size >> 16) & 0xFFFF) as u16).wrapping_sub(1) & 0x01FF).wrapping_add(1);

        log::debug!(
            "VRAM→VRAM transfer: ({}, {}) → ({}, {}) size {}×{}",
            src_x,
            src_y,
            dst_x,
            dst_y,
            width,
            height
        );

        // Copy rectangle
        // Note: Need to handle overlapping regions correctly
        let mut temp_buffer = vec![0u16; (width as usize) * (height as usize)];

        // Read source
        for y in 0..height {
            for x in 0..width {
                let sx = (src_x + x) & 0x3FF;
                let sy = (src_y + y) & 0x1FF;
                temp_buffer[(y as usize) * (width as usize) + (x as usize)] =
                    self.read_vram(sx, sy);
            }
        }

        // Write destination
        for y in 0..height {
            for x in 0..width {
                let dx = (dst_x + x) & 0x3FF;
                let dy = (dst_y + y) & 0x1FF;
                let pixel = temp_buffer[(y as usize) * (width as usize) + (x as usize)];
                self.write_vram(dx, dy, pixel);
            }
        }
    }
}
