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

//! GPU (Graphics Processing Unit) implementation
//!
//! This module implements the Sony CXD8561Q GPU (Graphics Processing Unit) used in the
//! PlayStation console. The GPU is responsible for:
//! - Managing 1MB of VRAM (1024×512 pixels, 16-bit per pixel)
//! - Processing GP0 (drawing) and GP1 (control) commands
//! - Rendering primitives (polygons, lines, rectangles)
//! - Display output control
//!
//! # VRAM Layout
//!
//! The GPU has 1MB of VRAM organized as a 1024×512 pixel framebuffer where each pixel
//! is 16-bit (5-5-5-bit RGB). The framebuffer can be used flexibly for display buffers,
//! textures, and color lookup tables (CLUTs).
//!
//! # Coordinate System
//!
//! The coordinate system origin (0, 0) is at the top-left corner of VRAM:
//! - X-axis: 0 to 1023 (left to right)
//! - Y-axis: 0 to 511 (top to bottom)
//!
//! # Color Format
//!
//! VRAM pixels use 16-bit color in 5-5-5 RGB format:
//! - Bits 0-4: Red (5 bits)
//! - Bits 5-9: Green (5 bits)
//! - Bits 10-14: Blue (5 bits)
//! - Bit 15: Mask bit (used for draw masking)
//!
//! # References
//!
//! - [PSX-SPX: GPU](http://problemkaputt.de/psx-spx.htm#gpu)
//! - [PSX-SPX: GPU Rendering](http://problemkaputt.de/psx-spx.htm#gpurenderstatecommands)

use std::collections::VecDeque;

/// GPU state representing the CXD8561 graphics processor
///
/// The GPU manages all graphics rendering and display functions for the PlayStation.
/// It includes 1MB of VRAM for framebuffers and textures, and processes drawing commands
/// via the GP0 and GP1 command interfaces.
///
/// # Examples
///
/// ```
/// use echo_core::core::GPU;
///
/// let mut gpu = GPU::new();
/// gpu.reset();
///
/// // Write a white pixel to VRAM
/// gpu.write_vram(100, 100, 0x7FFF);
/// assert_eq!(gpu.read_vram(100, 100), 0x7FFF);
/// ```
pub struct GPU {
    /// VRAM: 1024×512 pixels, 16-bit per pixel (1MB total)
    ///
    /// Stored as a flat Vec for cache efficiency. Pixels are stored in row-major order
    /// (left-to-right, top-to-bottom). Each pixel is a 16-bit value in 5-5-5 RGB format.
    vram: Vec<u16>,

    /// Drawing mode state
    draw_mode: DrawMode,

    /// Drawing area (clipping rectangle)
    ///
    /// All drawing operations are clipped to this rectangle.
    draw_area: DrawingArea,

    /// Drawing offset (added to all vertex coordinates)
    ///
    /// This offset is applied to all vertex positions before rendering.
    draw_offset: (i16, i16),

    /// Texture window settings
    ///
    /// Defines texture coordinate wrapping and masking behavior.
    texture_window: TextureWindow,

    /// Display area settings
    ///
    /// Defines the region of VRAM that is output to the display.
    display_area: DisplayArea,

    /// Display mode (resolution, color depth, etc.)
    display_mode: DisplayMode,

    /// Command FIFO buffer
    ///
    /// Stores GP0 commands that are being processed.
    command_fifo: VecDeque<u32>,

    /// Current command being processed
    current_command: Option<GPUCommand>,

    /// GPU status flags
    status: GPUStatus,

    /// VRAM transfer state
    ///
    /// Tracks the state of ongoing VRAM-to-CPU or CPU-to-VRAM transfers.
    vram_transfer: Option<VRAMTransfer>,
}

/// Drawing mode configuration
///
/// Specifies how primitives are rendered, including texture mapping settings,
/// transparency mode, and dithering.
#[derive(Debug, Clone, Copy, Default)]
pub struct DrawMode {
    /// Texture page base X coordinate (in units of 64 pixels)
    pub texture_page_x_base: u16,

    /// Texture page base Y coordinate (0 or 256)
    pub texture_page_y_base: u16,

    /// Semi-transparency mode (0-3)
    ///
    /// - 0: 0.5×Back + 0.5×Front (average)
    /// - 1: 1.0×Back + 1.0×Front (additive)
    /// - 2: 1.0×Back - 1.0×Front (subtractive)
    /// - 3: 1.0×Back + 0.25×Front (additive with quarter)
    pub semi_transparency: u8,

    /// Texture color depth (0=4bit, 1=8bit, 2=15bit)
    pub texture_depth: u8,

    /// Dithering enabled
    ///
    /// When enabled, dithers 24-bit colors down to 15-bit for display.
    pub dithering: bool,

    /// Drawing to display area allowed
    pub draw_to_display: bool,

    /// Texture disable (draw solid colors instead of textured)
    pub texture_disable: bool,

    /// Textured rectangle X-flip
    pub texture_x_flip: bool,

    /// Textured rectangle Y-flip
    pub texture_y_flip: bool,
}

/// Drawing area (clipping rectangle)
///
/// Defines the rectangular region in VRAM where drawing operations are allowed.
/// Primitives are clipped to this region.
#[derive(Debug, Clone, Copy)]
pub struct DrawingArea {
    /// Left edge X coordinate (inclusive)
    pub left: u16,

    /// Top edge Y coordinate (inclusive)
    pub top: u16,

    /// Right edge X coordinate (inclusive)
    pub right: u16,

    /// Bottom edge Y coordinate (inclusive)
    pub bottom: u16,
}

impl Default for DrawingArea {
    fn default() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 1023,
            bottom: 511,
        }
    }
}

/// Texture window settings
///
/// Controls texture coordinate wrapping and masking within a specified window.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextureWindow {
    /// Texture window mask X (in 8-pixel steps)
    pub mask_x: u8,

    /// Texture window mask Y (in 8-pixel steps)
    pub mask_y: u8,

    /// Texture window offset X (in 8-pixel steps)
    pub offset_x: u8,

    /// Texture window offset Y (in 8-pixel steps)
    pub offset_y: u8,
}

/// Display area configuration
///
/// Defines the region of VRAM that is output to the display.
#[derive(Debug, Clone, Copy)]
pub struct DisplayArea {
    /// Display area X coordinate in VRAM
    pub x: u16,

    /// Display area Y coordinate in VRAM
    pub y: u16,

    /// Display width in pixels
    pub width: u16,

    /// Display height in pixels
    pub height: u16,
}

impl Default for DisplayArea {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 320,
            height: 240,
        }
    }
}

/// Display mode settings
///
/// Controls the display output format including resolution, video mode, and color depth.
#[derive(Debug, Clone, Copy)]
pub struct DisplayMode {
    /// Horizontal resolution
    pub horizontal_res: HorizontalRes,

    /// Vertical resolution
    pub vertical_res: VerticalRes,

    /// Video mode (NTSC/PAL)
    pub video_mode: VideoMode,

    /// Display area color depth
    pub display_area_color_depth: ColorDepth,

    /// Interlaced mode enabled
    pub interlaced: bool,

    /// Display disabled
    pub display_disabled: bool,
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self {
            horizontal_res: HorizontalRes::R320,
            vertical_res: VerticalRes::R240,
            video_mode: VideoMode::NTSC,
            display_area_color_depth: ColorDepth::C15Bit,
            interlaced: false,
            display_disabled: true,
        }
    }
}

/// Horizontal resolution modes
///
/// The GPU supports several horizontal resolutions for display output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalRes {
    /// 256 pixels wide
    R256,

    /// 320 pixels wide (most common)
    R320,

    /// 512 pixels wide
    R512,

    /// 640 pixels wide
    R640,

    /// 368 pixels wide (rarely used)
    R368,
}

/// Vertical resolution modes
///
/// The GPU supports two vertical resolutions, with different values for NTSC and PAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalRes {
    /// 240 lines (NTSC) or 256 lines (PAL)
    R240,

    /// 480 lines (NTSC interlaced) or 512 lines (PAL interlaced)
    R480,
}

/// Video mode (refresh rate)
///
/// Determines the video timing: NTSC (60Hz) or PAL (50Hz).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    /// NTSC mode: 60Hz refresh rate
    NTSC,

    /// PAL mode: 50Hz refresh rate
    PAL,
}

/// Display color depth
///
/// Specifies the color depth used for display output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 15-bit color (5-5-5 RGB)
    C15Bit,

    /// 24-bit color (8-8-8 RGB)
    C24Bit,
}

/// GPU status register flags
///
/// Represents all the status bits returned by the GPUSTAT register (0x1F801814).
#[derive(Debug, Clone, Copy)]
pub struct GPUStatus {
    /// Texture page X base (N×64)
    pub texture_page_x_base: u8,

    /// Texture page Y base (0=0, 1=256)
    pub texture_page_y_base: u8,

    /// Semi-transparency mode (0-3)
    pub semi_transparency: u8,

    /// Texture color depth (0=4bit, 1=8bit, 2=15bit)
    pub texture_depth: u8,

    /// Dithering enabled
    pub dithering: bool,

    /// Drawing to display area allowed
    pub draw_to_display: bool,

    /// Set mask bit when drawing
    pub set_mask_bit: bool,

    /// Check mask bit before drawing (don't draw if set)
    pub draw_pixels: bool,

    /// Interlace field (even/odd)
    pub interlace_field: bool,

    /// Reverse flag (used for debugging)
    pub reverse_flag: bool,

    /// Texture disable
    pub texture_disable: bool,

    /// Horizontal resolution 2 (368 mode bit)
    pub horizontal_res_2: u8,

    /// Horizontal resolution 1 (256/320/512/640)
    pub horizontal_res_1: u8,

    /// Vertical resolution (0=240, 1=480)
    pub vertical_res: bool,

    /// Video mode (0=NTSC, 1=PAL)
    pub video_mode: bool,

    /// Display area color depth (0=15bit, 1=24bit)
    pub display_area_color_depth: bool,

    /// Vertical interlace enabled
    pub vertical_interlace: bool,

    /// Display disabled
    pub display_disabled: bool,

    /// Interrupt request
    pub interrupt_request: bool,

    /// DMA request
    pub dma_request: bool,

    /// Ready to receive command
    pub ready_to_receive_cmd: bool,

    /// Ready to send VRAM to CPU
    pub ready_to_send_vram: bool,

    /// Ready to receive DMA block
    pub ready_to_receive_dma: bool,

    /// DMA direction (0=Off, 1=?, 2=CPUtoGP0, 3=GPUREADtoCPU)
    pub dma_direction: u8,

    /// Drawing even/odd lines in interlaced mode
    pub drawing_odd_line: bool,
}

impl Default for GPUStatus {
    fn default() -> Self {
        Self {
            texture_page_x_base: 0,
            texture_page_y_base: 0,
            semi_transparency: 0,
            texture_depth: 0,
            dithering: false,
            draw_to_display: false,
            set_mask_bit: false,
            draw_pixels: false,
            interlace_field: false,
            reverse_flag: false,
            texture_disable: false,
            horizontal_res_2: 0,
            horizontal_res_1: 0,
            vertical_res: false,
            video_mode: false,
            display_area_color_depth: false,
            vertical_interlace: false,
            display_disabled: true,
            interrupt_request: false,
            dma_request: false,
            ready_to_receive_cmd: true,
            ready_to_send_vram: true,
            ready_to_receive_dma: true,
            dma_direction: 0,
            drawing_odd_line: false,
        }
    }
}

/// VRAM transfer state
///
/// Tracks the progress of a VRAM transfer operation (CPU-to-VRAM or VRAM-to-CPU).
#[derive(Debug, Clone)]
pub struct VRAMTransfer {
    /// Transfer start X coordinate
    pub x: u16,

    /// Transfer start Y coordinate
    pub y: u16,

    /// Transfer width in pixels
    pub width: u16,

    /// Transfer height in pixels
    pub height: u16,

    /// Current X position in transfer
    pub current_x: u16,

    /// Current Y position in transfer
    pub current_y: u16,
}

/// GPU command being processed
///
/// Represents a partially received GP0 command.
#[derive(Debug, Clone)]
pub struct GPUCommand {
    /// Command opcode
    pub opcode: u8,

    /// Command parameters
    pub params: Vec<u32>,

    /// Number of remaining words to receive
    pub remaining_words: usize,
}

impl GPU {
    /// VRAM width in pixels
    pub const VRAM_WIDTH: usize = 1024;

    /// VRAM height in pixels
    pub const VRAM_HEIGHT: usize = 512;

    /// Total VRAM size in pixels
    pub const VRAM_SIZE: usize = Self::VRAM_WIDTH * Self::VRAM_HEIGHT;

    /// Create a new GPU instance with initialized VRAM
    ///
    /// Initializes the GPU with:
    /// - All VRAM pixels set to black (0x0000)
    /// - Default drawing area (full VRAM)
    /// - Default display settings (320×240, NTSC)
    /// - Display initially disabled
    ///
    /// # Returns
    ///
    /// A new GPU instance ready for operation
    ///
    /// # Examples
    ///
    /// ```
    /// use echo_core::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// assert_eq!(gpu.read_vram(0, 0), 0x0000); // Black
    /// ```
    pub fn new() -> Self {
        Self {
            // Initialize VRAM to all black (0x0000)
            vram: vec![0x0000; Self::VRAM_SIZE],
            draw_mode: DrawMode::default(),
            draw_area: DrawingArea::default(),
            draw_offset: (0, 0),
            texture_window: TextureWindow::default(),
            display_area: DisplayArea::default(),
            display_mode: DisplayMode::default(),
            command_fifo: VecDeque::new(),
            current_command: None,
            status: GPUStatus::default(),
            vram_transfer: None,
        }
    }

    /// Reset GPU to initial state
    ///
    /// Clears all VRAM to black and resets all GPU state to default values.
    /// This is equivalent to a hardware reset.
    ///
    /// # Examples
    ///
    /// ```
    /// use echo_core::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// gpu.write_vram(500, 250, 0xFFFF);
    /// gpu.reset();
    /// assert_eq!(gpu.read_vram(500, 250), 0x0000); // Back to black
    /// ```
    pub fn reset(&mut self) {
        // Clear VRAM to black
        self.vram.fill(0x0000);

        // Reset all state to default values
        self.draw_mode = DrawMode::default();
        self.draw_area = DrawingArea::default();
        self.draw_offset = (0, 0);
        self.texture_window = TextureWindow::default();
        self.display_area = DisplayArea::default();
        self.display_mode = DisplayMode::default();
        self.command_fifo.clear();
        self.current_command = None;
        self.status = GPUStatus::default();
        self.vram_transfer = None;
    }

    /// Read a 16-bit pixel from VRAM
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (0-1023)
    /// * `y` - Y coordinate (0-511)
    ///
    /// # Returns
    ///
    /// The 16-bit pixel value in 5-5-5 RGB format
    ///
    /// # Note
    ///
    /// Coordinates are automatically wrapped to valid VRAM ranges
    /// (0-1023 for X, 0-511 for Y), matching PlayStation hardware behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use echo_core::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// let pixel = gpu.read_vram(100, 100);
    /// ```
    #[inline(always)]
    pub fn read_vram(&self, x: u16, y: u16) -> u16 {
        let index = self.vram_index(x, y);
        self.vram[index]
    }

    /// Write a 16-bit pixel to VRAM
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (0-1023)
    /// * `y` - Y coordinate (0-511)
    /// * `value` - 16-bit pixel value in 5-5-5 RGB format
    ///
    /// # Note
    ///
    /// Coordinates are automatically wrapped to valid VRAM ranges
    /// (0-1023 for X, 0-511 for Y), matching PlayStation hardware behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use echo_core::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// gpu.write_vram(100, 100, 0x7FFF); // White
    /// assert_eq!(gpu.read_vram(100, 100), 0x7FFF);
    /// ```
    #[inline(always)]
    pub fn write_vram(&mut self, x: u16, y: u16, value: u16) {
        let index = self.vram_index(x, y);
        self.vram[index] = value;
    }

    /// Get VRAM index from coordinates
    ///
    /// Converts 2D VRAM coordinates to a 1D array index.
    /// Coordinates are automatically wrapped to valid ranges.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    ///
    /// # Returns
    ///
    /// Linear index into the VRAM array
    #[inline(always)]
    fn vram_index(&self, x: u16, y: u16) -> usize {
        // Wrap coordinates to valid VRAM bounds
        let x = (x & 0x3FF) as usize; // 0-1023
        let y = (y & 0x1FF) as usize; // 0-511
        y * Self::VRAM_WIDTH + x
    }

    /// Get current GPU status register value
    ///
    /// Packs all GPU status flags into a 32-bit GPUSTAT register value
    /// that can be read from 0x1F801814.
    ///
    /// # Returns
    ///
    /// 32-bit GPU status register value
    ///
    /// # Register Format
    ///
    /// - Bits 0-3: Texture page X base
    /// - Bit 4: Texture page Y base
    /// - Bits 5-6: Semi-transparency
    /// - Bits 7-8: Texture depth
    /// - Bit 9: Dithering
    /// - Bit 10: Drawing to display
    /// - Bit 11: Set mask bit
    /// - Bit 12: Draw pixels
    /// - Bit 13: Interlace field
    /// - Bit 14: Reverse flag
    /// - Bit 15: Texture disable
    /// - Bit 16: Horizontal resolution 2
    /// - Bits 17-18: Horizontal resolution 1
    /// - Bit 19: Vertical resolution
    /// - Bit 20: Video mode
    /// - Bit 21: Display area color depth
    /// - Bit 22: Vertical interlace
    /// - Bit 23: Display disabled
    /// - Bit 24: Interrupt request
    /// - Bit 25: DMA request
    /// - Bit 26: Ready to receive command
    /// - Bit 27: Ready to send VRAM to CPU
    /// - Bit 28: Ready to receive DMA
    /// - Bits 29-30: DMA direction
    /// - Bit 31: Drawing odd line
    pub fn status(&self) -> u32 {
        let mut status = 0u32;

        status |= (self.status.texture_page_x_base as u32) & 0x0F;
        status |= ((self.status.texture_page_y_base as u32) & 0x01) << 4;
        status |= ((self.status.semi_transparency as u32) & 0x03) << 5;
        status |= ((self.status.texture_depth as u32) & 0x03) << 7;
        status |= (self.status.dithering as u32) << 9;
        status |= (self.status.draw_to_display as u32) << 10;
        status |= (self.status.set_mask_bit as u32) << 11;
        status |= (self.status.draw_pixels as u32) << 12;
        status |= (self.status.interlace_field as u32) << 13;
        status |= (self.status.reverse_flag as u32) << 14;
        status |= (self.status.texture_disable as u32) << 15;
        status |= ((self.status.horizontal_res_2 as u32) & 0x01) << 16;
        status |= ((self.status.horizontal_res_1 as u32) & 0x03) << 17;
        status |= (self.status.vertical_res as u32) << 19;
        status |= (self.status.video_mode as u32) << 20;
        status |= (self.status.display_area_color_depth as u32) << 21;
        status |= (self.status.vertical_interlace as u32) << 22;
        status |= (self.status.display_disabled as u32) << 23;
        status |= (self.status.interrupt_request as u32) << 24;
        status |= (self.status.dma_request as u32) << 25;
        status |= (self.status.ready_to_receive_cmd as u32) << 26;
        status |= (self.status.ready_to_send_vram as u32) << 27;
        status |= (self.status.ready_to_receive_dma as u32) << 28;
        status |= ((self.status.dma_direction as u32) & 0x03) << 29;
        status |= (self.status.drawing_odd_line as u32) << 31;

        status
    }

    /// Tick GPU (called once per CPU cycle)
    ///
    /// Advances the GPU state by the specified number of cycles.
    /// This is a placeholder for future timing-accurate emulation.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles to advance
    pub fn tick(&mut self, cycles: u32) {
        // TODO: Implement timing-accurate GPU emulation
        // For now, this is a placeholder
        let _ = cycles;
    }
}

impl Default for GPU {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_initialization() {
        let gpu = GPU::new();

        // Verify VRAM size
        assert_eq!(gpu.vram.len(), GPU::VRAM_SIZE);

        // All VRAM should be initialized to black
        assert!(gpu.vram.iter().all(|&pixel| pixel == 0x0000));
    }

    #[test]
    fn test_vram_read_write() {
        let mut gpu = GPU::new();

        // Write a pixel
        gpu.write_vram(100, 100, 0x7FFF); // White
        assert_eq!(gpu.read_vram(100, 100), 0x7FFF);

        // Test bounds (corners)
        gpu.write_vram(0, 0, 0x1234);
        assert_eq!(gpu.read_vram(0, 0), 0x1234);

        gpu.write_vram(1023, 511, 0x5678);
        assert_eq!(gpu.read_vram(1023, 511), 0x5678);
    }

    #[test]
    fn test_vram_index_calculation() {
        let gpu = GPU::new();

        // Top-left corner
        assert_eq!(gpu.vram_index(0, 0), 0);

        // One row down
        assert_eq!(gpu.vram_index(0, 1), GPU::VRAM_WIDTH);

        // Bottom-right corner
        assert_eq!(gpu.vram_index(1023, 511), (GPU::VRAM_WIDTH * 511) + 1023);

        // Test wrapping (coordinates beyond bounds wrap around)
        assert_eq!(gpu.vram_index(1024, 0), gpu.vram_index(0, 0));
        assert_eq!(gpu.vram_index(0, 512), gpu.vram_index(0, 0));
    }

    #[test]
    fn test_default_state() {
        let gpu = GPU::new();

        // Check default drawing area (full VRAM)
        assert_eq!(gpu.draw_area.left, 0);
        assert_eq!(gpu.draw_area.top, 0);
        assert_eq!(gpu.draw_area.right, 1023);
        assert_eq!(gpu.draw_area.bottom, 511);

        // Check display is initially disabled
        assert!(gpu.display_mode.display_disabled);

        // Check default resolution (320×240, NTSC)
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);
        assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R240);
        assert_eq!(gpu.display_mode.video_mode, VideoMode::NTSC);

        // Check default display area
        assert_eq!(gpu.display_area.width, 320);
        assert_eq!(gpu.display_area.height, 240);
    }

    #[test]
    fn test_status_register() {
        let gpu = GPU::new();
        let status = gpu.status();

        // Status register should be a valid 32-bit value
        assert_eq!(status & 0x1FFF_FFFF, status); // Check no reserved bits

        // Display should be disabled initially
        assert_ne!(status & (1 << 23), 0);

        // Ready flags should be set
        assert_ne!(status & (1 << 26), 0); // Ready to receive command
        assert_ne!(status & (1 << 27), 0); // Ready to send VRAM
        assert_ne!(status & (1 << 28), 0); // Ready to receive DMA
    }

    #[test]
    fn test_gpu_reset() {
        let mut gpu = GPU::new();

        // Modify some state
        gpu.write_vram(500, 250, 0xFFFF);
        gpu.draw_offset = (100, 100);
        gpu.display_mode.display_disabled = false;

        // Reset
        gpu.reset();

        // Verify state is reset
        assert_eq!(gpu.read_vram(500, 250), 0x0000);
        assert_eq!(gpu.draw_offset, (0, 0));
        assert!(gpu.display_mode.display_disabled);

        // Verify default drawing area
        assert_eq!(gpu.draw_area.left, 0);
        assert_eq!(gpu.draw_area.right, 1023);
    }

    #[test]
    fn test_vram_wrapping() {
        let mut gpu = GPU::new();

        // Write using wrapped coordinates
        gpu.write_vram(1024, 512, 0xABCD); // Should wrap to (0, 0)
        assert_eq!(gpu.read_vram(0, 0), 0xABCD);

        gpu.write_vram(1025, 513, 0x1234); // Should wrap to (1, 1)
        assert_eq!(gpu.read_vram(1, 1), 0x1234);
    }

    #[test]
    fn test_multiple_pixel_operations() {
        let mut gpu = GPU::new();

        // Write a pattern
        for i in 0..10 {
            gpu.write_vram(i * 10, i * 10, 0x1000 + i);
        }

        // Verify pattern
        for i in 0..10 {
            assert_eq!(gpu.read_vram(i * 10, i * 10), 0x1000 + i);
        }
    }

    #[test]
    fn test_vram_size_constants() {
        assert_eq!(GPU::VRAM_WIDTH, 1024);
        assert_eq!(GPU::VRAM_HEIGHT, 512);
        assert_eq!(GPU::VRAM_SIZE, 1024 * 512);
        assert_eq!(GPU::VRAM_SIZE, 524_288);
    }

    #[test]
    fn test_gpu_tick() {
        let mut gpu = GPU::new();

        // Tick should not panic
        gpu.tick(100);
        gpu.tick(1000);
    }

    #[test]
    fn test_default_trait() {
        let gpu1 = GPU::new();
        let gpu2 = GPU::default();

        // Both should have the same initial state
        assert_eq!(gpu1.vram.len(), gpu2.vram.len());
        assert_eq!(gpu1.read_vram(0, 0), gpu2.read_vram(0, 0));
    }
}
