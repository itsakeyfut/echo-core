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

/// A 24-bit RGB color used in GPU commands
///
/// PlayStation GPU commands use 24-bit RGB colors (8 bits per channel)
/// which are converted to 15-bit RGB for VRAM storage.
///
/// # Examples
///
/// ```
/// use psrx::core::Color;
///
/// let color = Color::from_u32(0x00FF8040);
/// assert_eq!(color.r, 0x40);
/// assert_eq!(color.g, 0x80);
/// assert_eq!(color.b, 0xFF);
///
/// let rgb15 = color.to_rgb15();
/// assert_eq!(rgb15 & 0x1F, 0x08); // Red: 0x40 >> 3 = 8
/// assert_eq!((rgb15 >> 5) & 0x1F, 0x10); // Green: 0x80 >> 3 = 16
/// assert_eq!((rgb15 >> 10) & 0x1F, 0x1F); // Blue: 0xFF >> 3 = 31
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red channel (0-255)
    pub r: u8,
    /// Green channel (0-255)
    pub g: u8,
    /// Blue channel (0-255)
    pub b: u8,
}

impl Color {
    /// Create a Color from a 32-bit command word
    ///
    /// The color is encoded in the lower 24 bits:
    /// - Bits 0-7: Red
    /// - Bits 8-15: Green
    /// - Bits 16-23: Blue
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit word containing RGB color in bits 0-23
    ///
    /// # Returns
    ///
    /// Color struct with 8-bit RGB values
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::Color;
    ///
    /// let color = Color::from_u32(0xFF8040);
    /// assert_eq!(color.r, 0x40);
    /// assert_eq!(color.g, 0x80);
    /// assert_eq!(color.b, 0xFF);
    /// ```
    pub fn from_u32(value: u32) -> Self {
        Self {
            r: (value & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: ((value >> 16) & 0xFF) as u8,
        }
    }

    /// Convert 24-bit RGB to 15-bit RGB format for VRAM
    ///
    /// Converts each 8-bit channel to 5-bit by right-shifting by 3.
    /// The result is packed in VRAM's 5-5-5 RGB format:
    /// - Bits 0-4: Red (5 bits)
    /// - Bits 5-9: Green (5 bits)
    /// - Bits 10-14: Blue (5 bits)
    ///
    /// # Returns
    ///
    /// 16-bit color value in 5-5-5 RGB format (bit 15 is 0)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::Color;
    ///
    /// let color = Color { r: 255, g: 128, b: 64 };
    /// let rgb15 = color.to_rgb15();
    ///
    /// assert_eq!(rgb15 & 0x1F, 31);        // R: 255 >> 3 = 31
    /// assert_eq!((rgb15 >> 5) & 0x1F, 16); // G: 128 >> 3 = 16
    /// assert_eq!((rgb15 >> 10) & 0x1F, 8); // B: 64 >> 3 = 8
    /// ```
    pub fn to_rgb15(&self) -> u16 {
        let r = ((self.r as u16) >> 3) & 0x1F;
        let g = ((self.g as u16) >> 3) & 0x1F;
        let b = ((self.b as u16) >> 3) & 0x1F;
        (b << 10) | (g << 5) | r
    }
}

/// A 2D vertex position used in polygon rendering
///
/// Vertices specify positions in VRAM coordinates (signed 16-bit).
/// Negative coordinates and coordinates outside VRAM bounds are clipped.
///
/// # Coordinate System
///
/// - Origin (0, 0) is at top-left
/// - X increases to the right (0-1023)
/// - Y increases downward (0-511)
/// - Drawing offset is added to vertex positions before rendering
///
/// # Examples
///
/// ```
/// use psrx::core::Vertex;
///
/// let vertex = Vertex::from_u32(0x00640032);
/// assert_eq!(vertex.x, 50);
/// assert_eq!(vertex.y, 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vertex {
    /// X coordinate (signed 16-bit)
    pub x: i16,
    /// Y coordinate (signed 16-bit)
    pub y: i16,
}

impl Vertex {
    /// Create a Vertex from a 32-bit command word
    ///
    /// Vertices are encoded as:
    /// - Bits 0-15: X coordinate (signed 16-bit)
    /// - Bits 16-31: Y coordinate (signed 16-bit)
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit word containing X and Y coordinates
    ///
    /// # Returns
    ///
    /// Vertex struct with signed 16-bit coordinates
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::Vertex;
    ///
    /// let v = Vertex::from_u32(0x00640032);
    /// assert_eq!(v.x, 50);
    /// assert_eq!(v.y, 100);
    /// ```
    pub fn from_u32(value: u32) -> Self {
        let x = (value & 0xFFFF) as i16;
        let y = ((value >> 16) & 0xFFFF) as i16;
        Self { x, y }
    }
}

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

    /// 384 pixels wide
    R384,
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

/// VRAM transfer direction
///
/// Indicates whether a VRAM transfer is uploading from CPU to VRAM or downloading from VRAM to CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VRAMTransferDirection {
    /// CPU→VRAM transfer (GP0 0xA0)
    CpuToVram,
    /// VRAM→CPU transfer (GP0 0xC0)
    VramToCpu,
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

    /// Transfer direction
    pub direction: VRAMTransferDirection,
}

/// GPU rendering command
///
/// Represents a fully parsed GPU drawing command ready for execution.
/// These commands are created by parsing GP0 command sequences.
///
/// # Polygon Rendering
///
/// Polygons are the fundamental 3D rendering primitive. The PSX GPU supports:
/// - **Monochrome (flat-shaded)**: Single color for entire polygon
/// - **Gouraud-shaded**: Color interpolated across vertices (future)
/// - **Textured**: Textured polygons (future)
///
/// Quadrilaterals are rendered as two triangles internally.
///
/// # Command Format
///
/// GP0 polygon commands encode data as follows:
/// - Word 0: Command byte (bits 24-31) + Color (bits 0-23)
/// - Word 1-N: Vertex positions (X in bits 0-15, Y in bits 16-31)
///
/// # Examples
///
/// ```
/// use psrx::core::{GPUCommand, Vertex, Color};
///
/// let cmd = GPUCommand::MonochromeTriangle {
///     vertices: [
///         Vertex { x: 0, y: 0 },
///         Vertex { x: 100, y: 0 },
///         Vertex { x: 50, y: 100 },
///     ],
///     color: Color { r: 255, g: 0, b: 0 },
///     semi_transparent: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub enum GPUCommand {
    /// Monochrome (flat-shaded) triangle
    ///
    /// GP0 commands: 0x20 (opaque), 0x22 (semi-transparent)
    /// Requires 4 words: command + 3 vertices
    MonochromeTriangle {
        /// Triangle vertices (3 points)
        vertices: [Vertex; 3],
        /// Flat color for entire triangle
        color: Color,
        /// Semi-transparency enabled
        semi_transparent: bool,
    },

    /// Monochrome (flat-shaded) quadrilateral
    ///
    /// GP0 commands: 0x28 (opaque), 0x2A (semi-transparent)
    /// Requires 5 words: command + 4 vertices
    /// Rendered as two triangles: (v0,v1,v2) and (v1,v2,v3)
    MonochromeQuad {
        /// Quad vertices (4 points in order)
        vertices: [Vertex; 4],
        /// Flat color for entire quad
        color: Color,
        /// Semi-transparency enabled
        semi_transparent: bool,
    },

    /// Gouraud-shaded triangle (color per vertex)
    ///
    /// GP0 commands: 0x30 (opaque), 0x32 (semi-transparent)
    /// Requires 6 words: (command+color1, vertex1, color2, vertex2, color3, vertex3)
    /// Future implementation - placeholder for issue #33
    ShadedTriangle {
        /// Triangle vertices with colors
        vertices: [(Vertex, Color); 3],
        /// Semi-transparency enabled
        semi_transparent: bool,
    },

    /// Gouraud-shaded quadrilateral (color per vertex)
    ///
    /// GP0 commands: 0x38 (opaque), 0x3A (semi-transparent)
    /// Requires 8 words: 4×(command+color, vertex)
    /// Future implementation - placeholder for issue #33
    ShadedQuad {
        /// Quad vertices with colors
        vertices: [(Vertex, Color); 4],
        /// Semi-transparency enabled
        semi_transparent: bool,
    },
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
    /// use psrx::core::GPU;
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
    fn reset_state_preserving_vram(&mut self) {
        self.draw_mode = DrawMode::default();
        self.draw_area = DrawingArea::default();
        self.draw_offset = (0, 0);
        self.texture_window = TextureWindow::default();
        self.display_area = DisplayArea::default();
        self.display_mode = DisplayMode::default();
        self.command_fifo.clear();
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

    /// Process GP0 command (drawing and VRAM commands)
    ///
    /// GP0 commands handle drawing operations and VRAM transfers.
    /// Commands are buffered in a FIFO and processed when complete.
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit GP0 command word
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// // Start CPU→VRAM transfer
    /// gpu.write_gp0(0xA0000000);
    /// gpu.write_gp0(0x0000000A);  // position
    /// gpu.write_gp0(0x00020002);  // size
    /// ```
    pub fn write_gp0(&mut self, value: u32) {
        // If we're in the middle of a CPU→VRAM transfer, handle it
        if let Some(ref transfer) = self.vram_transfer {
            if transfer.direction == VRAMTransferDirection::CpuToVram {
                self.process_vram_write(value);
                return;
            }
            // VRAM→CPU transfer is in progress; ignore stray GP0 writes
            // (CPU should be reading from GPUREAD instead)
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
            // Monochrome triangles
            0x20 => self.parse_monochrome_triangle_opaque(),
            0x22 => self.parse_monochrome_triangle_semi_transparent(),

            // Monochrome quadrilaterals
            0x28 => self.parse_monochrome_quad_opaque(),
            0x2A => self.parse_monochrome_quad_semi_transparent(),

            // Shaded triangles (placeholders for issue #33)
            0x30 => self.parse_shaded_triangle_opaque(),
            0x32 => self.parse_shaded_triangle_semi_transparent(),

            // Shaded quads (placeholders for issue #33)
            0x38 => self.parse_shaded_quad_opaque(),
            0x3A => self.parse_shaded_quad_semi_transparent(),

            // VRAM transfer commands
            0xA0 => self.gp0_cpu_to_vram_transfer(),
            0xC0 => self.gp0_vram_to_cpu_transfer(),
            0x80 => self.gp0_vram_to_vram_transfer(),

            // Other commands will be implemented in later issues
            _ => {
                log::warn!("Unimplemented GP0 command: 0x{:02X}", command);
                // Remove unknown command to prevent stalling
                self.command_fifo.pop_front();
            }
        }
    }

    /// GP0(0xA0): CPU→VRAM Transfer
    ///
    /// Initiates a transfer from CPU to VRAM. The transfer requires 3 command words:
    /// - Word 0: Command (0xA0000000)
    /// - Word 1: Destination coordinates (X in bits 0-15, Y in bits 16-31)
    /// - Word 2: Size (Width in bits 0-15, Height in bits 16-31)
    ///
    /// After these words, subsequent GP0 writes are treated as pixel data (2 pixels per word).
    fn gp0_cpu_to_vram_transfer(&mut self) {
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
    fn process_vram_write(&mut self, value: u32) {
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
    fn gp0_vram_to_cpu_transfer(&mut self) {
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

    /// Read from GPUREAD register (0x1F801810)
    ///
    /// Returns pixel data during VRAM→CPU transfers. Each read returns
    /// two 16-bit pixels packed into a 32-bit word.
    ///
    /// # Returns
    ///
    /// 32-bit word containing two pixels (pixel1 in bits 0-15, pixel2 in bits 16-31)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// gpu.write_vram(100, 100, 0x1234);
    /// gpu.write_vram(101, 100, 0x5678);
    ///
    /// // Start VRAM→CPU transfer
    /// gpu.write_gp0(0xC0000000);
    /// gpu.write_gp0(0x00640064);  // position (100, 100)
    /// gpu.write_gp0(0x00010002);  // size 2×1
    ///
    /// let data = gpu.read_gpuread();
    /// assert_eq!(data & 0xFFFF, 0x1234);
    /// assert_eq!((data >> 16) & 0xFFFF, 0x5678);
    /// ```
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
            // Transfer is complete, don't restore it
        } else {
            // Restore transfer state for next read
            self.vram_transfer = Some(transfer);
        }

        (pixel1 as u32) | ((pixel2 as u32) << 16)
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
    fn gp0_vram_to_vram_transfer(&mut self) {
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

    /// GP0(0x20): Monochrome Triangle (Opaque)
    ///
    /// Renders a flat-shaded triangle with a single color.
    /// Requires 4 words: command+color, vertex1, vertex2, vertex3
    fn parse_monochrome_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 4 {
            return; // Need more words
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_monochrome_triangle(&vertices, &color, false);
    }

    /// GP0(0x22): Monochrome Triangle (Semi-Transparent)
    ///
    /// Renders a flat-shaded triangle with semi-transparency enabled.
    /// Requires 4 words: command+color, vertex1, vertex2, vertex3
    fn parse_monochrome_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_monochrome_triangle(&vertices, &color, true);
    }

    /// GP0(0x28): Monochrome Quad (Opaque)
    ///
    /// Renders a flat-shaded quadrilateral with a single color.
    /// Requires 5 words: command+color, vertex1, vertex2, vertex3, vertex4
    fn parse_monochrome_quad_opaque(&mut self) {
        if self.command_fifo.len() < 5 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_monochrome_quad(&vertices, &color, false);
    }

    /// GP0(0x2A): Monochrome Quad (Semi-Transparent)
    ///
    /// Renders a flat-shaded quadrilateral with semi-transparency enabled.
    /// Requires 5 words: command+color, vertex1, vertex2, vertex3, vertex4
    fn parse_monochrome_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 5 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_monochrome_quad(&vertices, &color, true);
    }

    /// GP0(0x30): Gouraud-Shaded Triangle (Opaque) - Placeholder
    ///
    /// Will be implemented in issue #33
    fn parse_shaded_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..6 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded triangle rendering not yet implemented (issue #33)");
    }

    /// GP0(0x32): Gouraud-Shaded Triangle (Semi-Transparent) - Placeholder
    ///
    /// Will be implemented in issue #33
    fn parse_shaded_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..6 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded triangle rendering not yet implemented (issue #33)");
    }

    /// GP0(0x38): Gouraud-Shaded Quad (Opaque) - Placeholder
    ///
    /// Will be implemented in issue #33
    fn parse_shaded_quad_opaque(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..8 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded quad rendering not yet implemented (issue #33)");
    }

    /// GP0(0x3A): Gouraud-Shaded Quad (Semi-Transparent) - Placeholder
    ///
    /// Will be implemented in issue #33
    fn parse_shaded_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..8 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded quad rendering not yet implemented (issue #33)");
    }

    /// Render a monochrome (flat-shaded) triangle
    ///
    /// Applies the drawing offset to all vertices and logs the rendering operation.
    /// Actual rasterization will be implemented in issue #33.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `color` - Flat color for the entire triangle
    /// * `semi_transparent` - Whether semi-transparency is enabled
    fn render_monochrome_triangle(
        &mut self,
        vertices: &[Vertex; 3],
        color: &Color,
        semi_transparent: bool,
    ) {
        // Apply drawing offset
        let v0 = Vertex {
            x: vertices[0].x + self.draw_offset.0,
            y: vertices[0].y + self.draw_offset.1,
        };
        let v1 = Vertex {
            x: vertices[1].x + self.draw_offset.0,
            y: vertices[1].y + self.draw_offset.1,
        };
        let v2 = Vertex {
            x: vertices[2].x + self.draw_offset.0,
            y: vertices[2].y + self.draw_offset.1,
        };

        log::trace!(
            "Rendering {}triangle: ({}, {}), ({}, {}), ({}, {}) color=({},{},{})",
            if semi_transparent {
                "semi-transparent "
            } else {
                ""
            },
            v0.x,
            v0.y,
            v1.x,
            v1.y,
            v2.x,
            v2.y,
            color.r,
            color.g,
            color.b
        );

        // Actual rasterization will be implemented in issue #33
        // For now, just log the command
    }

    /// Render a monochrome (flat-shaded) quadrilateral
    ///
    /// Quads are rendered as two triangles: (v0, v1, v2) and (v1, v2, v3).
    /// Applies the drawing offset and delegates to triangle rendering.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 4 vertices defining the quad (in order)
    /// * `color` - Flat color for the entire quad
    /// * `semi_transparent` - Whether semi-transparency is enabled
    fn render_monochrome_quad(
        &mut self,
        vertices: &[Vertex; 4],
        color: &Color,
        semi_transparent: bool,
    ) {
        // Quads are rendered as two triangles
        let tri1 = [vertices[0], vertices[1], vertices[2]];
        let tri2 = [vertices[1], vertices[2], vertices[3]];

        self.render_monochrome_triangle(&tri1, color, semi_transparent);
        self.render_monochrome_triangle(&tri2, color, semi_transparent);
    }

    /// Process GP1 command (control commands)
    ///
    /// GP1 commands control the GPU's display parameters and operational state.
    /// These commands configure display settings, DMA modes, and GPU state.
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit GP1 command word with command in bits 24-31
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::GPU;
    ///
    /// let mut gpu = GPU::new();
    /// // Reset GPU
    /// gpu.write_gp1(0x00000000);
    /// // Enable display
    /// gpu.write_gp1(0x03000000);
    /// ```
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

    /// GP1(0x00): Reset GPU
    ///
    /// Resets the GPU to its initial power-on state. This includes:
    /// - Resetting all GPU registers and state to defaults
    /// - Clearing the command buffer
    /// - Disabling the display
    /// - Clearing texture settings
    ///
    /// Note: VRAM contents are preserved per PSX-SPX specification.
    fn gp1_reset_gpu(&mut self) {
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
    fn gp1_reset_command_buffer(&mut self) {
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
    fn gp1_acknowledge_interrupt(&mut self) {
        self.status.interrupt_request = false;
        log::debug!("GPU interrupt acknowledged");
    }

    /// GP1(0x03): Display Enable
    ///
    /// Enables or disables the display output.
    ///
    /// # Arguments
    ///
    /// * `value` - Bit 0: 0=Enable, 1=Disable (inverted logic)
    fn gp1_display_enable(&mut self, value: u32) {
        let enabled = (value & 1) == 0;
        self.display_mode.display_disabled = !enabled;
        self.status.display_disabled = !enabled;

        log::debug!("Display {}", if enabled { "enabled" } else { "disabled" });
    }

    /// GP1(0x04): DMA Direction
    ///
    /// Sets the DMA transfer direction/mode.
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-1: Direction (0=Off, 1=FIFO, 2=CPUtoGP0, 3=GPUREADtoCPU)
    fn gp1_dma_direction(&mut self, value: u32) {
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

    /// GP1(0x05): Start of Display Area
    ///
    /// Sets the top-left corner of the display area in VRAM.
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-9: X coordinate, Bits 10-18: Y coordinate
    fn gp1_display_area_start(&mut self, value: u32) {
        let x = (value & 0x3FF) as u16;
        let y = ((value >> 10) & 0x1FF) as u16;

        self.display_area.x = x;
        self.display_area.y = y;

        log::debug!("Display area start: ({}, {})", x, y);
    }

    /// GP1(0x06): Horizontal Display Range
    ///
    /// Sets the horizontal display range on screen (scanline timing).
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-11: X1 start, Bits 12-23: X2 end
    fn gp1_horizontal_display_range(&mut self, value: u32) {
        let x1 = (value & 0xFFF) as u16;
        let x2 = ((value >> 12) & 0xFFF) as u16;

        // Store as width
        self.display_area.width = x2.saturating_sub(x1);

        log::debug!(
            "Horizontal display range: {} to {} (width: {})",
            x1,
            x2,
            self.display_area.width
        );
    }

    /// GP1(0x07): Vertical Display Range
    ///
    /// Sets the vertical display range on screen (scanline timing).
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-9: Y1 start, Bits 10-19: Y2 end
    fn gp1_vertical_display_range(&mut self, value: u32) {
        let y1 = (value & 0x3FF) as u16;
        let y2 = ((value >> 10) & 0x3FF) as u16;

        // Store as height
        self.display_area.height = y2.saturating_sub(y1);

        log::debug!(
            "Vertical display range: {} to {} (height: {})",
            y1,
            y2,
            self.display_area.height
        );
    }

    /// GP1(0x08): Display Mode
    ///
    /// Sets the display mode including resolution, video mode, and color depth.
    ///
    /// # Arguments
    ///
    /// * `value` - Display mode configuration bits:
    ///   - Bits 0-1: Horizontal resolution 1
    ///   - Bit 2: Vertical resolution (0=240, 1=480)
    ///   - Bit 3: Video mode (0=NTSC, 1=PAL)
    ///   - Bit 4: Color depth (0=15bit, 1=24bit)
    ///   - Bit 5: Interlace (0=Off, 1=On)
    ///   - Bit 6: Horizontal resolution 2
    ///   - Bit 7: Reverse flag
    fn gp1_display_mode(&mut self, value: u32) {
        // Horizontal resolution
        let hr1 = (value & 3) as u8;
        let hr2 = ((value >> 6) & 1) as u8;
        self.display_mode.horizontal_res = match (hr2, hr1) {
            (0, 0) => HorizontalRes::R256,
            (0, 1) => HorizontalRes::R320,
            (0, 2) => HorizontalRes::R512,
            (0, 3) => HorizontalRes::R640,
            (1, 0) => HorizontalRes::R368,
            (1, 1) => HorizontalRes::R384,
            (1, _) => HorizontalRes::R368, // Reserved combinations default to 368
            _ => HorizontalRes::R320,
        };

        // Update status register horizontal resolution bits
        self.status.horizontal_res_1 = hr1;
        self.status.horizontal_res_2 = hr2;

        // Vertical resolution
        let vres = ((value >> 2) & 1) != 0;
        self.display_mode.vertical_res = if vres {
            VerticalRes::R480
        } else {
            VerticalRes::R240
        };
        self.status.vertical_res = vres;

        // Video mode (NTSC/PAL)
        let video_mode = ((value >> 3) & 1) != 0;
        self.display_mode.video_mode = if video_mode {
            VideoMode::PAL
        } else {
            VideoMode::NTSC
        };
        self.status.video_mode = video_mode;

        // Color depth
        let color_depth = ((value >> 4) & 1) != 0;
        self.display_mode.display_area_color_depth = if color_depth {
            ColorDepth::C24Bit
        } else {
            ColorDepth::C15Bit
        };
        self.status.display_area_color_depth = color_depth;

        // Interlace
        let interlaced = ((value >> 5) & 1) != 0;
        self.display_mode.interlaced = interlaced;
        self.status.vertical_interlace = interlaced;

        // Reverse flag (rarely used)
        self.status.reverse_flag = ((value >> 7) & 1) != 0;

        log::debug!(
            "Display mode: {:?} {:?} {:?} {:?} interlaced={}",
            self.display_mode.horizontal_res,
            self.display_mode.vertical_res,
            self.display_mode.video_mode,
            self.display_mode.display_area_color_depth,
            interlaced
        );
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
    fn gp1_get_gpu_info(&mut self, value: u32) {
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

    // GP1 Command Tests

    #[test]
    fn test_gp1_reset_gpu() {
        let mut gpu = GPU::new();
        gpu.display_mode.display_disabled = false;
        gpu.status.display_disabled = false;
        gpu.command_fifo.push_back(0x12345678);

        gpu.write_gp1(0x00000000);

        assert!(gpu.display_mode.display_disabled);
        assert!(gpu.status.display_disabled);
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_gp1_reset_preserves_vram() {
        let mut gpu = GPU::new();

        // Write some data to VRAM
        gpu.write_vram(100, 100, 0xABCD);
        gpu.write_vram(500, 250, 0x1234);
        gpu.write_vram(1023, 511, 0x5678);

        // Reset via GP1 command
        gpu.write_gp1(0x00000000);

        // VRAM should be preserved (not cleared)
        assert_eq!(gpu.read_vram(100, 100), 0xABCD);
        assert_eq!(gpu.read_vram(500, 250), 0x1234);
        assert_eq!(gpu.read_vram(1023, 511), 0x5678);

        // But state should be reset
        assert!(gpu.display_mode.display_disabled);
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_gp1_reset_command_buffer() {
        let mut gpu = GPU::new();
        gpu.command_fifo.push_back(0x12345678);
        gpu.command_fifo.push_back(0x9ABCDEF0);

        gpu.write_gp1(0x01000000);

        assert!(gpu.command_fifo.is_empty());
        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_gp1_acknowledge_interrupt() {
        let mut gpu = GPU::new();
        gpu.status.interrupt_request = true;

        gpu.write_gp1(0x02000000);

        assert!(!gpu.status.interrupt_request);
    }

    #[test]
    fn test_gp1_display_enable() {
        let mut gpu = GPU::new();

        // Enable display (bit 0 = 0)
        gpu.write_gp1(0x03000000);
        assert!(!gpu.display_mode.display_disabled);
        assert!(!gpu.status.display_disabled);

        // Disable display (bit 0 = 1)
        gpu.write_gp1(0x03000001);
        assert!(gpu.display_mode.display_disabled);
        assert!(gpu.status.display_disabled);
    }

    #[test]
    fn test_gp1_dma_direction() {
        let mut gpu = GPU::new();

        // Test all DMA directions
        gpu.write_gp1(0x04000000);
        assert_eq!(gpu.status.dma_direction, 0);

        gpu.write_gp1(0x04000001);
        assert_eq!(gpu.status.dma_direction, 1);

        gpu.write_gp1(0x04000002);
        assert_eq!(gpu.status.dma_direction, 2);

        gpu.write_gp1(0x04000003);
        assert_eq!(gpu.status.dma_direction, 3);
    }

    #[test]
    fn test_gp1_display_area_start() {
        let mut gpu = GPU::new();

        // Set display area start to (8, 16)
        gpu.write_gp1(0x05000008 | (0x10 << 10));
        assert_eq!(gpu.display_area.x, 8);
        assert_eq!(gpu.display_area.y, 16);

        // Test with different coordinates
        gpu.write_gp1(0x05000100 | (0x80 << 10));
        assert_eq!(gpu.display_area.x, 256);
        assert_eq!(gpu.display_area.y, 128);
    }

    #[test]
    fn test_gp1_horizontal_display_range() {
        let mut gpu = GPU::new();

        // Set horizontal range from 100 to 400 (width = 300)
        gpu.write_gp1(0x06000064 | (0x190 << 12));
        assert_eq!(gpu.display_area.width, 300);

        // Test with different values
        gpu.write_gp1(0x06000000 | (0x280 << 12));
        assert_eq!(gpu.display_area.width, 640);
    }

    #[test]
    fn test_gp1_vertical_display_range() {
        let mut gpu = GPU::new();

        // Set vertical range from 16 to 256 (height = 240)
        gpu.write_gp1(0x07000010 | (0x100 << 10));
        assert_eq!(gpu.display_area.height, 240);

        // Test with different values
        gpu.write_gp1(0x07000020 | (0x200 << 10));
        assert_eq!(gpu.display_area.height, 480);
    }

    #[test]
    fn test_gp1_display_mode_320x240_ntsc() {
        let mut gpu = GPU::new();

        // 320x240 NTSC 15-bit non-interlaced
        // Bits: HR1=1, VRes=0, VideoMode=0, ColorDepth=0, Interlace=0, HR2=0
        gpu.write_gp1(0x08000001);

        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);
        assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R240);
        assert_eq!(gpu.display_mode.video_mode, VideoMode::NTSC);
        assert_eq!(
            gpu.display_mode.display_area_color_depth,
            ColorDepth::C15Bit
        );
        assert!(!gpu.display_mode.interlaced);

        // Check status bits are updated
        assert_eq!(gpu.status.horizontal_res_1, 1);
        assert_eq!(gpu.status.horizontal_res_2, 0);
        assert!(!gpu.status.vertical_res);
        assert!(!gpu.status.video_mode);
        assert!(!gpu.status.display_area_color_depth);
        assert!(!gpu.status.vertical_interlace);
    }

    #[test]
    fn test_gp1_display_mode_640x480_pal_interlaced() {
        let mut gpu = GPU::new();

        // 640x480 PAL 24-bit interlaced
        // Bits: HR1=3, VRes=1, VideoMode=1, ColorDepth=1, Interlace=1, HR2=0
        // Value = 0x03 | (1<<2) | (1<<3) | (1<<4) | (1<<5) = 0x3F
        gpu.write_gp1(0x0800003F);

        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R640);
        assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R480);
        assert_eq!(gpu.display_mode.video_mode, VideoMode::PAL);
        assert_eq!(
            gpu.display_mode.display_area_color_depth,
            ColorDepth::C24Bit
        );
        assert!(gpu.display_mode.interlaced);

        // Check status bits are updated
        assert_eq!(gpu.status.horizontal_res_1, 3);
        assert_eq!(gpu.status.horizontal_res_2, 0);
        assert!(gpu.status.vertical_res);
        assert!(gpu.status.video_mode);
        assert!(gpu.status.display_area_color_depth);
        assert!(gpu.status.vertical_interlace);
    }

    #[test]
    fn test_gp1_display_mode_368_horizontal() {
        let mut gpu = GPU::new();

        // 368 width mode (HR2=1, HR1=0)
        // Bits: HR1=0, HR2=1
        // Value = 0x00 | (1<<6) = 0x40
        gpu.write_gp1(0x08000040);

        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R368);
        assert_eq!(gpu.status.horizontal_res_1, 0);
        assert_eq!(gpu.status.horizontal_res_2, 1);
    }

    #[test]
    fn test_gp1_display_mode_384_horizontal() {
        let mut gpu = GPU::new();

        // 384 width mode (HR2=1, HR1=1)
        // Bits: HR1=1, HR2=1
        // Value = 0x01 | (1<<6) = 0x41
        gpu.write_gp1(0x08000041);

        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R384);
        assert_eq!(gpu.status.horizontal_res_1, 1);
        assert_eq!(gpu.status.horizontal_res_2, 1);
    }

    #[test]
    fn test_gp1_display_mode_all_resolutions() {
        let mut gpu = GPU::new();

        // Test 256 width
        gpu.write_gp1(0x08000000);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R256);

        // Test 320 width
        gpu.write_gp1(0x08000001);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);

        // Test 512 width
        gpu.write_gp1(0x08000002);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R512);

        // Test 640 width
        gpu.write_gp1(0x08000003);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R640);

        // Test 368 width (HR2=1, HR1=0)
        gpu.write_gp1(0x08000040);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R368);

        // Test 384 width (HR2=1, HR1=1)
        gpu.write_gp1(0x08000041);
        assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R384);
    }

    #[test]
    fn test_gp1_display_mode_reverse_flag() {
        let mut gpu = GPU::new();

        // Set reverse flag (bit 7)
        gpu.write_gp1(0x08000080);
        assert!(gpu.status.reverse_flag);

        // Clear reverse flag
        gpu.write_gp1(0x08000000);
        assert!(!gpu.status.reverse_flag);
    }

    #[test]
    fn test_gp1_get_gpu_info() {
        let mut gpu = GPU::new();

        // Test various info types (should not panic)
        gpu.write_gp1(0x10000002); // Texture window
        gpu.write_gp1(0x10000003); // Draw area top left
        gpu.write_gp1(0x10000004); // Draw area bottom right
        gpu.write_gp1(0x10000005); // Draw offset
        gpu.write_gp1(0x10000007); // GPU version
    }

    #[test]
    fn test_gp1_unknown_command() {
        let mut gpu = GPU::new();

        // Unknown command should not panic
        gpu.write_gp1(0xFF000000);
    }

    #[test]
    fn test_gp1_reset_clears_transfer() {
        let mut gpu = GPU::new();

        // Set up a VRAM transfer
        gpu.vram_transfer = Some(VRAMTransfer {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            current_x: 50,
            current_y: 50,
            direction: VRAMTransferDirection::CpuToVram,
        });

        // Reset command buffer should clear transfer
        gpu.write_gp1(0x01000000);
        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_gp1_display_area_boundaries() {
        let mut gpu = GPU::new();

        // Test maximum coordinates
        gpu.write_gp1(0x050003FF | (0x1FF << 10)); // Max X=1023, Y=511
        assert_eq!(gpu.display_area.x, 1023);
        assert_eq!(gpu.display_area.y, 511);

        // Test zero coordinates
        gpu.write_gp1(0x05000000);
        assert_eq!(gpu.display_area.x, 0);
        assert_eq!(gpu.display_area.y, 0);
    }

    #[test]
    fn test_gp1_display_range_saturation() {
        let mut gpu = GPU::new();

        // Test horizontal range where x2 < x1 (should saturate to 0)
        gpu.write_gp1(0x06000200 | (0x100 << 12));
        assert_eq!(gpu.display_area.width, 0);

        // Test vertical range where y2 < y1 (should saturate to 0)
        gpu.write_gp1(0x07000200 | (0x100 << 10));
        assert_eq!(gpu.display_area.height, 0);
    }

    // GP0 VRAM Transfer Tests

    #[test]
    fn test_cpu_to_vram_transfer() {
        let mut gpu = GPU::new();

        // Start transfer: position (10, 20), size 2x2
        gpu.write_gp0(0xA0000000);
        gpu.write_gp0(0x0014000A); // y=20, x=10
        gpu.write_gp0(0x00020002); // height=2, width=2

        // Write 2 u32 words (4 pixels total for 2x2)
        gpu.write_gp0(0x7FFF7FFF); // Two white pixels
        gpu.write_gp0(0x00000000); // Two black pixels

        // Verify pixels written correctly
        assert_eq!(gpu.read_vram(10, 20), 0x7FFF);
        assert_eq!(gpu.read_vram(11, 20), 0x7FFF);
        assert_eq!(gpu.read_vram(10, 21), 0x0000);
        assert_eq!(gpu.read_vram(11, 21), 0x0000);

        // Transfer should be complete
        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_cpu_to_vram_transfer_wrapping() {
        let mut gpu = GPU::new();

        // Test coordinate wrapping at VRAM boundary
        gpu.write_gp0(0xA0000000);
        gpu.write_gp0(0x000003FF); // position (1023, 0)
        gpu.write_gp0(0x00010002); // size 2x1

        // Write 1 u32 word (2 pixels)
        gpu.write_gp0(0x12345678);

        // Verify wrapping: second pixel wraps to x=0 same row
        // VRAM coordinates wrap independently from transfer coordinates
        assert_eq!(gpu.read_vram(1023, 0), 0x5678);
        assert_eq!(gpu.read_vram(0, 0), 0x1234); // Wrapped to x=0 same row
    }

    #[test]
    fn test_cpu_to_vram_transfer_odd_width() {
        let mut gpu = GPU::new();

        // Test transfer with odd width (3 pixels = 2 u32 words)
        gpu.write_gp0(0xA0000000);
        gpu.write_gp0(0x00000000); // position (0, 0)
        gpu.write_gp0(0x00010003); // size 3x1

        // Write 2 u32 words (4 pixels, but only 3 are in transfer)
        gpu.write_gp0(0xAAAABBBB);
        gpu.write_gp0(0xCCCCDDDD);

        // Verify only 3 pixels written
        assert_eq!(gpu.read_vram(0, 0), 0xBBBB);
        assert_eq!(gpu.read_vram(1, 0), 0xAAAA);
        assert_eq!(gpu.read_vram(2, 0), 0xDDDD);

        // Transfer should be complete after 3 pixels
        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_vram_to_cpu_transfer() {
        let mut gpu = GPU::new();

        // Setup VRAM with test pattern
        gpu.write_vram(100, 100, 0x1234);
        gpu.write_vram(101, 100, 0x5678);
        gpu.write_vram(102, 100, 0x9ABC);
        gpu.write_vram(103, 100, 0xDEF0);

        // Start read transfer: position (100, 100), size 4x1
        gpu.write_gp0(0xC0000000);
        gpu.write_gp0(0x00640064); // position (100, 100)
        gpu.write_gp0(0x00010004); // size 4x1

        // Read data (2 pixels per read)
        let data1 = gpu.read_gpuread();
        assert_eq!(data1 & 0xFFFF, 0x1234);
        assert_eq!((data1 >> 16) & 0xFFFF, 0x5678);

        let data2 = gpu.read_gpuread();
        assert_eq!(data2 & 0xFFFF, 0x9ABC);
        assert_eq!((data2 >> 16) & 0xFFFF, 0xDEF0);

        // Transfer should be complete
        assert!(gpu.vram_transfer.is_none());
        assert!(!gpu.status.ready_to_send_vram);
    }

    #[test]
    fn test_vram_to_cpu_transfer_odd_width() {
        let mut gpu = GPU::new();

        // Setup VRAM with test pattern
        gpu.write_vram(50, 50, 0xAAAA);
        gpu.write_vram(51, 50, 0xBBBB);
        gpu.write_vram(52, 50, 0xCCCC);

        // Start read transfer: position (50, 50), size 3x1
        gpu.write_gp0(0xC0000000);
        gpu.write_gp0(0x00320032); // position (50, 50)
        gpu.write_gp0(0x00010003); // size 3x1

        // Read first 2 pixels
        let data1 = gpu.read_gpuread();
        assert_eq!(data1 & 0xFFFF, 0xAAAA);
        assert_eq!((data1 >> 16) & 0xFFFF, 0xBBBB);

        // Read remaining pixel (second pixel should be 0 as transfer ends)
        let data2 = gpu.read_gpuread();
        assert_eq!(data2 & 0xFFFF, 0xCCCC);
        assert_eq!((data2 >> 16) & 0xFFFF, 0); // No more data

        // Transfer should be complete
        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_vram_to_cpu_status_flag() {
        let mut gpu = GPU::new();

        // Initially not ready to send
        assert!(gpu.status.ready_to_send_vram);

        // Start transfer
        gpu.write_gp0(0xC0000000);
        gpu.write_gp0(0x00000000);
        gpu.write_gp0(0x00010001); // 1x1 transfer

        // Should be ready to send
        assert!(gpu.status.ready_to_send_vram);

        // Read data
        let _ = gpu.read_gpuread();

        // Should no longer be ready after transfer complete
        assert!(!gpu.status.ready_to_send_vram);
    }

    #[test]
    fn test_vram_to_vram_transfer() {
        let mut gpu = GPU::new();

        // Write source data
        gpu.write_vram(0, 0, 0xAAAA);
        gpu.write_vram(1, 0, 0xBBBB);
        gpu.write_vram(0, 1, 0xCCCC);
        gpu.write_vram(1, 1, 0xDDDD);

        // Copy 2x2 rectangle from (0,0) to (10,10)
        gpu.write_gp0(0x80000000);
        gpu.write_gp0(0x00000000); // src (0, 0)
        gpu.write_gp0(0x000A000A); // dst (10, 10)
        gpu.write_gp0(0x00020002); // size 2x2

        // Verify destination
        assert_eq!(gpu.read_vram(10, 10), 0xAAAA);
        assert_eq!(gpu.read_vram(11, 10), 0xBBBB);
        assert_eq!(gpu.read_vram(10, 11), 0xCCCC);
        assert_eq!(gpu.read_vram(11, 11), 0xDDDD);

        // Source should be unchanged
        assert_eq!(gpu.read_vram(0, 0), 0xAAAA);
        assert_eq!(gpu.read_vram(1, 0), 0xBBBB);
    }

    #[test]
    fn test_vram_to_vram_transfer_overlapping() {
        let mut gpu = GPU::new();

        // Write source data in a line
        for i in 0..5 {
            gpu.write_vram(i, 0, 0x1000 + i);
        }

        // Copy overlapping region: (0,0) to (2,0), size 3x1
        // This tests that we use a temporary buffer
        gpu.write_gp0(0x80000000);
        gpu.write_gp0(0x00000000); // src (0, 0)
        gpu.write_gp0(0x00000002); // dst (2, 0)
        gpu.write_gp0(0x00010003); // size 3x1

        // Verify copy worked correctly despite overlap
        assert_eq!(gpu.read_vram(2, 0), 0x1000);
        assert_eq!(gpu.read_vram(3, 0), 0x1001);
        assert_eq!(gpu.read_vram(4, 0), 0x1002);
    }

    #[test]
    fn test_vram_to_vram_transfer_wrapping() {
        let mut gpu = GPU::new();

        // Write at edge of VRAM
        gpu.write_vram(1023, 511, 0xABCD);
        gpu.write_vram(0, 0, 0x1234);

        // Copy from edge, should wrap
        gpu.write_gp0(0x80000000);
        gpu.write_gp0(0x01FF03FF); // src (1023, 511)
        gpu.write_gp0(0x00640064); // dst (100, 100)
        gpu.write_gp0(0x00020002); // size 2x2

        // Verify wrapped copy
        assert_eq!(gpu.read_vram(100, 100), 0xABCD);
        // Other pixels will be from wrapped coordinates
    }

    #[test]
    fn test_gp0_command_buffering() {
        let mut gpu = GPU::new();

        // Send partial command (should buffer)
        gpu.write_gp0(0xA0000000);
        assert_eq!(gpu.command_fifo.len(), 1);
        assert!(gpu.vram_transfer.is_none());

        // Send second word
        gpu.write_gp0(0x00000000);
        assert_eq!(gpu.command_fifo.len(), 2);
        assert!(gpu.vram_transfer.is_none());

        // Send third word - command should execute
        gpu.write_gp0(0x00010001);
        assert_eq!(gpu.command_fifo.len(), 0);
        assert!(gpu.vram_transfer.is_some());
    }

    #[test]
    fn test_gp0_unknown_command() {
        let mut gpu = GPU::new();

        // Send unknown command (should be ignored)
        gpu.write_gp0(0xFF000000);

        // FIFO should be empty (command removed)
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_vram_transfer_interrupt_by_gp1_reset() {
        let mut gpu = GPU::new();

        // Start a VRAM transfer
        gpu.write_gp0(0xA0000000);
        gpu.write_gp0(0x00000000);
        gpu.write_gp0(0x00010001);
        assert!(gpu.vram_transfer.is_some());

        // Reset command buffer via GP1
        gpu.write_gp1(0x01000000);

        // Transfer should be cancelled
        assert!(gpu.vram_transfer.is_none());
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_cpu_to_vram_size_alignment() {
        let mut gpu = GPU::new();

        // Test that size of 0 wraps to maximum (1024×512 per PSX hardware behavior)
        gpu.write_gp0(0xA0000000);
        gpu.write_gp0(0x00000000); // position (0, 0)
        gpu.write_gp0(0x00000000); // size 0x0 (wraps to 1024×512)

        // Write 1 data word (2 pixels)
        gpu.write_gp0(0x12345678);

        // Verify first two pixels were written
        assert_eq!(gpu.read_vram(0, 0), 0x5678);
        assert_eq!(gpu.read_vram(1, 0), 0x1234);

        // Transfer should still be in progress (1024×512 is much larger than 2 pixels)
        assert!(gpu.vram_transfer.is_some());
    }

    #[test]
    fn test_vram_to_cpu_multiline() {
        let mut gpu = GPU::new();

        // Write 2x2 pattern
        gpu.write_vram(0, 0, 0xAAAA);
        gpu.write_vram(1, 0, 0xBBBB);
        gpu.write_vram(0, 1, 0xCCCC);
        gpu.write_vram(1, 1, 0xDDDD);

        // Read 2x2 area
        gpu.write_gp0(0xC0000000);
        gpu.write_gp0(0x00000000); // position (0, 0)
        gpu.write_gp0(0x00020002); // size 2x2

        // Read first row
        let data1 = gpu.read_gpuread();
        assert_eq!(data1 & 0xFFFF, 0xAAAA);
        assert_eq!((data1 >> 16) & 0xFFFF, 0xBBBB);

        // Read second row
        let data2 = gpu.read_gpuread();
        assert_eq!(data2 & 0xFFFF, 0xCCCC);
        assert_eq!((data2 >> 16) & 0xFFFF, 0xDDDD);

        assert!(gpu.vram_transfer.is_none());
    }

    #[test]
    fn test_color_conversion() {
        let color = Color {
            r: 255,
            g: 128,
            b: 64,
        };
        let rgb15 = color.to_rgb15();

        // Verify 15-bit conversion
        assert_eq!(rgb15 & 0x1F, 31); // R: 255 >> 3 = 31
        assert_eq!((rgb15 >> 5) & 0x1F, 16); // G: 128 >> 3 = 16
        assert_eq!((rgb15 >> 10) & 0x1F, 8); // B: 64 >> 3 = 8
    }

    #[test]
    fn test_color_from_u32() {
        let color = Color::from_u32(0x00FF8040);
        assert_eq!(color.r, 0x40);
        assert_eq!(color.g, 0x80);
        assert_eq!(color.b, 0xFF);
    }

    #[test]
    fn test_vertex_from_u32() {
        let v = Vertex::from_u32(0x00640032); // x=50, y=100
        assert_eq!(v.x, 50);
        assert_eq!(v.y, 100);
    }

    #[test]
    fn test_vertex_from_u32_negative() {
        // Test negative coordinates (signed 16-bit)
        let v = Vertex::from_u32(0xFFFFFFFF);
        assert_eq!(v.x, -1);
        assert_eq!(v.y, -1);
    }

    #[test]
    fn test_monochrome_triangle_parsing() {
        let mut gpu = GPU::new();

        // Monochrome triangle command
        gpu.write_gp0(0x20FF0000); // Red triangle
        gpu.write_gp0(0x00640032); // V1: (50, 100)
        gpu.write_gp0(0x00C80096); // V2: (200, 150)
        gpu.write_gp0(0x00320064); // V3: (50, 100)

        // Command should be processed (no crash, FIFO empty)
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_monochrome_triangle_semi_transparent() {
        let mut gpu = GPU::new();

        // Semi-transparent triangle command
        gpu.write_gp0(0x2200FF00); // Green semi-transparent triangle
        gpu.write_gp0(0x00000000); // V1: (0, 0)
        gpu.write_gp0(0x00640000); // V2: (100, 0)
        gpu.write_gp0(0x00320064); // V3: (50, 100)

        // Command should be processed
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_monochrome_quad_parsing() {
        let mut gpu = GPU::new();

        gpu.write_gp0(0x2800FF00); // Green quad
        gpu.write_gp0(0x00000000); // V1: (0, 0)
        gpu.write_gp0(0x00640000); // V2: (100, 0)
        gpu.write_gp0(0x00640064); // V3: (100, 100)
        gpu.write_gp0(0x00000064); // V4: (0, 100)

        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_monochrome_quad_semi_transparent() {
        let mut gpu = GPU::new();

        gpu.write_gp0(0x2A0000FF); // Blue semi-transparent quad
        gpu.write_gp0(0x000A000A); // V1: (10, 10)
        gpu.write_gp0(0x0032000A); // V2: (50, 10)
        gpu.write_gp0(0x00320032); // V3: (50, 50)
        gpu.write_gp0(0x000A0032); // V4: (10, 50)

        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_drawing_offset_applied() {
        let mut gpu = GPU::new();
        gpu.draw_offset = (10, 20);

        let vertices = [
            Vertex { x: 0, y: 0 },
            Vertex { x: 10, y: 0 },
            Vertex { x: 5, y: 10 },
        ];

        let color = Color { r: 255, g: 0, b: 0 };

        // Should not crash
        gpu.render_monochrome_triangle(&vertices, &color, false);
    }

    #[test]
    fn test_partial_command_buffering() {
        let mut gpu = GPU::new();

        // Send only 2 words of a triangle command (needs 4)
        gpu.write_gp0(0x20FF0000); // Command + color
        gpu.write_gp0(0x00000000); // V1

        // Should be buffered, not processed
        assert_eq!(gpu.command_fifo.len(), 2);

        // Send remaining words
        gpu.write_gp0(0x00640000); // V2
        gpu.write_gp0(0x00320064); // V3

        // Now command should be processed
        assert!(gpu.command_fifo.is_empty());
    }

    #[test]
    fn test_quad_splits_into_two_triangles() {
        let mut gpu = GPU::new();

        // Render a quad - internally it should split into two triangles
        let vertices = [
            Vertex { x: 0, y: 0 },
            Vertex { x: 100, y: 0 },
            Vertex { x: 100, y: 100 },
            Vertex { x: 0, y: 100 },
        ];

        let color = Color {
            r: 255,
            g: 255,
            b: 255,
        };

        // Should not crash - actual rendering stub is called twice internally
        gpu.render_monochrome_quad(&vertices, &color, false);
    }
}
