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

//! GP0 Drawing Mode Commands
//!
//! This module implements GP0 commands that control drawing settings such as
//! texture page, drawing area, drawing offset, and masking behavior.
//!
//! # Commands
//!
//! - 0xE1: Draw Mode Setting (texture page, transparency, dithering, etc.)
//! - 0xE2: Texture Window Setting
//! - 0xE3: Set Drawing Area Top-Left
//! - 0xE4: Set Drawing Area Bottom-Right
//! - 0xE5: Set Drawing Offset
//! - 0xE6: Mask Bit Setting
//!
//! # References
//!
//! - [PSX-SPX: GP0 Drawing Settings](http://problemkaputt.de/psx-spx.htm#gpurenderattributes)

use crate::core::gpu::GPU;

impl GPU {
    /// GP0(E1h) - Draw Mode Setting (aka "Texpage")
    ///
    /// Sets texture page location, texture color depth, semi-transparency mode,
    /// dithering, drawing to display area, and texture disable flags.
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE1000000 | params
    ///   Bit 0-3:   Texture page X Base   (N*64)
    ///   Bit 4:     Texture page Y Base   (N*256, 0=0, 1=256)
    ///   Bit 5-6:   Semi Transparency     (0=B/2+F/2, 1=B+F, 2=B-F, 3=B+F/4)
    ///   Bit 7-8:   Texture page colors   (0=4bit, 1=8bit, 2=15bit)
    ///   Bit 9:     Dithering enabled     (0=Off, 1=On)
    ///   Bit 10:    Drawing to display    (0=Prohibited, 1=Allowed)
    ///   Bit 11:    Texture disable       (0=Normal, 1=Disable)
    ///   Bit 12:    Textured rect X-flip (for Textured Rectangle command)
    ///   Bit 13:    Textured rect Y-flip (for Textured Rectangle command)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Set texture page to (128, 256) with 4-bit color
    /// gpu.write_gp0(0xE1000012);  // X=2 (2*64=128), Y=1 (1*256=256)
    /// ```
    pub(crate) fn gp0_draw_mode(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        // Texture page coordinates
        let texture_page_x_base = (cmd & 0xF) as u16 * 64;
        let texture_page_y_base = ((cmd >> 4) & 1) as u16 * 256;

        // Semi-transparency mode (0-3)
        let semi_transparency = ((cmd >> 5) & 3) as u8;

        // Texture depth (0=4bit, 1=8bit, 2/3=15bit)
        let texture_depth = ((cmd >> 7) & 3) as u8;

        // Dithering enable
        let dithering = ((cmd >> 9) & 1) != 0;

        // Drawing to display area allowed
        let draw_to_display = ((cmd >> 10) & 1) != 0;

        // Texture disable (draw solid colors instead)
        let texture_disable = ((cmd >> 11) & 1) != 0;

        // Texture flipping (for textured rectangles)
        let texture_x_flip = ((cmd >> 12) & 1) != 0;
        let texture_y_flip = ((cmd >> 13) & 1) != 0;

        // Update draw mode
        self.draw_mode.texture_page_x_base = texture_page_x_base;
        self.draw_mode.texture_page_y_base = texture_page_y_base;
        self.draw_mode.semi_transparency = semi_transparency;
        self.draw_mode.texture_depth = texture_depth;
        self.draw_mode.dithering = dithering;
        self.draw_mode.draw_to_display = draw_to_display;
        self.draw_mode.texture_disable = texture_disable;
        self.draw_mode.texture_x_flip = texture_x_flip;
        self.draw_mode.texture_y_flip = texture_y_flip;

        // Update GPU status to mirror draw mode (GPUSTAT must reflect GP0 settings)
        self.status.texture_page_x_base = (cmd & 0xF) as u8;
        self.status.texture_page_y_base = ((cmd >> 4) & 1) as u8;
        self.status.semi_transparency = semi_transparency;
        self.status.texture_depth = texture_depth;
        self.status.dithering = dithering;
        self.status.draw_to_display = draw_to_display;
        self.status.texture_disable = texture_disable;

        log::debug!(
            "Draw mode: page=({}, {}) depth={} semi={} dither={} tex_disable={}",
            texture_page_x_base,
            texture_page_y_base,
            texture_depth,
            semi_transparency,
            dithering,
            texture_disable
        );
    }

    /// GP0(E2h) - Texture Window Setting
    ///
    /// Sets the texture window which controls texture coordinate wrapping.
    /// The texture window allows wrapping and offsetting of texture coordinates
    /// within a specified rectangular region.
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE2000000 | params
    ///   Bit 0-4:   Texture window Mask X   (in 8 pixel steps)
    ///   Bit 5-9:   Texture window Mask Y   (in 8 pixel steps)
    ///   Bit 10-14: Texture window Offset X (in 8 pixel steps)
    ///   Bit 15-19: Texture window Offset Y (in 8 pixel steps)
    /// ```
    ///
    /// # Masking Formula
    ///
    /// The texture coordinates are modified as:
    /// ```text
    /// Texcoord = (Texcoord AND (NOT (Mask*8))) OR ((Offset AND Mask)*8)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Set texture window: mask=(8,8), offset=(16,16)
    /// gpu.write_gp0(0xE2000008 | (8 << 5) | (16 << 10) | (16 << 15));
    /// ```
    pub(crate) fn gp0_texture_window(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        let mask_x = (cmd & 0x1F) as u8;
        let mask_y = ((cmd >> 5) & 0x1F) as u8;
        let offset_x = ((cmd >> 10) & 0x1F) as u8;
        let offset_y = ((cmd >> 15) & 0x1F) as u8;

        self.texture_window.mask_x = mask_x;
        self.texture_window.mask_y = mask_y;
        self.texture_window.offset_x = offset_x;
        self.texture_window.offset_y = offset_y;

        log::debug!(
            "Texture window: mask=({}, {}) offset=({}, {})",
            mask_x,
            mask_y,
            offset_x,
            offset_y
        );
    }

    /// GP0(E3h) - Set Drawing Area Top-Left
    ///
    /// Sets the top-left corner of the drawing area (clipping rectangle).
    /// All drawing operations are clipped to the drawing area.
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE3000000 | params
    ///   Bit 0-9:   X-coordinate (0-1023)
    ///   Bit 10-18: Y-coordinate (0-511)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Set top-left to (100, 100)
    /// gpu.write_gp0(0xE3000064 | (100 << 10));
    /// ```
    pub(crate) fn gp0_draw_area_top_left(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        let x = (cmd & 0x3FF) as u16;
        let y = ((cmd >> 10) & 0x1FF) as u16;

        self.draw_area.left = x;
        self.draw_area.top = y;

        self.update_rasterizer_clip_rect();

        log::debug!("Draw area top-left: ({}, {})", x, y);
    }

    /// GP0(E4h) - Set Drawing Area Bottom-Right
    ///
    /// Sets the bottom-right corner of the drawing area (clipping rectangle).
    /// All drawing operations are clipped to the drawing area.
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE4000000 | params
    ///   Bit 0-9:   X-coordinate (0-1023)
    ///   Bit 10-18: Y-coordinate (0-511)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Set bottom-right to (200, 200)
    /// gpu.write_gp0(0xE40000C8 | (200 << 10));
    /// ```
    pub(crate) fn gp0_draw_area_bottom_right(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        let x = (cmd & 0x3FF) as u16;
        let y = ((cmd >> 10) & 0x1FF) as u16;

        self.draw_area.right = x;
        self.draw_area.bottom = y;

        self.update_rasterizer_clip_rect();

        log::debug!("Draw area bottom-right: ({}, {})", x, y);
    }

    /// GP0(E5h) - Set Drawing Offset
    ///
    /// Sets the drawing offset which is added to all vertex coordinates
    /// before rendering. The offset is a signed 11-bit value that is
    /// sign-extended to 16 bits.
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE5000000 | params
    ///   Bit 0-10:  X-offset (signed 11-bit, -1024 to +1023)
    ///   Bit 11-21: Y-offset (signed 11-bit, -1024 to +1023)
    /// ```
    ///
    /// # Sign Extension
    ///
    /// The 11-bit values are sign-extended to 16-bit signed integers.
    /// - Bit 10 is the sign bit
    /// - Positive: 0x000-0x3FF maps to 0 to +1023
    /// - Negative: 0x400-0x7FF maps to -1024 to -1
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Set offset to (10, -20)
    /// let x = 10u32;
    /// let y = ((-20i16) as u16 as u32) & 0x7FF;
    /// gpu.write_gp0(0xE5000000 | x | (y << 11));
    /// ```
    pub(crate) fn gp0_draw_offset(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        // Sign-extend 11-bit values to 16-bit
        // Take bits 0-10, shift left by 5, then arithmetic shift right by 5
        let x = ((cmd & 0x7FF) as i16) << 5 >> 5;
        let y = (((cmd >> 11) & 0x7FF) as i16) << 5 >> 5;

        self.draw_offset = (x, y);

        log::debug!("Draw offset: ({}, {})", x, y);
    }

    /// GP0(E6h) - Mask Bit Setting
    ///
    /// Controls the mask bit behavior when drawing pixels:
    /// - Set mask bit: Whether to set bit 15 of VRAM pixels when drawing
    /// - Check mask bit: Whether to skip drawing to pixels that have bit 15 set
    ///
    /// # Command Format
    ///
    /// ```text
    /// 0xE6000000 | params
    ///   Bit 0: Set mask bit while drawing       (0=No, 1=Yes/Bit15)
    ///   Bit 1: Check mask bit before draw       (0=Draw Always, 1=Draw only if Bit15=0)
    /// ```
    ///
    /// # Mask Bit Behavior
    ///
    /// - When "set mask bit" is enabled, all drawn pixels have bit 15 set to 1
    /// - When "check mask bit" is enabled, pixels with bit 15=1 are not overwritten
    /// - This is used to prevent certain areas from being drawn over
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Enable both mask bit set and check
    /// gpu.write_gp0(0xE6000003);
    /// ```
    pub(crate) fn gp0_mask_settings(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();

        let set_mask_while_drawing = (cmd & 1) != 0;
        let check_mask_before_draw = ((cmd >> 1) & 1) != 0;

        self.status.set_mask_bit = set_mask_while_drawing;
        self.status.draw_pixels = !check_mask_before_draw;

        log::debug!(
            "Mask settings: set={} check={}",
            set_mask_while_drawing,
            check_mask_before_draw
        );
    }
}
