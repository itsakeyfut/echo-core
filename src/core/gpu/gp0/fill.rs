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

//! GP0 Fill Rectangle command
//!
//! Implements the GP0(0x02) Fill Rectangle command, which performs fast VRAM fills.
//! This command is commonly used by the BIOS and games for clearing buffers and
//! initializing VRAM regions.

use super::super::GPU;

impl GPU {
    /// GP0(0x02): Fill Rectangle in VRAM
    ///
    /// Fills a rectangular region of VRAM with a solid color. Unlike normal rectangle
    /// drawing commands (0x60-0x7F), this command:
    /// - Operates directly on VRAM coordinates (not screen coordinates)
    /// - Ignores drawing area/offset settings
    /// - Is optimized for speed (used for buffer clears)
    /// - Fills in 16-pixel wide strips (hardware optimization)
    ///
    /// # Command Format
    ///
    /// ```text
    /// Command word:
    /// Bits 0-7:   Red component
    /// Bits 8-15:  Green component
    /// Bits 16-23: Blue component
    /// Bits 24-31: Command (0x02)
    ///
    /// Parameter 1: Top-Left Corner (X,Y)
    /// Bits 0-15:  Y coordinate (in VRAM)
    /// Bits 16-31: X coordinate (in VRAM)
    ///
    /// Parameter 2: Width + Height
    /// Bits 0-15:  Height (in pixels)
    /// Bits 16-31: Width (in pixels)
    /// ```
    ///
    /// # Hardware Behavior
    ///
    /// - Coordinate wrapping: Uses raw VRAM coordinates (0-1023, 0-511)
    /// - Width alignment: Width is rounded up to multiples of 16 pixels
    /// - Ignores settings: Does NOT respect drawing area, drawing offset, or mask bits
    /// - Color format: RGB color is converted to 15-bit format (5-5-5)
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::GPU;
    /// let mut gpu = GPU::new();
    ///
    /// // Fill 100×100 region at (50, 50) with red (0xFF0000)
    /// gpu.write_gp0(0x02FF0000); // Command + Red color
    /// gpu.write_gp0(0x00320032); // X=50, Y=50
    /// gpu.write_gp0(0x00640064); // Width=100, Height=100
    ///
    /// // Verify the fill - note width is aligned to 16 pixels (100 → 112)
    /// assert_eq!(gpu.read_vram(50, 50), 0x001F); // Red in 15-bit format
    /// ```
    pub(in crate::core::gpu) fn gp0_fill_rectangle(&mut self) {
        // Need 3 words for fill command
        if self.command_fifo.len() < 3 {
            return;
        }

        // Extract command words
        let cmd_word = self.command_fifo.pop_front().unwrap();
        let coords = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        // Extract RGB color from command word (bits 0-23)
        let r = (cmd_word & 0xFF) as u8;
        let g = ((cmd_word >> 8) & 0xFF) as u8;
        let b = ((cmd_word >> 16) & 0xFF) as u8;

        // Convert 8-bit RGB to 15-bit (5-5-5) format
        // Each 8-bit component (0-255) is converted to 5-bit (0-31) by right-shifting by 3
        let r5 = (r >> 3) as u16;
        let g5 = (g >> 3) as u16;
        let b5 = (b >> 3) as u16;
        let color = r5 | (g5 << 5) | (b5 << 10);

        // Extract coordinates - note the bit layout matches PSX-SPX spec
        // Parameter 1: YyyyXxxx (16-bit Y, 16-bit X)
        let x = ((coords >> 16) & 0xFFFF) as u16;
        let y = (coords & 0xFFFF) as u16;

        // Extract size - note the bit layout
        // Parameter 2: HhhhWwww (16-bit Height, 16-bit Width)
        let width = ((size >> 16) & 0xFFFF) as u16;
        let height = (size & 0xFFFF) as u16;

        // Apply VRAM coordinate wrapping
        let x = x & 0x3FF; // 10-bit (0-1023)
        let y = y & 0x1FF; // 9-bit (0-511)

        // Apply width/height masking
        let width = width & 0x3FF; // 10-bit max
        let height = height & 0x1FF; // 9-bit max

        // Hardware aligns width to 16-pixel boundaries (rounds up)
        let aligned_width = (width + 15) & !15;

        log::debug!(
            "Fill Rectangle: ({}, {}) size {}×{} (aligned width: {}) color=0x{:04X} (RGB {},{},{})",
            x,
            y,
            width,
            height,
            aligned_width,
            color,
            r,
            g,
            b
        );

        // Perform the fill operation
        self.fill_vram_rect(x, y, aligned_width, height, color);
    }

    /// Fill a rectangular region of VRAM with a solid color
    ///
    /// This is a direct VRAM operation that bypasses all drawing settings
    /// (drawing area, offset, mask bits, etc.).
    ///
    /// # Arguments
    ///
    /// * `x` - Top-left X coordinate in VRAM (will be wrapped to 0-1023)
    /// * `y` - Top-left Y coordinate in VRAM (will be wrapped to 0-511)
    /// * `width` - Width in pixels (should already be 16-pixel aligned)
    /// * `height` - Height in pixels
    /// * `color` - 16-bit color value in 5-5-5 RGB format
    ///
    /// # Note
    ///
    /// Coordinates automatically wrap at VRAM boundaries (1024×512).
    #[inline]
    fn fill_vram_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) {
        // Fill rectangle row by row
        for dy in 0..height {
            let vram_y = (y + dy) & 0x1FF; // Wrap Y coordinate

            for dx in 0..width {
                let vram_x = (x + dx) & 0x3FF; // Wrap X coordinate
                self.write_vram(vram_x, vram_y, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_rectangle_parsing() {
        let mut gpu = GPU::new();

        // Fill 32×32 region at (100, 100) with white (0xFFFFFF)
        gpu.write_gp0(0x02FFFFFF); // Command + White color
        gpu.write_gp0(0x00640064); // X=100, Y=100
        gpu.write_gp0(0x00200020); // Width=32, Height=32

        // Verify pixels are white (0x7FFF in 15-bit format)
        assert_eq!(gpu.read_vram(100, 100), 0x7FFF); // Top-left
        assert_eq!(gpu.read_vram(131, 100), 0x7FFF); // Top-right (100 + 31)
        assert_eq!(gpu.read_vram(100, 131), 0x7FFF); // Bottom-left
        assert_eq!(gpu.read_vram(131, 131), 0x7FFF); // Bottom-right
    }

    #[test]
    fn test_fill_rectangle_width_alignment() {
        let mut gpu = GPU::new();

        // Fill 100×50 region - width should be aligned to 112 pixels (100 rounded up to next 16)
        // Color format: 0xCCBBGGRR (Command, Blue, Green, Red)
        // Coordinate format: YyyyXxxx (Y in bits 0-15, X in bits 16-31)
        // Size format: WwwwHhhh (Width in bits 16-31, Height in bits 0-15)
        gpu.write_gp0(0x02FF0000); // Command + Blue color (0x0000FF in RGB)
        gpu.write_gp0(0x00000000); // X=0, Y=0
        gpu.write_gp0(0x00640032); // Width=100 (0x64), Height=50 (0x32)

        // Verify that width was aligned to 112 pixels
        // Blue = 0xFF >> 3 = 0x1F in 5-bit format, shifted left by 10 = 0x7C00
        assert_eq!(gpu.read_vram(0, 0), 0x7C00); // First pixel
        assert_eq!(gpu.read_vram(99, 0), 0x7C00); // Original width
        assert_eq!(gpu.read_vram(111, 0), 0x7C00); // Aligned width (112 - 1)
        assert_eq!(gpu.read_vram(112, 0), 0x0000); // Beyond aligned width
    }

    #[test]
    fn test_fill_rectangle_coordinate_wrapping() {
        let mut gpu = GPU::new();

        // Fill at coordinates that exceed VRAM bounds - should wrap
        gpu.write_gp0(0x0200FF00); // Command + Green color
        gpu.write_gp0(0x04000400); // X=1024, Y=1024 (should wrap to 0, 0)
        gpu.write_gp0(0x00100010); // Width=16, Height=16

        // Green = 0xFF >> 3 = 0x1F, shifted left by 5 = 0x03E0
        assert_eq!(gpu.read_vram(0, 0), 0x03E0); // Wrapped to (0, 0)
    }

    #[test]
    fn test_fill_rectangle_color_conversion() {
        let mut gpu = GPU::new();

        // Test RGB to 15-bit conversion
        gpu.write_gp0(0x02F8F8F8); // Command + (248, 248, 248) RGB
        gpu.write_gp0(0x00500050); // X=80, Y=80
        gpu.write_gp0(0x00100010); // Width=16, Height=16

        // 248 >> 3 = 31 (0x1F) for all channels
        // Result: 0x1F | (0x1F << 5) | (0x1F << 10) = 0x7FFF (white)
        assert_eq!(gpu.read_vram(80, 80), 0x7FFF);
    }

    #[test]
    fn test_fill_rectangle_large_region() {
        let mut gpu = GPU::new();

        // Fill a large region (256×256)
        // Color format: 0xCCBBGGRR (Command, Blue, Green, Red)
        gpu.write_gp0(0x020000FF); // Command + Red color (0xFF0000 in RGB)
        gpu.write_gp0(0x01000100); // X=256, Y=256
        gpu.write_gp0(0x01000100); // Width=256, Height=256

        // Red = 0xFF >> 3 = 0x1F in 5-bit format, so color = 0x001F
        assert_eq!(gpu.read_vram(256, 256), 0x001F); // Top-left
        assert_eq!(gpu.read_vram(511, 256), 0x001F); // Top-right
        assert_eq!(gpu.read_vram(256, 511), 0x001F); // Bottom-left
        assert_eq!(gpu.read_vram(511, 511), 0x001F); // Bottom-right
        assert_eq!(gpu.read_vram(400, 400), 0x001F); // Middle
    }

    #[test]
    fn test_fill_rectangle_ignores_drawing_area() {
        let mut gpu = GPU::new();

        // Set drawing area to small region
        gpu.write_gp0(0xE3000000); // Draw area top-left = (0, 0)
        gpu.write_gp0(0xE4003200); // Draw area bottom-right = (50, 50)

        // Fill outside the drawing area - should NOT be clipped
        gpu.write_gp0(0x02FFFFFF); // Command + White color
        gpu.write_gp0(0x00640064); // X=100, Y=100 (outside draw area)
        gpu.write_gp0(0x00200020); // Width=32, Height=32

        // Pixels should be filled even though they're outside the draw area
        assert_eq!(gpu.read_vram(100, 100), 0x7FFF);
    }

    #[test]
    fn test_fill_rectangle_zero_size() {
        let mut gpu = GPU::new();

        // Fill with zero width/height - should do nothing
        gpu.write_gp0(0x02FFFFFF); // Command + White color
        gpu.write_gp0(0x00000000); // X=0, Y=0
        gpu.write_gp0(0x00000000); // Width=0, Height=0

        // VRAM should remain black
        assert_eq!(gpu.read_vram(0, 0), 0x0000);
    }
}
