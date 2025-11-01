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

//! GPU type definitions
//!
//! This module contains all the type definitions used by the GPU,
//! including colors, vertices, drawing modes, display settings, and GPU commands.

/// A 24-bit RGB color used in GPU commands
///
/// PlayStation GPU commands use 24-bit RGB colors (8 bits per channel)
/// which are converted to 15-bit RGB for VRAM storage.
///
/// # Examples
///
/// ```
/// use psrx::core::gpu::Color;
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
    /// use psrx::core::gpu::Color;
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
    /// use psrx::core::gpu::Color;
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
/// use psrx::core::gpu::Vertex;
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
    /// use psrx::core::gpu::Vertex;
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

/// Texture coordinate for textured primitives
///
/// Texture coordinates specify which texel (texture pixel) to sample from VRAM.
/// Coordinates are in texel units within the texture page.
///
/// # Coordinate System
///
/// - U: Horizontal texture coordinate (0-255)
/// - V: Vertical texture coordinate (0-255)
/// - Coordinates wrap within the texture page
///
/// # Examples
///
/// ```
/// use psrx::core::gpu::TexCoord;
///
/// let texcoord = TexCoord::from_u32(0x00804020);
/// assert_eq!(texcoord.u, 0x20);
/// assert_eq!(texcoord.v, 0x40);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TexCoord {
    /// U coordinate (horizontal, 0-255)
    pub u: u8,
    /// V coordinate (vertical, 0-255)
    pub v: u8,
}

impl TexCoord {
    /// Create a TexCoord from a 32-bit command word
    ///
    /// Texture coordinates are encoded in the lower 16 bits:
    /// - Bits 0-7: U coordinate
    /// - Bits 8-15: V coordinate
    ///
    /// # Arguments
    ///
    /// * `value` - 32-bit word containing texture coordinates
    ///
    /// # Returns
    ///
    /// TexCoord struct with U and V coordinates
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::TexCoord;
    ///
    /// let tc = TexCoord::from_u32(0x00804020);
    /// assert_eq!(tc.u, 0x20);
    /// assert_eq!(tc.v, 0x40);
    /// ```
    pub fn from_u32(value: u32) -> Self {
        Self {
            u: (value & 0xFF) as u8,
            v: ((value >> 8) & 0xFF) as u8,
        }
    }
}

/// Texture color depth modes
///
/// The PlayStation GPU supports three texture formats:
/// - 4-bit: 16 colors using a 16-color CLUT (Color Lookup Table)
/// - 8-bit: 256 colors using a 256-color CLUT
/// - 15-bit: Direct color (no CLUT needed)
///
/// # CLUT (Color Lookup Table)
///
/// For 4-bit and 8-bit textures, the texture data contains palette indices
/// that are looked up in a CLUT stored elsewhere in VRAM. Each CLUT entry
/// is a 16-bit color in 5-5-5 RGB format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDepth {
    /// 4-bit indexed color (16 colors, uses CLUT)
    T4Bit,
    /// 8-bit indexed color (256 colors, uses CLUT)
    T8Bit,
    /// 15-bit direct color (no CLUT)
    T15Bit,
}

impl From<u8> for TextureDepth {
    /// Convert DrawMode texture_depth value to TextureDepth enum
    ///
    /// # Arguments
    ///
    /// * `value` - Texture depth value (0=4bit, 1=8bit, 2=15bit)
    ///
    /// # Returns
    ///
    /// TextureDepth enum value
    fn from(value: u8) -> Self {
        match value {
            0 => TextureDepth::T4Bit,
            1 => TextureDepth::T8Bit,
            _ => TextureDepth::T15Bit,
        }
    }
}

/// Texture mapping information
///
/// Contains all information needed to sample a texture from VRAM, including
/// texture page location, CLUT location, and color depth.
///
/// # Texture Pages
///
/// VRAM is divided into texture pages of varying sizes depending on color depth:
/// - 4-bit: 256×256 pixels (stored as 64×256 16-bit values, 4 pixels per value)
/// - 8-bit: 128×256 pixels (stored as 64×256 16-bit values, 2 pixels per value)
/// - 15-bit: 64×256 pixels (1 pixel per 16-bit value)
///
/// # CLUT Location
///
/// For 4-bit and 8-bit textures, the CLUT (palette) is stored separately in VRAM.
/// - 4-bit CLUT: 16 colors (16 consecutive pixels)
/// - 8-bit CLUT: 256 colors (256 consecutive pixels)
///
/// # Examples
///
/// ```
/// use psrx::core::gpu::{TextureInfo, TextureDepth};
///
/// let texture = TextureInfo {
///     page_x: 64,      // Texture page at X=64
///     page_y: 0,       // Texture page at Y=0
///     clut_x: 0,       // CLUT at X=0
///     clut_y: 0,       // CLUT at Y=0
///     depth: TextureDepth::T4Bit,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TextureInfo {
    /// Texture page base X coordinate (in pixels)
    pub page_x: u16,

    /// Texture page base Y coordinate (0 or 256)
    pub page_y: u16,

    /// CLUT X position in VRAM (for 4-bit/8-bit textures)
    pub clut_x: u16,

    /// CLUT Y position in VRAM (for 4-bit/8-bit textures)
    pub clut_y: u16,

    /// Texture color depth
    pub depth: TextureDepth,
}

/// Semi-transparency blending modes
///
/// The PlayStation GPU supports 4 semi-transparency (alpha blending) modes
/// for rendering translucent effects like glass, water, and explosions.
///
/// # Blending Formula
///
/// Each mode specifies how to combine the background color (B) with the
/// foreground color (F):
/// - **Average**: (B/2 + F/2) - 50% blend
/// - **Additive**: (B + F) - additive blending (brightens)
/// - **Subtractive**: (B - F) - subtractive blending (darkens)
/// - **AddQuarter**: (B + F/4) - adds 25% of foreground
///
/// # Color Components
///
/// Blending operates on each RGB channel independently in 5-bit precision (0-31).
/// Results are clamped to prevent overflow.
///
/// # Examples
///
/// ```
/// use psrx::core::gpu::BlendMode;
///
/// let mode = BlendMode::from_bits(0); // Average mode
/// let blended = mode.blend(0x7FFF, 0x0000); // White bg, black fg
/// // Result: ~50% gray
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Average: 0.5×B + 0.5×F (50% blend)
    ///
    /// The most common blending mode, producing a 50/50 mix of the
    /// background and foreground colors. Used for semi-transparent surfaces.
    Average,

    /// Additive: 1.0×B + 1.0×F (additive blending)
    ///
    /// Adds the foreground color to the background. Creates brightening
    /// effects like fire, explosions, and light beams. Results are clamped to max (31).
    Additive,

    /// Subtractive: 1.0×B - 1.0×F (subtractive blending)
    ///
    /// Subtracts the foreground color from the background. Creates darkening
    /// effects like shadows. Results are clamped to min (0).
    Subtractive,

    /// AddQuarter: 1.0×B + 0.25×F (add 25% of foreground)
    ///
    /// Adds 25% of the foreground to the background. Creates subtle
    /// brightening effects. Results are clamped to max (31).
    AddQuarter,
}

impl BlendMode {
    /// Create BlendMode from semi-transparency bits
    ///
    /// Converts the 2-bit semi-transparency value from GPU commands or
    /// draw mode settings to a BlendMode enum.
    ///
    /// # Arguments
    ///
    /// * `bits` - Semi-transparency mode (0-3)
    ///
    /// # Returns
    ///
    /// Corresponding BlendMode enum value
    ///
    /// # Mapping
    ///
    /// - 0 → Average (0.5×B + 0.5×F)
    /// - 1 → Additive (1.0×B + 1.0×F)
    /// - 2 → Subtractive (1.0×B - 1.0×F)
    /// - 3 → AddQuarter (1.0×B + 0.25×F)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::BlendMode;
    ///
    /// assert_eq!(BlendMode::from_bits(0), BlendMode::Average);
    /// assert_eq!(BlendMode::from_bits(1), BlendMode::Additive);
    /// assert_eq!(BlendMode::from_bits(2), BlendMode::Subtractive);
    /// assert_eq!(BlendMode::from_bits(3), BlendMode::AddQuarter);
    /// ```
    pub fn from_bits(bits: u8) -> Self {
        match bits & 3 {
            0 => BlendMode::Average,
            1 => BlendMode::Additive,
            2 => BlendMode::Subtractive,
            3 => BlendMode::AddQuarter,
            _ => unreachable!(),
        }
    }

    /// Blend background and foreground colors
    ///
    /// Performs semi-transparent blending of two 15-bit RGB colors according
    /// to the blend mode. Each RGB channel is processed independently in 5-bit
    /// precision, and results are clamped to the valid range (0-31).
    ///
    /// # Arguments
    ///
    /// * `background` - Background color in 5-5-5 RGB format
    /// * `foreground` - Foreground color in 5-5-5 RGB format
    ///
    /// # Returns
    ///
    /// Blended color in 5-5-5 RGB format
    ///
    /// # Algorithm
    ///
    /// 1. Unpack 15-bit colors to separate 5-bit R, G, B channels
    /// 2. Apply blend formula to each channel independently
    /// 3. Clamp results to 0-31 range
    /// 4. Pack back to 15-bit RGB format
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::BlendMode;
    ///
    /// let bg = 0x7FFF; // White (31, 31, 31)
    /// let fg = 0x0000; // Black (0, 0, 0)
    ///
    /// let result = BlendMode::Average.blend(bg, fg);
    /// // Result should be ~50% gray (15, 15, 15)
    /// ```
    pub fn blend(&self, background: u16, foreground: u16) -> u16 {
        let (br, bg, bb) = Self::unpack_rgb15(background);
        let (fr, fg, fb) = Self::unpack_rgb15(foreground);

        let (r, g, b) = match self {
            BlendMode::Average => (
                (br / 2 + fr / 2).min(31),
                (bg / 2 + fg / 2).min(31),
                (bb / 2 + fb / 2).min(31),
            ),
            BlendMode::Additive => ((br + fr).min(31), (bg + fg).min(31), (bb + fb).min(31)),
            BlendMode::Subtractive => (
                br.saturating_sub(fr),
                bg.saturating_sub(fg),
                bb.saturating_sub(fb),
            ),
            BlendMode::AddQuarter => (
                (br + fr / 4).min(31),
                (bg + fg / 4).min(31),
                (bb + fb / 4).min(31),
            ),
        };

        Self::pack_rgb15(r, g, b)
    }

    /// Unpack 15-bit RGB color to separate 5-bit channels
    ///
    /// Extracts the red, green, and blue components from a 15-bit color value.
    ///
    /// # Arguments
    ///
    /// * `color` - 16-bit color in 5-5-5 RGB format
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 5-bit values (0-31)
    ///
    /// # Format
    ///
    /// - Bits 0-4: Red (5 bits)
    /// - Bits 5-9: Green (5 bits)
    /// - Bits 10-14: Blue (5 bits)
    /// - Bit 15: Mask bit (ignored)
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::gpu::BlendMode;
    /// // This is a private method, shown for documentation
    /// // 0x7FFF (white) -> (31, 31, 31)
    /// // 0x001F (red) -> (31, 0, 0)
    /// ```
    fn unpack_rgb15(color: u16) -> (u16, u16, u16) {
        let r = color & 0x1F;
        let g = (color >> 5) & 0x1F;
        let b = (color >> 10) & 0x1F;
        (r, g, b)
    }

    /// Pack 5-bit RGB channels into 15-bit color
    ///
    /// Combines separate red, green, and blue components into a single
    /// 15-bit color value.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel (0-31)
    /// * `g` - Green channel (0-31)
    /// * `b` - Blue channel (0-31)
    ///
    /// # Returns
    ///
    /// 16-bit color in 5-5-5 RGB format (bit 15 is 0)
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::gpu::BlendMode;
    /// // This is a private method, shown for documentation
    /// // (31, 0, 0) -> 0x001F (red)
    /// // (31, 31, 31) -> 0x7FFF (white)
    /// ```
    fn pack_rgb15(r: u16, g: u16, b: u16) -> u16 {
        (b << 10) | (g << 5) | r
    }
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
/// use psrx::core::gpu::{GPUCommand, Vertex, Color};
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
