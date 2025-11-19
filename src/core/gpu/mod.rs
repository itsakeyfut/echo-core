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

use super::timing::EventHandle;

// Module declarations
mod gp0;
mod gp1;
mod primitives;
mod registers;
mod render;
#[cfg(test)]
mod tests;

// Public re-exports
pub use primitives::*;
pub use registers::*;
pub use render::Rasterizer;

/// GPU state representing the CXD8561 graphics processor
///
/// The GPU manages all graphics rendering and display functions for the PlayStation.
/// It includes 1MB of VRAM for framebuffers and textures, and processes drawing commands
/// via the GP0 and GP1 command interfaces.
///
/// # Examples
///
/// ```
/// use psrx::core::GPU;
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
    pub(in crate::core::gpu) vram: Vec<u16>,

    /// Software rasterizer for drawing primitives
    ///
    /// Handles the actual pixel-level rasterization of triangles and other primitives.
    pub(in crate::core::gpu) rasterizer: Rasterizer,

    /// Drawing mode state
    pub(in crate::core::gpu) draw_mode: DrawMode,

    /// Drawing area (clipping rectangle)
    ///
    /// All drawing operations are clipped to this rectangle.
    pub(in crate::core::gpu) draw_area: DrawingArea,

    /// Drawing offset (added to all vertex coordinates)
    ///
    /// This offset is applied to all vertex positions before rendering.
    pub(in crate::core::gpu) draw_offset: (i16, i16),

    /// Texture window settings
    ///
    /// Defines texture coordinate wrapping and masking behavior.
    pub(in crate::core::gpu) texture_window: TextureWindow,

    /// Display area settings
    ///
    /// Defines the region of VRAM that is output to the display.
    pub(in crate::core::gpu) display_area: DisplayArea,

    /// Display mode (resolution, color depth, etc.)
    pub(in crate::core::gpu) display_mode: DisplayMode,

    /// Command FIFO buffer
    ///
    /// Stores GP0 commands that are being processed.
    pub(in crate::core::gpu) command_fifo: VecDeque<u32>,

    /// GPU status flags
    pub(in crate::core::gpu) status: GPUStatus,

    /// VRAM transfer state
    ///
    /// Tracks the state of ongoing VRAM-to-CPU or CPU-to-VRAM transfers.
    pub(in crate::core::gpu) vram_transfer: Option<VRAMTransfer>,

    /// Scanline counter (0-262 for NTSC)
    ///
    /// Tracks the current scanline being rendered. NTSC mode uses 263 scanlines total,
    /// with VBlank occurring during scanlines 243-262.
    scanline: u16,

    /// Dots counter (0-3412 per scanline)
    ///
    /// Tracks horizontal position within a scanline. Each scanline consists of 3413 dots
    /// at the PSX GPU pixel clock rate.
    dots: u16,

    /// VBlank status flag
    ///
    /// True when the GPU is in the vertical blanking period (scanlines 243-262 for NTSC).
    /// During VBlank, no active display output occurs and games typically perform
    /// frame synchronization and VRAM updates.
    in_vblank: bool,

    /// HBlank status flag
    ///
    /// True during horizontal blanking periods. Currently simplified (always false).
    /// Can be extended later for proper HBlank timing if needed by games.
    in_hblank: bool,

    // Timing event handles
    /// VBlank event handle
    vblank_event: Option<EventHandle>,

    /// HBlank event handle
    hblank_event: Option<EventHandle>,

    // Interrupt flags
    /// VBlank interrupt pending flag
    vblank_interrupt_pending: bool,

    /// HBlank interrupt pending flag
    hblank_interrupt_pending: bool,
}

impl GPU {
    /// VRAM width in pixels
    pub const VRAM_WIDTH: usize = 1024;

    /// VRAM height in pixels
    pub const VRAM_HEIGHT: usize = 512;

    /// Total VRAM size in pixels
    pub const VRAM_SIZE: usize = Self::VRAM_WIDTH * Self::VRAM_HEIGHT;

    /// Total scanlines per frame (NTSC)
    ///
    /// NTSC video uses 263 scanlines per frame (0-262 inclusive).
    pub const SCANLINES_PER_FRAME: u16 = 263;

    /// Dots per scanline
    ///
    /// Each scanline consists of 3413 dots at the GPU pixel clock rate.
    pub const DOTS_PER_SCANLINE: u16 = 3413;

    /// VBlank start scanline (NTSC)
    ///
    /// VBlank begins at scanline 243 in NTSC mode.
    pub const VBLANK_START: u16 = 243;

    /// VBlank end scanline (NTSC)
    ///
    /// VBlank ends when wrapping back to scanline 0 (at scanline 263).
    pub const VBLANK_END: u16 = 263;

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
    /// use psrx::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// assert_eq!(gpu.read_vram(0, 0), 0x0000); // Black
    /// ```
    pub fn new() -> Self {
        // Create GPU with rasterizer
        let mut gpu = Self {
            vram: vec![0x0000; Self::VRAM_SIZE],
            rasterizer: Rasterizer::new(),
            draw_mode: DrawMode::default(),
            draw_area: DrawingArea::default(),
            draw_offset: (0, 0),
            texture_window: TextureWindow::default(),
            display_area: DisplayArea::default(),
            display_mode: DisplayMode::default(),
            command_fifo: VecDeque::new(),
            status: GPUStatus::default(),
            vram_transfer: None,
            scanline: 0,
            dots: 0,
            in_vblank: false,
            in_hblank: false,
            vblank_event: None,
            hblank_event: None,
            vblank_interrupt_pending: false,
            hblank_interrupt_pending: false,
        };

        // Initialize rasterizer with default clip rect
        gpu.update_rasterizer_clip_rect();
        gpu
    }

    /// Reset GPU to initial state
    ///
    /// Clears all VRAM to black and resets all GPU state to default values.
    /// This is equivalent to a hardware reset.
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// gpu.write_vram(500, 250, 0xFFFF);
    /// gpu.reset();
    /// assert_eq!(gpu.read_vram(500, 250), 0x0000); // Back to black
    /// ```
    pub fn reset(&mut self) {
        // Reset all GPU state
        self.reset_state_preserving_vram();

        // Clear VRAM to black (separate from state reset)
        self.vram.fill(0x0000);
    }

    /// Reset GPU state without clearing VRAM
    ///
    /// Resets all GPU registers, drawing modes, display settings, command buffer,
    /// and status flags to their default values, but preserves VRAM contents.
    /// This is used by GP1(0x00) command which must not clear VRAM per PSX-SPX spec.
    pub(in crate::core::gpu) fn reset_state_preserving_vram(&mut self) {
        self.draw_mode = DrawMode::default();
        self.draw_area = DrawingArea::default();
        self.draw_offset = (0, 0);
        self.texture_window = TextureWindow::default();
        self.display_area = DisplayArea::default();
        self.display_mode = DisplayMode::default();
        self.command_fifo.clear();
        self.status = GPUStatus::default();
        self.vram_transfer = None;
        self.scanline = 0;
        self.dots = 0;
        self.in_vblank = false;
        self.in_hblank = false;
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
    /// use psrx::core::GPU;
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
    /// use psrx::core::GPU;
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
    pub(in crate::core::gpu) fn vram_index(&self, x: u16, y: u16) -> usize {
        // Wrap coordinates to valid VRAM bounds
        let x = (x & 0x3FF) as usize; // 0-1023
        let y = (y & 0x1FF) as usize; // 0-511
        y * Self::VRAM_WIDTH + x
    }

    /// Update the rasterizer's clipping rectangle from the drawing area
    ///
    /// This should be called whenever the drawing area is modified
    /// to keep the rasterizer's clip rect in sync.
    pub(in crate::core::gpu) fn update_rasterizer_clip_rect(&mut self) {
        self.rasterizer.set_clip_rect(
            self.draw_area.left as i16,
            self.draw_area.top as i16,
            self.draw_area.right as i16,
            self.draw_area.bottom as i16,
        );
    }

    /// Generate RGB24 framebuffer for display
    ///
    /// Extracts the display area from VRAM and converts it to 24-bit RGB
    /// format suitable for display. Each pixel is converted from 15-bit
    /// (5-5-5 RGB) to 24-bit (8-8-8 RGB) by left-shifting each channel.
    ///
    /// # Returns
    ///
    /// A Vec<u8> containing RGB24 data (width × height × 3 bytes).
    /// Pixels are in row-major order (left-to-right, top-to-bottom).
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// gpu.write_vram(10, 10, 0x7FFF); // White pixel
    ///
    /// let framebuffer = gpu.get_framebuffer();
    /// // framebuffer is 320 × 240 × 3 = 230,400 bytes
    /// assert_eq!(framebuffer.len(), 320 * 240 * 3);
    /// ```
    pub fn get_framebuffer(&self) -> Vec<u8> {
        let width = self.display_area.width as usize;
        let height = self.display_area.height as usize;
        let mut framebuffer = vec![0u8; width * height * 3];

        for y in 0..height {
            for x in 0..width {
                // Calculate VRAM coordinates with wrapping
                let vram_x = (self.display_area.x as usize + x) % 1024;
                let vram_y = (self.display_area.y as usize + y) % 512;
                let vram_index = vram_y * 1024 + vram_x;
                let pixel = self.vram[vram_index];

                // Convert 15-bit (5-5-5) to 24-bit (8-8-8) RGB
                // Left-shift by 3 to expand from 5-bit to 8-bit
                let r = ((pixel & 0x1F) << 3) as u8;
                let g = (((pixel >> 5) & 0x1F) << 3) as u8;
                let b = (((pixel >> 10) & 0x1F) << 3) as u8;

                let fb_index = (y * width + x) * 3;
                framebuffer[fb_index] = r;
                framebuffer[fb_index + 1] = g;
                framebuffer[fb_index + 2] = b;
            }
        }

        framebuffer
    }

    /// Get current GPU status register value
    ///
    /// Packs all GPU status flags into a 32-bit GPUSTAT register value
    /// that can be read from 0x1F801814.
    ///
    /// # Returns
    ///
    /// 32-bit GPU status register value
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

        // Bit 31: VBlank flag
        // Set to 1 when in VBlank period (scanlines 243-262 for NTSC)
        if self.in_vblank {
            status |= 1 << 31;
        }

        status
    }

    /// Get the display area configuration
    ///
    /// Returns the current display area settings which define the region of VRAM
    /// that is output to the display.
    ///
    /// # Returns
    ///
    /// Display area configuration (position and dimensions)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// let display_area = gpu.display_area();
    /// assert_eq!(display_area.width, 320);
    /// assert_eq!(display_area.height, 240);
    /// ```
    pub fn display_area(&self) -> DisplayArea {
        self.display_area
    }

    /// Get current VBlank status
    ///
    /// Returns true if the GPU is currently in the vertical blanking period.
    /// VBlank occurs during scanlines 243-262 in NTSC mode.
    ///
    /// # Returns
    ///
    /// true if in VBlank, false otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// assert_eq!(gpu.is_in_vblank(), false); // Initially not in VBlank
    /// ```
    pub fn is_in_vblank(&self) -> bool {
        self.in_vblank
    }

    /// Get current scanline number
    ///
    /// Returns the current scanline being rendered (0-262 for NTSC).
    ///
    /// # Returns
    ///
    /// Current scanline number
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let gpu = GPU::new();
    /// assert_eq!(gpu.get_scanline(), 0); // Initially at scanline 0
    /// ```
    pub fn get_scanline(&self) -> u16 {
        self.scanline
    }

    /// Tick GPU and update scanline/VBlank/HBlank state
    ///
    /// Advances the GPU state by the specified number of cycles, updating the scanline
    /// counter and generating VBlank/HBlank interrupt signals when appropriate.
    ///
    /// The PlayStation GPU operates on a scanline-based timing model:
    /// - Each scanline consists of 3413 dots
    /// - Each frame has 263 scanlines (NTSC)
    /// - VBlank occurs during scanlines 243-262
    /// - HBlank occurs at the end of each scanline
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles to advance
    ///
    /// # Returns
    ///
    /// A tuple `(vblank_interrupt, hblank_interrupt)` indicating whether interrupts
    /// should be triggered:
    /// - `vblank_interrupt`: true when VBlank starts (once per frame at scanline 243)
    /// - `hblank_interrupt`: true when a scanline completes (occurs every scanline)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    ///
    /// // Tick for one CPU cycle
    /// let (vblank, hblank) = gpu.tick(1);
    ///
    /// // Process interrupts
    /// if vblank {
    ///     // Handle VBlank interrupt
    /// }
    /// if hblank {
    ///     // Handle HBlank signal (for timers)
    /// }
    /// ```
    pub fn tick(&mut self, cycles: u32) -> (bool, bool) {
        let mut vblank_interrupt = false;
        let mut hblank_interrupt = false;

        for _ in 0..cycles {
            self.dots += 1;

            if self.dots >= Self::DOTS_PER_SCANLINE {
                self.dots = 0;
                self.scanline += 1;

                // HBlank occurs at end of each scanline
                hblank_interrupt = true;

                if self.scanline >= Self::SCANLINES_PER_FRAME {
                    self.scanline = 0;
                }

                // Check VBlank region
                let was_in_vblank = self.in_vblank;
                self.in_vblank =
                    self.scanline >= Self::VBLANK_START && self.scanline < Self::VBLANK_END;

                // VBlank interrupt at start of VBlank
                if self.in_vblank && !was_in_vblank {
                    vblank_interrupt = true;
                }
            }

            // HBlank during horizontal blanking period
            // (simplified: always false for now, can add proper timing later)
            self.in_hblank = false;
        }

        (vblank_interrupt, hblank_interrupt)
    }

    /// Process GP0 command (drawing and VRAM commands)
    ///
    /// GP0 commands handle drawing operations and VRAM transfers.
    /// Commands are buffered in a FIFO and processed when complete.
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit GP0 command word
    pub fn write_gp0(&mut self, value: u32) {
        // Log GP0 writes for debugging
        use std::sync::atomic::{AtomicU32, Ordering};
        static GP0_COUNT: AtomicU32 = AtomicU32::new(0);

        let count = GP0_COUNT.fetch_add(1, Ordering::Relaxed);
        if count < 20 || count.is_multiple_of(100) {
            let cmd = (value >> 24) & 0xFF;
            log::info!("GP0 write #{}: 0x{:08X} (cmd=0x{:02X})", count, value, cmd);
        }

        // If we're in the middle of a CPU→VRAM transfer, handle it
        if let Some(ref transfer) = self.vram_transfer {
            if transfer.direction == VRAMTransferDirection::CpuToVram {
                self.process_vram_write(value);
                return;
            }
        }

        // Otherwise, buffer the command
        self.command_fifo.push_back(value);

        // Try to process command
        self.try_process_command();
    }

    /// Try to process the next command in the FIFO
    ///
    /// Examines the command FIFO and attempts to process the next GP0 command
    /// if enough words have been received.
    fn try_process_command(&mut self) {
        if self.command_fifo.is_empty() {
            return;
        }

        let first_word = self.command_fifo[0];
        let command = (first_word >> 24) & 0xFF;

        match command {
            // Fill commands
            0x02 => self.gp0_fill_rectangle(),

            // Monochrome triangles
            0x20 => self.parse_monochrome_triangle_opaque(),
            0x22 => self.parse_monochrome_triangle_semi_transparent(),

            // Textured triangles
            0x24 => self.parse_textured_triangle_opaque(),
            0x26 => self.parse_textured_triangle_semi_transparent(),

            // Monochrome quadrilaterals
            0x28 => self.parse_monochrome_quad_opaque(),
            0x2A => self.parse_monochrome_quad_semi_transparent(),

            // Textured quadrilaterals
            0x2C => self.parse_textured_quad_opaque(),
            0x2E => self.parse_textured_quad_semi_transparent(),

            // Shaded triangles
            0x30 => self.parse_shaded_triangle_opaque(),
            0x32 => self.parse_shaded_triangle_semi_transparent(),

            // Shaded quads
            0x38 => self.parse_shaded_quad_opaque(),
            0x3A => self.parse_shaded_quad_semi_transparent(),

            // Lines (monochrome)
            0x40 => self.parse_line_opaque(),
            0x42 => self.parse_line_semi_transparent(),

            // Polylines (monochrome)
            0x48 => self.parse_polyline_opaque(),
            0x4A => self.parse_polyline_semi_transparent(),

            // Lines (shaded/Gouraud)
            0x50 => self.parse_shaded_line_opaque(),
            0x52 => self.parse_shaded_line_semi_transparent(),

            // Polylines (shaded/Gouraud)
            0x58 => self.parse_shaded_polyline_opaque(),
            0x5A => self.parse_shaded_polyline_semi_transparent(),

            // Monochrome rectangles
            0x60 => self.parse_monochrome_rect_variable_opaque(),
            0x62 => self.parse_monochrome_rect_variable_semi_transparent(),
            0x68 => self.parse_monochrome_rect_1x1_opaque(),
            0x6A => self.parse_monochrome_rect_1x1_semi_transparent(),
            0x70 => self.parse_monochrome_rect_8x8_opaque(),
            0x72 => self.parse_monochrome_rect_8x8_semi_transparent(),
            0x78 => self.parse_monochrome_rect_16x16_opaque(),
            0x7A => self.parse_monochrome_rect_16x16_semi_transparent(),

            // Textured rectangles (variable size)
            0x64 => self.parse_textured_rect_variable_opaque(),
            0x65 => self.parse_textured_rect_variable_opaque_modulated(),
            0x66 => self.parse_textured_rect_variable_semi_transparent(),
            0x67 => self.parse_textured_rect_variable_semi_transparent_modulated(),

            // Textured rectangles (1×1)
            0x6C => self.parse_textured_rect_1x1_opaque(),
            0x6D => self.parse_textured_rect_1x1_opaque_modulated(),
            0x6E => self.parse_textured_rect_1x1_semi_transparent(),
            0x6F => self.parse_textured_rect_1x1_semi_transparent_modulated(),

            // Textured rectangles (8×8)
            0x74 => self.parse_textured_rect_8x8_opaque(),
            0x75 => self.parse_textured_rect_8x8_opaque_modulated(),
            0x76 => self.parse_textured_rect_8x8_semi_transparent(),
            0x77 => self.parse_textured_rect_8x8_semi_transparent_modulated(),

            // Textured rectangles (16×16)
            0x7C => self.parse_textured_rect_16x16_opaque(),
            0x7D => self.parse_textured_rect_16x16_opaque_modulated(),
            0x7E => self.parse_textured_rect_16x16_semi_transparent(),
            0x7F => self.parse_textured_rect_16x16_semi_transparent_modulated(),

            // VRAM transfer commands
            0xA0 => self.gp0_cpu_to_vram_transfer(),
            0xC0 => self.gp0_vram_to_cpu_transfer(),
            0x80 => self.gp0_vram_to_vram_transfer(),

            // Drawing mode settings
            0xE1 => self.gp0_draw_mode(),
            0xE2 => self.gp0_texture_window(),
            0xE3 => self.gp0_draw_area_top_left(),
            0xE4 => self.gp0_draw_area_bottom_right(),
            0xE5 => self.gp0_draw_offset(),
            0xE6 => self.gp0_mask_settings(),

            _ => {
                log::warn!("Unimplemented GP0 command: 0x{:02X}", command);
                self.command_fifo.pop_front();
            }
        }
    }

    /// Read from GPUREAD register (0x1F801810)
    ///
    /// Returns pixel data during VRAM→CPU transfers. Each read returns
    /// two 16-bit pixels packed into a 32-bit word.
    pub fn read_gpuread(&mut self) -> u32 {
        // Extract transfer state to avoid borrowing issues
        let mut transfer = match self.vram_transfer.take() {
            Some(t) => t,
            None => return 0,
        };

        // Read two pixels and pack into u32
        let vram_x1 = (transfer.x + transfer.current_x) & 0x3FF;
        let vram_y1 = (transfer.y + transfer.current_y) & 0x1FF;
        let pixel1 = self.read_vram(vram_x1, vram_y1);

        transfer.current_x += 1;
        if transfer.current_x >= transfer.width {
            transfer.current_x = 0;
            transfer.current_y += 1;
        }

        let pixel2 = if transfer.current_y < transfer.height {
            let vram_x2 = (transfer.x + transfer.current_x) & 0x3FF;
            let vram_y2 = (transfer.y + transfer.current_y) & 0x1FF;
            let p = self.read_vram(vram_x2, vram_y2);

            transfer.current_x += 1;
            if transfer.current_x >= transfer.width {
                transfer.current_x = 0;
                transfer.current_y += 1;
            }

            p
        } else {
            0
        };

        // Check if complete
        if transfer.current_y >= transfer.height {
            self.status.ready_to_send_vram = false;
            log::debug!("VRAM→CPU transfer complete");
        } else {
            self.vram_transfer = Some(transfer);
        }

        (pixel1 as u32) | ((pixel2 as u32) << 16)
    }

    /// Process GP1 command (control commands)
    pub fn write_gp1(&mut self, value: u32) {
        let command = (value >> 24) & 0xFF;

        match command {
            0x00 => self.gp1_reset_gpu(),
            0x01 => self.gp1_reset_command_buffer(),
            0x02 => self.gp1_acknowledge_interrupt(),
            0x03 => self.gp1_display_enable(value),
            0x04 => self.gp1_dma_direction(value),
            0x05 => self.gp1_display_area_start(value),
            0x06 => self.gp1_horizontal_display_range(value),
            0x07 => self.gp1_vertical_display_range(value),
            0x08 => self.gp1_display_mode(value),
            0x10 => self.gp1_get_gpu_info(value),
            _ => {
                log::warn!("Unknown GP1 command: 0x{:02X}", command);
            }
        }
    }

    /// Register timing events for GPU VBlank and HBlank
    ///
    /// This should be called during system initialization to register GPU
    /// timing events with the timing manager.
    ///
    /// # Arguments
    ///
    /// * `timing` - Timing event manager
    pub fn register_events(&mut self, timing: &mut super::timing::TimingEventManager) {
        // VBlank event: fires every frame (~16.67ms for NTSC 60Hz)
        // Total cycles per frame = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME
        // = 3413 * 263 = 897,619 cycles
        // But PSX CPU runs at 33.8688 MHz, and at 60fps that's 564,480 cycles/frame
        // Using the standard PSX timing
        const CYCLES_PER_FRAME: i32 = 564_480;

        self.vblank_event = Some(timing.register_periodic_event("GPU VBlank", CYCLES_PER_FRAME));

        // Activate VBlank event
        if let Some(handle) = self.vblank_event {
            timing.schedule(handle, CYCLES_PER_FRAME);
        }

        // HBlank event: fires every scanline
        // = CYCLES_PER_FRAME / SCANLINES_PER_FRAME = 564_480 / 263 ≈ 2146 cycles
        const CYCLES_PER_SCANLINE: i32 = 2_146;

        self.hblank_event = Some(timing.register_periodic_event("GPU HBlank", CYCLES_PER_SCANLINE));

        // Activate HBlank event
        if let Some(handle) = self.hblank_event {
            timing.schedule(handle, CYCLES_PER_SCANLINE);
        }

        log::info!(
            "GPU: Timing events registered and activated (VBlank={} cycles, HBlank={} cycles)",
            CYCLES_PER_FRAME,
            CYCLES_PER_SCANLINE
        );
    }

    /// Process GPU timing events
    ///
    /// This should be called by System when timing events fire.
    ///
    /// # Arguments
    ///
    /// * `timing` - Timing event manager
    /// * `triggered_events` - List of event handles that have fired
    pub fn process_events(
        &mut self,
        _timing: &mut super::timing::TimingEventManager,
        triggered_events: &[EventHandle],
    ) {
        // Check if VBlank event fired
        if let Some(handle) = self.vblank_event {
            if triggered_events.contains(&handle) {
                self.vblank_callback();
            }
        }

        // Check if HBlank event fired
        if let Some(handle) = self.hblank_event {
            if triggered_events.contains(&handle) {
                self.hblank_callback();
            }
        }
    }

    /// VBlank callback (called when vblank_event fires)
    ///
    /// Triggered at the start of vertical blanking period (scanline 243).
    fn vblank_callback(&mut self) {
        // Reset scanline counter at start of VBlank
        self.scanline = Self::VBLANK_START;
        self.in_vblank = true;
        self.dots = 0;

        // Set VBlank interrupt pending
        self.vblank_interrupt_pending = true;

        log::trace!("GPU: VBlank");
    }

    /// HBlank callback (called when hblank_event fires)
    ///
    /// Triggered at the end of each scanline.
    fn hblank_callback(&mut self) {
        self.scanline += 1;
        self.dots = 0;

        // Wrap scanline counter
        if self.scanline >= Self::SCANLINES_PER_FRAME {
            self.scanline = 0;
        }

        // Update VBlank status
        self.in_vblank = self.scanline >= Self::VBLANK_START && self.scanline < Self::VBLANK_END;

        // Set HBlank interrupt pending (always signal for timer synchronization)
        self.hblank_interrupt_pending = true;

        log::trace!("GPU: HBlank (scanline {})", self.scanline);
    }

    /// Poll GPU interrupt flags
    ///
    /// Returns interrupt flags and clears them.
    /// Replaces the return value of tick() for event-driven timing.
    ///
    /// # Returns
    ///
    /// Tuple of (vblank_interrupt, hblank_interrupt)
    pub fn poll_interrupts(&mut self) -> (bool, bool) {
        let vblank = self.vblank_interrupt_pending;
        let hblank = self.hblank_interrupt_pending;

        self.vblank_interrupt_pending = false;
        self.hblank_interrupt_pending = false;

        (vblank, hblank)
    }
}

impl Default for GPU {
    fn default() -> Self {
        Self::new()
    }
}

/// ⚠️ **UNUSED PREPARATORY CODE - NOT CURRENTLY INVOKED** ⚠️
///
/// This IODevice trait implementation for GPU is **preparatory code** that exists for
/// future Phase 2 work. It is NOT used by any current code path.
///
/// **Current Implementation**: The Bus directly calls GPU methods via
/// `gpu.borrow_mut().read_gpuread()` / `write_gp0()` / `write_gp1()`. This provides
/// proper mutable access and works correctly.
///
/// **Future Work (Phase 2+)**: Before switching the Bus to trait-based device routing,
/// the GPUREAD limitation below MUST be resolved via one of these approaches:
/// - Change IODevice trait signature to allow `read_register(&mut self)`
/// - Use interior mutability (RefCell/Mutex) for VRAM transfer state
/// - Implement a small IO wrapper type around GPU that handles mutability
///
/// ## GPU Register Layout
///
/// The GPU has two 32-bit registers accessible via memory-mapped I/O:
/// - Offset 0x00 (0x1F801810): GP0 (write) / GPUREAD (read)
/// - Offset 0x04 (0x1F801814): GP1 (write) / GPUSTAT (read)
///
/// ## Known Limitations (To Be Fixed Before Activation)
///
/// - **GPUREAD register returns status instead of transfer data**: The `read_gpuread()`
///   method requires `&mut self` to consume VRAM→CPU transfer data, but the IODevice
///   trait's `read_register()` only provides `&self`. The current workaround returns
///   `status()` with a warning. **This MUST be fixed before activating trait-based
///   routing**, otherwise VRAM-to-CPU transfers will not work.
/// - **No runtime impact currently**: Since this implementation is unused, the limitation
///   doesn't affect emulation accuracy or behavior.
#[allow(dead_code)]
impl crate::core::memory::IODevice for GPU {
    fn address_range(&self) -> (u32, u32) {
        // GPU registers: 0x1F801810 - 0x1F801817 (8 bytes, 2 registers)
        (0x1F801810, 0x1F801817)
    }

    fn read_register(&self, offset: u32) -> crate::core::error::Result<u32> {
        match offset {
            // GPUREAD register (0x1F801810, offset 0x00)
            0x00 => {
                // GPUREAD requires &mut self, but trait uses &self
                // This is a limitation - we'll need to handle this specially
                // For now, log a warning and return status
                log::warn!("GPUREAD access via IODevice trait (requires mutable access)");
                Ok(self.status())
            }

            // GPUSTAT register (0x1F801814, offset 0x04)
            0x04 => Ok(self.status()),

            // Invalid offset
            _ => {
                log::warn!("Invalid GPU register read at offset 0x{:02X}", offset);
                Ok(0xFFFFFFFF)
            }
        }
    }

    fn write_register(&mut self, offset: u32, value: u32) -> crate::core::error::Result<()> {
        match offset {
            // GP0 register (0x1F801810, offset 0x00)
            0x00 => {
                log::trace!("GPU GP0 write: 0x{:08X}", value);
                self.write_gp0(value);
                Ok(())
            }

            // GP1 register (0x1F801814, offset 0x04)
            0x04 => {
                log::trace!("GPU GP1 write: 0x{:08X}", value);
                self.write_gp1(value);
                Ok(())
            }

            // Invalid offset
            _ => {
                log::warn!(
                    "Invalid GPU register write at offset 0x{:02X} = 0x{:08X}",
                    offset,
                    value
                );
                Ok(())
            }
        }
    }

    fn name(&self) -> &str {
        "GPU"
    }
}
