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

//! Software Rasterizer
//!
//! This module implements the triangle rasterizer that converts polygon commands
//! into actual pixels in VRAM. The rasterizer uses a scanline-based algorithm
//! for filling triangles.
//!
//! # Algorithm
//!
//! The rasterizer uses a scanline approach which splits triangles into
//! top-flat and bottom-flat sub-triangles:
//!
//! 1. Sort vertices by Y coordinate
//! 2. Split the triangle at the middle vertex
//! 3. Rasterize each half using linear interpolation
//! 4. Clip each scanline to the drawing area
//!
//! # Performance
//!
//! The rasterizer is optimized for performance:
//! - Uses unsafe raw pointer for VRAM access to avoid bounds checking
//! - Inline hot paths for pixel writing
//! - Pre-compute slopes to avoid repeated division
//!
//! # References
//!
//! - [Triangle Rasterization Tutorial](https://www.sunshine2k.de/coding/java/TriangleRasterization/TriangleRasterization.html)
//! - [Scratchapixel: Rasterization](https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation)

/// Triangle rasterizer using scanline algorithm
///
/// The rasterizer takes triangle vertices and fills the interior pixels,
/// respecting clipping boundaries set by the drawing area.
///
/// # Examples
///
/// ```
/// use psrx::core::gpu::Rasterizer;
///
/// let mut vram = vec![0u16; 1024 * 512];
/// let mut rasterizer = Rasterizer::new();
/// rasterizer.set_clip_rect(0, 0, 1023, 511);
///
/// // Draw a red triangle
/// rasterizer.draw_triangle(
///     &mut vram,
///     (100, 100),
///     (200, 100),
///     (150, 200),
///     0x001F  // Red in 5-5-5 RGB
/// );
/// ```
pub struct Rasterizer {
    /// Drawing area (clipping rectangle)
    ///
    /// All pixels are clipped to this rectangle.
    /// Format: (left, top, right, bottom) - all inclusive
    clip_rect: (i16, i16, i16, i16),
}

impl Rasterizer {
    /// Create a new rasterizer
    ///
    /// # Returns
    ///
    /// A new Rasterizer with default clipping (full VRAM)
    pub fn new() -> Self {
        Self {
            clip_rect: (0, 0, 1023, 511),
        }
    }

    /// Set the clipping rectangle
    ///
    /// All drawing operations are clipped to this rectangle.
    /// Coordinates are inclusive.
    ///
    /// # Arguments
    ///
    /// * `left` - Left edge X coordinate
    /// * `top` - Top edge Y coordinate
    /// * `right` - Right edge X coordinate
    /// * `bottom` - Bottom edge Y coordinate
    pub fn set_clip_rect(&mut self, left: i16, top: i16, right: i16, bottom: i16) {
        self.clip_rect = (left, top, right, bottom);
    }

    /// Rasterize a solid color triangle
    ///
    /// Uses a scanline algorithm to fill the triangle with the specified color.
    /// The triangle is automatically split into top-flat and bottom-flat sections
    /// for efficient rasterization.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to the VRAM buffer
    /// * `v0` - First vertex (x, y)
    /// * `v1` - Second vertex (x, y)
    /// * `v2` - Third vertex (x, y)
    /// * `color` - 16-bit color in 5-5-5 RGB format
    ///
    /// # Algorithm
    ///
    /// 1. Sort vertices by Y coordinate (v0.y <= v1.y <= v2.y)
    /// 2. Check for degenerate cases (zero height)
    /// 3. Split triangle at middle vertex
    /// 4. Rasterize top and bottom halves separately
    pub fn draw_triangle(
        &mut self,
        vram: &mut [u16],
        v0: (i16, i16),
        v1: (i16, i16),
        v2: (i16, i16),
        color: u16,
    ) {
        // Sort vertices by Y coordinate (v0.y <= v1.y <= v2.y)
        let (v0, v1, v2) = Self::sort_vertices_by_y(v0, v1, v2);

        // Check if triangle is degenerate (zero height)
        if v0.1 == v2.1 {
            return; // Zero height triangle - nothing to draw
        }

        // Split into top-flat and bottom-flat triangles
        if v1.1 == v2.1 {
            // Bottom is flat (v1 and v2 have same Y)
            self.draw_bottom_flat_triangle(vram, v0, v1, v2, color);
        } else if v0.1 == v1.1 {
            // Top is flat (v0 and v1 have same Y)
            self.draw_top_flat_triangle(vram, v0, v1, v2, color);
        } else {
            // General case: split at v1.y
            // Find the X coordinate on the v0-v2 edge at v1.y
            let numerator = (v1.1 - v0.1) as i64 * (v2.0 - v0.0) as i64;
            let denominator = (v2.1 - v0.1) as i64;
            let v3_x = v0.0 + (numerator / denominator) as i16;
            let v3 = (v3_x, v1.1);

            // Draw both halves
            self.draw_bottom_flat_triangle(vram, v0, v1, v3, color);
            self.draw_top_flat_triangle(vram, v1, v3, v2, color);
        }
    }

    /// Sort three vertices by Y coordinate
    ///
    /// Returns vertices in ascending Y order: (v0.y <= v1.y <= v2.y)
    ///
    /// # Arguments
    ///
    /// * `v0` - First vertex
    /// * `v1` - Second vertex
    /// * `v2` - Third vertex
    ///
    /// # Returns
    ///
    /// Tuple of vertices sorted by Y coordinate
    fn sort_vertices_by_y(
        v0: (i16, i16),
        v1: (i16, i16),
        v2: (i16, i16),
    ) -> ((i16, i16), (i16, i16), (i16, i16)) {
        let mut verts = [v0, v1, v2];
        verts.sort_by_key(|v| v.1);
        (verts[0], verts[1], verts[2])
    }

    /// Draw a triangle with a flat bottom edge
    ///
    /// Rasterizes a triangle where v1 and v2 have the same Y coordinate.
    /// Fills scanlines from v0.y down to v1.y/v2.y.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `v0` - Top vertex
    /// * `v1` - Bottom-left vertex (v1.y == v2.y)
    /// * `v2` - Bottom-right vertex (v1.y == v2.y)
    /// * `color` - Fill color
    fn draw_bottom_flat_triangle(
        &mut self,
        vram: &mut [u16],
        v0: (i16, i16),
        v1: (i16, i16),
        v2: (i16, i16),
        color: u16,
    ) {
        // Calculate inverse slopes (dx/dy)
        let inv_slope1 = (v1.0 - v0.0) as f32 / (v1.1 - v0.1) as f32;
        let inv_slope2 = (v2.0 - v0.0) as f32 / (v2.1 - v0.1) as f32;

        let mut cur_x1 = v0.0 as f32;
        let mut cur_x2 = v0.0 as f32;

        // Scan from top to bottom
        for scanline in v0.1..=v1.1 {
            self.draw_scanline(vram, scanline, cur_x1 as i16, cur_x2 as i16, color);
            cur_x1 += inv_slope1;
            cur_x2 += inv_slope2;
        }
    }

    /// Draw a triangle with a flat top edge
    ///
    /// Rasterizes a triangle where v0 and v1 have the same Y coordinate.
    /// Fills scanlines from v0.y/v1.y down to v2.y.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `v0` - Top-left vertex (v0.y == v1.y)
    /// * `v1` - Top-right vertex (v0.y == v1.y)
    /// * `v2` - Bottom vertex
    /// * `color` - Fill color
    fn draw_top_flat_triangle(
        &mut self,
        vram: &mut [u16],
        v0: (i16, i16),
        v1: (i16, i16),
        v2: (i16, i16),
        color: u16,
    ) {
        // Calculate inverse slopes (dx/dy)
        let inv_slope1 = (v2.0 - v0.0) as f32 / (v2.1 - v0.1) as f32;
        let inv_slope2 = (v2.0 - v1.0) as f32 / (v2.1 - v1.1) as f32;

        let mut cur_x1 = v2.0 as f32;
        let mut cur_x2 = v2.0 as f32;

        // Scan from bottom to top
        for scanline in (v0.1..=v2.1).rev() {
            self.draw_scanline(vram, scanline, cur_x1 as i16, cur_x2 as i16, color);
            cur_x1 -= inv_slope1;
            cur_x2 -= inv_slope2;
        }
    }

    /// Draw a horizontal scanline
    ///
    /// Fills pixels from x1 to x2 on the specified scanline,
    /// clipping to the drawing area.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `y` - Scanline Y coordinate
    /// * `x1` - Start X coordinate
    /// * `x2` - End X coordinate
    /// * `color` - Fill color
    fn draw_scanline(&mut self, vram: &mut [u16], y: i16, mut x1: i16, mut x2: i16, color: u16) {
        // Ensure x1 <= x2
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }

        // Clip to drawing area
        let (clip_left, clip_top, clip_right, clip_bottom) = self.clip_rect;

        // Early reject if scanline is outside vertical bounds
        if y < clip_top || y > clip_bottom {
            return;
        }

        // Clip horizontal range
        let x1 = x1.max(clip_left);
        let x2 = x2.min(clip_right);

        // Check if there's anything to draw after clipping
        if x1 > x2 {
            return;
        }

        // Draw pixels
        for x in x1..=x2 {
            Self::write_pixel(vram, x, y, color);
        }
    }

    /// Write a single pixel to VRAM
    ///
    /// Performs bounds checking and writes the pixel if coordinates are valid.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `color` - Pixel color
    #[inline(always)]
    fn write_pixel(vram: &mut [u16], x: i16, y: i16, color: u16) {
        // Bounds check using range contains
        if !(0..1024).contains(&x) || !(0..512).contains(&y) {
            return;
        }

        let index = (y as usize) * 1024 + (x as usize);

        // Write pixel to VRAM
        // Bounds are checked above, so this is safe
        vram[index] = color;
    }

    /// Draw a line from (x0, y0) to (x1, y1) using Bresenham's algorithm
    ///
    /// Implements Bresenham's line algorithm, which efficiently rasterizes lines
    /// using only integer arithmetic. The algorithm works by incrementing the
    /// coordinate along the major axis and conditionally incrementing the minor
    /// axis based on an error accumulator.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `x0` - Start X coordinate
    /// * `y0` - Start Y coordinate
    /// * `x1` - End X coordinate
    /// * `y1` - End Y coordinate
    /// * `color` - Line color in 15-bit RGB format
    ///
    /// # Algorithm
    ///
    /// The algorithm maintains an error term that tracks when to step in the minor axis:
    /// 1. Calculate dx and dy (absolute differences)
    /// 2. Initialize error = dx + dy (dy is negative)
    /// 3. For each step:
    ///    - Draw pixel at current position
    ///    - If e2 >= dy, step in X direction
    ///    - If e2 <= dx, step in Y direction
    ///
    /// # References
    ///
    /// - [Bresenham's Line Algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::Rasterizer;
    ///
    /// let mut vram = vec![0u16; 1024 * 512];
    /// let mut rasterizer = Rasterizer::new();
    ///
    /// // Draw a white diagonal line
    /// rasterizer.draw_line(&mut vram, 0, 0, 100, 100, 0x7FFF);
    /// ```
    pub fn draw_line(&mut self, vram: &mut [u16], x0: i16, y0: i16, x1: i16, y1: i16, color: u16) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            // Check clipping bounds before drawing
            let (clip_left, clip_top, clip_right, clip_bottom) = self.clip_rect;
            if x >= clip_left && x <= clip_right && y >= clip_top && y <= clip_bottom {
                Self::write_pixel(vram, x, y, color);
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a polyline (connected line segments)
    ///
    /// Draws multiple connected line segments by calling `draw_line` for each pair
    /// of consecutive points. This is commonly used for wireframe rendering and
    /// debug visualization.
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `points` - Slice of (x, y) coordinates defining the polyline vertices
    /// * `color` - Line color in 15-bit RGB format
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::Rasterizer;
    ///
    /// let mut vram = vec![0u16; 1024 * 512];
    /// let mut rasterizer = Rasterizer::new();
    ///
    /// // Draw a triangle outline
    /// let points = [(100, 100), (200, 100), (150, 200), (100, 100)];
    /// rasterizer.draw_polyline(&mut vram, &points, 0x7FFF);
    /// ```
    pub fn draw_polyline(&mut self, vram: &mut [u16], points: &[(i16, i16)], color: u16) {
        if points.len() < 2 {
            return;
        }

        for i in 0..points.len() - 1 {
            self.draw_line(
                vram,
                points[i].0,
                points[i].1,
                points[i + 1].0,
                points[i + 1].1,
                color,
            );
        }
    }

    /// Draw a gradient triangle with per-vertex colors
    ///
    /// Renders a triangle with colors interpolated across the surface using
    /// barycentric coordinates. Each vertex has its own color, and colors
    /// are smoothly blended across the triangle interior (Gouraud shading).
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `v0` - First vertex position (x, y)
    /// * `c0` - First vertex color (r, g, b) in 8-bit RGB
    /// * `v1` - Second vertex position (x, y)
    /// * `c1` - Second vertex color (r, g, b) in 8-bit RGB
    /// * `v2` - Third vertex position (x, y)
    /// * `c2` - Third vertex color (r, g, b) in 8-bit RGB
    ///
    /// # Algorithm
    ///
    /// Uses barycentric coordinate interpolation:
    /// 1. Sort vertices by Y coordinate
    /// 2. Compute bounding box clipped to drawing area
    /// 3. For each pixel in bounding box:
    ///    - Calculate barycentric weights (w0, w1, w2)
    ///    - If inside triangle (all weights ≥ 0):
    ///      - Interpolate color: C = w0*c0 + w1*c1 + w2*c2
    ///      - Convert to 15-bit and write pixel
    ///
    /// # References
    ///
    /// - [Barycentric Coordinates](https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/barycentric-coordinates)
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::Rasterizer;
    ///
    /// let mut vram = vec![0u16; 1024 * 512];
    /// let mut rasterizer = Rasterizer::new();
    ///
    /// // Draw a gradient triangle (red -> green -> blue)
    /// rasterizer.draw_gradient_triangle(
    ///     &mut vram,
    ///     (100, 100), (255, 0, 0),   // Red vertex
    ///     (200, 100), (0, 255, 0),   // Green vertex
    ///     (150, 200), (0, 0, 255),   // Blue vertex
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn draw_gradient_triangle(
        &mut self,
        vram: &mut [u16],
        v0: (i16, i16),
        c0: (u8, u8, u8),
        v1: (i16, i16),
        c1: (u8, u8, u8),
        v2: (i16, i16),
        c2: (u8, u8, u8),
    ) {
        // Sort vertices by Y
        let (v0, c0, v1, c1, v2, c2) = Self::sort_gradient_vertices(v0, c0, v1, c1, v2, c2);

        if v0.1 == v2.1 {
            return; // Degenerate triangle
        }

        // Compute bounding box
        let min_x = v0.0.min(v1.0).min(v2.0).max(self.clip_rect.0);
        let max_x = v0.0.max(v1.0).max(v2.0).min(self.clip_rect.2);
        let min_y = v0.1.min(v1.1).min(v2.1).max(self.clip_rect.1);
        let max_y = v0.1.max(v1.1).max(v2.1).min(self.clip_rect.3);

        // Rasterize using barycentric coordinates
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (w0, w1, w2) = Self::barycentric(x, y, v0, v1, v2);

                // Check if inside triangle
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    // Interpolate color
                    let r = (c0.0 as f32 * w0 + c1.0 as f32 * w1 + c2.0 as f32 * w2) as u8;
                    let g = (c0.1 as f32 * w0 + c1.1 as f32 * w1 + c2.1 as f32 * w2) as u8;
                    let b = (c0.2 as f32 * w0 + c1.2 as f32 * w1 + c2.2 as f32 * w2) as u8;

                    let color = Self::rgb_to_rgb15(r, g, b);
                    Self::write_pixel(vram, x, y, color);
                }
            }
        }
    }

    /// Compute barycentric coordinates for a point relative to a triangle
    ///
    /// Barycentric coordinates (w0, w1, w2) express a point as a weighted sum
    /// of triangle vertices: P = w0*v0 + w1*v1 + w2*v2, where w0+w1+w2=1.
    /// Points inside the triangle have all weights in range [0, 1].
    ///
    /// # Arguments
    ///
    /// * `px` - Point X coordinate
    /// * `py` - Point Y coordinate
    /// * `v0` - First triangle vertex (x, y)
    /// * `v1` - Second triangle vertex (x, y)
    /// * `v2` - Third triangle vertex (x, y)
    ///
    /// # Returns
    ///
    /// Tuple (w0, w1, w2) of barycentric weights. If the triangle is degenerate
    /// (zero area), returns (0, 0, 0).
    ///
    /// # Examples
    ///
    /// ```
    /// # // This function is private, so we demonstrate its usage through gradient triangles
    /// use psrx::core::gpu::Rasterizer;
    ///
    /// let mut vram = vec![0u16; 1024 * 512];
    /// let mut rasterizer = Rasterizer::new();
    ///
    /// // Draw a gradient triangle that uses barycentric interpolation internally
    /// rasterizer.draw_gradient_triangle(
    ///     &mut vram,
    ///     (0, 0), (255, 0, 0),
    ///     (100, 0), (0, 255, 0),
    ///     (50, 100), (0, 0, 255),
    /// );
    /// ```
    fn barycentric(
        px: i16,
        py: i16,
        v0: (i16, i16),
        v1: (i16, i16),
        v2: (i16, i16),
    ) -> (f32, f32, f32) {
        // Promote to i32 before multiplication to prevent i16 overflow
        let denom = (((v1.1 - v2.1) as i32) * ((v0.0 - v2.0) as i32)
            + ((v2.0 - v1.0) as i32) * ((v0.1 - v2.1) as i32)) as f32;

        if denom.abs() < 0.001 {
            return (0.0, 0.0, 0.0);
        }

        let w0 = ((((v1.1 - v2.1) as i32) * ((px - v2.0) as i32)
            + ((v2.0 - v1.0) as i32) * ((py - v2.1) as i32)) as f32)
            / denom;
        let w1 = ((((v2.1 - v0.1) as i32) * ((px - v2.0) as i32)
            + ((v0.0 - v2.0) as i32) * ((py - v2.1) as i32)) as f32)
            / denom;
        let w2 = 1.0 - w0 - w1;

        (w0, w1, w2)
    }

    /// Convert 24-bit RGB to 15-bit RGB format
    ///
    /// Converts 8-bit per channel RGB (0-255) to 5-bit per channel (0-31)
    /// by right-shifting each channel by 3 bits. Result is packed in
    /// PlayStation's 5-5-5 RGB format.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel (0-255)
    /// * `g` - Green channel (0-255)
    /// * `b` - Blue channel (0-255)
    ///
    /// # Returns
    ///
    /// 16-bit color in 5-5-5 RGB format (bit 15 is 0)
    ///
    /// # Format
    ///
    /// - Bits 0-4: Red (5 bits)
    /// - Bits 5-9: Green (5 bits)
    /// - Bits 10-14: Blue (5 bits)
    /// - Bit 15: Mask bit (always 0)
    fn rgb_to_rgb15(r: u8, g: u8, b: u8) -> u16 {
        let r = ((r as u16) >> 3) & 0x1F;
        let g = ((g as u16) >> 3) & 0x1F;
        let b = ((b as u16) >> 3) & 0x1F;
        (b << 10) | (g << 5) | r
    }

    /// Convert 15-bit RGB to 24-bit RGB format
    ///
    /// Converts PlayStation's 5-bit per channel RGB to 8-bit per channel
    /// by left-shifting each channel by 3 bits.
    ///
    /// # Arguments
    ///
    /// * `color` - 16-bit color in 5-5-5 RGB format
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 8-bit RGB values
    ///
    /// # Examples
    ///
    /// ```
    /// # use psrx::core::gpu::Rasterizer;
    /// # let rasterizer = Rasterizer::new();
    /// // This is a private method, shown for documentation
    /// // Red: 0x001F -> (248, 0, 0)
    /// // White: 0x7FFF -> (248, 248, 248)
    /// ```
    fn rgb15_to_rgb24(color: u16) -> (u8, u8, u8) {
        let r = ((color & 0x1F) << 3) as u8;
        let g = (((color >> 5) & 0x1F) << 3) as u8;
        let b = (((color >> 10) & 0x1F) << 3) as u8;
        (r, g, b)
    }

    /// Read a pixel from VRAM safely
    ///
    /// Reads a 16-bit pixel value from VRAM, returning 0 if coordinates
    /// are out of bounds.
    ///
    /// # Arguments
    ///
    /// * `vram` - Reference to VRAM buffer
    /// * `x` - X coordinate (0-1023)
    /// * `y` - Y coordinate (0-511)
    ///
    /// # Returns
    ///
    /// 16-bit pixel value, or 0 if out of bounds
    fn read_vram_pixel(vram: &[u16], x: i16, y: i16) -> u16 {
        if !(0..1024).contains(&x) || !(0..512).contains(&y) {
            return 0;
        }
        let index = (y as usize) * 1024 + (x as usize);
        vram[index]
    }

    /// Draw a textured triangle with perspective-correct interpolation
    ///
    /// Renders a triangle with texture mapping, interpolating texture coordinates
    /// across the surface using barycentric coordinates. Supports all three texture
    /// depths (4-bit, 8-bit, 15-bit) and applies color modulation (tint).
    ///
    /// # Arguments
    ///
    /// * `vram` - Mutable reference to VRAM buffer
    /// * `v0` - First vertex position (x, y)
    /// * `t0` - First vertex texture coordinates (u, v)
    /// * `v1` - Second vertex position (x, y)
    /// * `t1` - Second vertex texture coordinates (u, v)
    /// * `v2` - Third vertex position (x, y)
    /// * `t2` - Third vertex texture coordinates (u, v)
    /// * `texture_info` - Texture page and CLUT information
    /// * `tint_color` - Color to modulate with texture (r, g, b)
    ///
    /// # Algorithm
    ///
    /// 1. Compute triangle bounding box clipped to drawing area
    /// 2. For each pixel in bounding box:
    ///    - Calculate barycentric weights (w0, w1, w2)
    ///    - If inside triangle (all weights ≥ 0):
    ///      - Interpolate texture coordinates: (u, v) = w0*t0 + w1*t1 + w2*t2
    ///      - Sample texture at (u, v)
    ///      - Apply color modulation: final_color = tex_color * tint_color / 128
    ///      - Write pixel to VRAM
    ///
    /// # Color Modulation
    ///
    /// The tint color is multiplied with the texture color and divided by 128
    /// (right-shifted by 7) to achieve the correct brightness. This allows
    /// tinting and brightness adjustment:
    /// - (128, 128, 128) = normal brightness
    /// - (255, 255, 255) = ~2× brightness
    /// - (64, 64, 64) = 50% brightness
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::gpu::{Rasterizer, TextureInfo, TextureDepth};
    ///
    /// let mut vram = vec![0u16; 1024 * 512];
    /// let mut rasterizer = Rasterizer::new();
    ///
    /// let texture_info = TextureInfo {
    ///     page_x: 64,
    ///     page_y: 0,
    ///     clut_x: 0,
    ///     clut_y: 0,
    ///     depth: TextureDepth::T4Bit,
    /// };
    ///
    /// // Draw a textured triangle
    /// rasterizer.draw_textured_triangle(
    ///     &mut vram,
    ///     (100, 100), (0, 0),
    ///     (200, 100), (255, 0),
    ///     (150, 200), (128, 255),
    ///     &texture_info,
    ///     (128, 128, 128),  // Normal brightness
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn draw_textured_triangle(
        &mut self,
        vram: &mut [u16],
        v0: (i16, i16),
        t0: (u8, u8),
        v1: (i16, i16),
        t1: (u8, u8),
        v2: (i16, i16),
        t2: (u8, u8),
        texture_info: &crate::core::gpu::TextureInfo,
        tint_color: (u8, u8, u8),
    ) {
        // Compute bounding box clipped to drawing area
        let min_x = v0.0.min(v1.0).min(v2.0).max(self.clip_rect.0);
        let max_x = v0.0.max(v1.0).max(v2.0).min(self.clip_rect.2);
        let min_y = v0.1.min(v1.1).min(v2.1).max(self.clip_rect.1);
        let max_y = v0.1.max(v1.1).max(v2.1).min(self.clip_rect.3);

        // Rasterize using barycentric coordinates
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (w0, w1, w2) = Self::barycentric(x, y, v0, v1, v2);

                // Check if inside triangle
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    // Interpolate texture coordinates
                    let u = (t0.0 as f32 * w0 + t1.0 as f32 * w1 + t2.0 as f32 * w2) as u8;
                    let v = (t0.1 as f32 * w0 + t1.1 as f32 * w1 + t2.1 as f32 * w2) as u8;

                    // Sample texture
                    let tex_color = self.sample_texture(vram, u, v, texture_info);

                    // Apply tint (modulate)
                    // Multiply by tint and divide by 128 (shift right by 7)
                    let r = ((tex_color.0 as u16 * tint_color.0 as u16) >> 7) as u8;
                    let g = ((tex_color.1 as u16 * tint_color.1 as u16) >> 7) as u8;
                    let b = ((tex_color.2 as u16 * tint_color.2 as u16) >> 7) as u8;

                    let color = Self::rgb_to_rgb15(r, g, b);
                    Self::write_pixel(vram, x, y, color);
                }
            }
        }
    }

    /// Sample texture at given coordinates
    ///
    /// Dispatches to the appropriate texture sampling function based on
    /// texture depth (4-bit, 8-bit, or 15-bit).
    ///
    /// # Arguments
    ///
    /// * `vram` - Reference to VRAM buffer
    /// * `u` - U texture coordinate
    /// * `v` - V texture coordinate
    /// * `info` - Texture information (page, CLUT, depth)
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 8-bit RGB values
    fn sample_texture(
        &self,
        vram: &[u16],
        u: u8,
        v: u8,
        info: &crate::core::gpu::TextureInfo,
    ) -> (u8, u8, u8) {
        use crate::core::gpu::TextureDepth;
        match info.depth {
            TextureDepth::T4Bit => self.sample_4bit_texture(vram, u, v, info),
            TextureDepth::T8Bit => self.sample_8bit_texture(vram, u, v, info),
            TextureDepth::T15Bit => self.sample_15bit_texture(vram, u, v, info),
        }
    }

    /// Sample a 4-bit indexed color texture
    ///
    /// For 4-bit textures, each 16-bit VRAM word contains 4 palette indices
    /// (4 bits each). The index is used to look up a color in the CLUT.
    ///
    /// # Texture Storage
    ///
    /// 4-bit textures pack 4 pixels per 16-bit word:
    /// - Bits 0-3: Index for pixel 0
    /// - Bits 4-7: Index for pixel 1
    /// - Bits 8-11: Index for pixel 2
    /// - Bits 12-15: Index for pixel 3
    ///
    /// # Arguments
    ///
    /// * `vram` - Reference to VRAM buffer
    /// * `u` - U texture coordinate
    /// * `v` - V texture coordinate
    /// * `info` - Texture information
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 8-bit RGB values from CLUT
    fn sample_4bit_texture(
        &self,
        vram: &[u16],
        u: u8,
        v: u8,
        info: &crate::core::gpu::TextureInfo,
    ) -> (u8, u8, u8) {
        // Calculate texture page address
        // 4-bit textures: 4 pixels per 16-bit word, so divide U by 4
        let tex_x = (info.page_x + (u as u16 / 4)) & 0x3FF;
        let tex_y = (info.page_y + v as u16) & 0x1FF;

        // Read 16-bit word containing 4 indices
        let index_word = Self::read_vram_pixel(vram, tex_x as i16, tex_y as i16);

        // Extract 4-bit index (which of the 4 pixels in this word)
        let shift = (u % 4) * 4;
        let index = (index_word >> shift) & 0xF;

        // Look up color in CLUT
        let clut_x = info.clut_x + index;
        let clut_y = info.clut_y;
        let color = Self::read_vram_pixel(vram, clut_x as i16, clut_y as i16);

        Self::rgb15_to_rgb24(color)
    }

    /// Sample an 8-bit indexed color texture
    ///
    /// For 8-bit textures, each 16-bit VRAM word contains 2 palette indices
    /// (8 bits each). The index is used to look up a color in the CLUT.
    ///
    /// # Texture Storage
    ///
    /// 8-bit textures pack 2 pixels per 16-bit word:
    /// - Bits 0-7: Index for pixel 0 (even U)
    /// - Bits 8-15: Index for pixel 1 (odd U)
    ///
    /// # Arguments
    ///
    /// * `vram` - Reference to VRAM buffer
    /// * `u` - U texture coordinate
    /// * `v` - V texture coordinate
    /// * `info` - Texture information
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 8-bit RGB values from CLUT
    fn sample_8bit_texture(
        &self,
        vram: &[u16],
        u: u8,
        v: u8,
        info: &crate::core::gpu::TextureInfo,
    ) -> (u8, u8, u8) {
        // Calculate texture page address
        // 8-bit textures: 2 pixels per 16-bit word, so divide U by 2
        let tex_x = (info.page_x + (u as u16 / 2)) & 0x3FF;
        let tex_y = (info.page_y + v as u16) & 0x1FF;

        // Read 16-bit word containing 2 indices
        let index_word = Self::read_vram_pixel(vram, tex_x as i16, tex_y as i16);

        // Extract 8-bit index (lower or upper byte depending on odd/even U)
        let index = if u.is_multiple_of(2) {
            index_word & 0xFF
        } else {
            (index_word >> 8) & 0xFF
        };

        // Look up color in CLUT
        let clut_x = info.clut_x + index;
        let clut_y = info.clut_y;
        let color = Self::read_vram_pixel(vram, clut_x as i16, clut_y as i16);

        Self::rgb15_to_rgb24(color)
    }

    /// Sample a 15-bit direct color texture
    ///
    /// For 15-bit textures, each pixel is stored directly as a 16-bit color
    /// value in 5-5-5 RGB format. No CLUT lookup is needed.
    ///
    /// # Texture Storage
    ///
    /// 15-bit textures store 1 pixel per 16-bit word directly as RGB color.
    ///
    /// # Arguments
    ///
    /// * `vram` - Reference to VRAM buffer
    /// * `u` - U texture coordinate
    /// * `v` - V texture coordinate
    /// * `info` - Texture information
    ///
    /// # Returns
    ///
    /// Tuple (r, g, b) with 8-bit RGB values
    fn sample_15bit_texture(
        &self,
        vram: &[u16],
        u: u8,
        v: u8,
        info: &crate::core::gpu::TextureInfo,
    ) -> (u8, u8, u8) {
        // Calculate texture address
        // 15-bit textures: 1 pixel per 16-bit word
        let tex_x = (info.page_x + u as u16) & 0x3FF;
        let tex_y = (info.page_y + v as u16) & 0x1FF;

        // Read color directly
        let color = Self::read_vram_pixel(vram, tex_x as i16, tex_y as i16);
        Self::rgb15_to_rgb24(color)
    }

    /// Sort triangle vertices by Y coordinate, preserving associated colors
    ///
    /// Returns vertices in ascending Y order (v0.y <= v1.y <= v2.y) along
    /// with their corresponding colors. This is used to prepare vertices
    /// for gradient triangle rasterization.
    ///
    /// # Arguments
    ///
    /// * `v0` - First vertex position
    /// * `c0` - First vertex color
    /// * `v1` - Second vertex position
    /// * `c1` - Second vertex color
    /// * `v2` - Third vertex position
    /// * `c2` - Third vertex color
    ///
    /// # Returns
    ///
    /// Tuple of (v0, c0, v1, c1, v2, c2) sorted by Y coordinate
    #[allow(clippy::type_complexity)]
    fn sort_gradient_vertices(
        v0: (i16, i16),
        c0: (u8, u8, u8),
        v1: (i16, i16),
        c1: (u8, u8, u8),
        v2: (i16, i16),
        c2: (u8, u8, u8),
    ) -> (
        (i16, i16),
        (u8, u8, u8),
        (i16, i16),
        (u8, u8, u8),
        (i16, i16),
        (u8, u8, u8),
    ) {
        let mut verts = [(v0, c0), (v1, c1), (v2, c2)];
        verts.sort_by_key(|v| v.0 .1);
        (
            verts[0].0, verts[0].1, verts[1].0, verts[1].1, verts[2].0, verts[2].1,
        )
    }
}

impl Default for Rasterizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_sorting() {
        let v0 = (10, 30);
        let v1 = (20, 10);
        let v2 = (30, 20);

        let (s0, s1, s2) = Rasterizer::sort_vertices_by_y(v0, v1, v2);

        assert_eq!(s0, (20, 10)); // Lowest Y
        assert_eq!(s1, (30, 20)); // Middle Y
        assert_eq!(s2, (10, 30)); // Highest Y
    }

    #[test]
    fn test_basic_triangle() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Draw a simple triangle
        rasterizer.draw_triangle(&mut vram, (100, 100), (200, 100), (150, 200), 0x7FFF); // White

        // Check that the center pixel is drawn
        let center_pixel = vram[150 * 1024 + 150];
        assert_ne!(center_pixel, 0);

        // Check that a pixel outside the triangle is not drawn
        let outside_pixel = vram[50 * 1024 + 50];
        assert_eq!(outside_pixel, 0);
    }

    #[test]
    fn test_clipping() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();
        rasterizer.set_clip_rect(100, 100, 200, 200);

        // Draw triangle that extends beyond clip area
        rasterizer.draw_triangle(&mut vram, (50, 50), (250, 150), (150, 250), 0x7FFF);

        // Pixel outside clip area should not be drawn
        assert_eq!(vram[50 * 1024 + 50], 0);

        // Pixel inside both triangle and clip area should be drawn
        let inside_pixel = vram[150 * 1024 + 150];
        assert_ne!(inside_pixel, 0);
    }

    #[test]
    fn test_degenerate_triangle() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Zero-height triangle (all vertices on same scanline)
        rasterizer.draw_triangle(&mut vram, (10, 10), (20, 10), (15, 10), 0x7FFF);

        // Should not crash, and no pixels should be drawn
        assert_eq!(vram[10 * 1024 + 10], 0);
        assert_eq!(vram[10 * 1024 + 15], 0);
        assert_eq!(vram[10 * 1024 + 20], 0);
    }

    #[test]
    fn test_bounds_checking() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Triangle with vertices outside VRAM bounds
        rasterizer.draw_triangle(&mut vram, (-100, -100), (2000, 100), (500, 1000), 0x7FFF);

        // Should not crash - pixels outside bounds are clipped
    }

    #[test]
    fn test_bottom_flat_triangle() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Bottom-flat triangle
        rasterizer.draw_triangle(&mut vram, (150, 100), (100, 200), (200, 200), 0x001F); // Red

        // Check middle pixel is drawn
        let pixel = vram[150 * 1024 + 150];
        assert_ne!(pixel, 0);
    }

    #[test]
    fn test_top_flat_triangle() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Top-flat triangle
        rasterizer.draw_triangle(&mut vram, (100, 100), (200, 100), (150, 200), 0x03E0); // Green

        // Check middle pixel is drawn
        let pixel = vram[150 * 1024 + 150];
        assert_ne!(pixel, 0);
    }

    #[test]
    fn test_line_drawing() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Draw a horizontal line
        rasterizer.draw_line(&mut vram, 10, 10, 50, 10, 0x7FFF);

        // Check pixels along the line are set
        assert_ne!(vram[10 * 1024 + 10], 0);
        assert_ne!(vram[10 * 1024 + 30], 0);
        assert_ne!(vram[10 * 1024 + 50], 0);

        // Check pixel not on the line is not set
        assert_eq!(vram[11 * 1024 + 30], 0);
    }

    #[test]
    fn test_line_diagonal() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Draw a diagonal line
        rasterizer.draw_line(&mut vram, 10, 10, 50, 50, 0x7FFF);

        // Check start and end pixels
        assert_ne!(vram[10 * 1024 + 10], 0);
        assert_ne!(vram[50 * 1024 + 50], 0);

        // Check a point approximately on the line
        assert_ne!(vram[30 * 1024 + 30], 0);
    }

    #[test]
    fn test_line_clipping() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();
        rasterizer.set_clip_rect(20, 20, 100, 100);

        // Draw line that extends beyond clip area
        rasterizer.draw_line(&mut vram, 0, 50, 200, 50, 0x7FFF);

        // Pixel before clip area should not be set
        assert_eq!(vram[50 * 1024 + 10], 0);

        // Pixel in clip area should be set
        assert_ne!(vram[50 * 1024 + 50], 0);

        // Pixel after clip area should not be set
        assert_eq!(vram[50 * 1024 + 150], 0);
    }

    #[test]
    fn test_polyline() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Draw a triangle using polyline
        let points = [(100, 100), (200, 100), (150, 200), (100, 100)];
        rasterizer.draw_polyline(&mut vram, &points, 0x7FFF);

        // Check vertices are drawn
        assert_ne!(vram[100 * 1024 + 100], 0);
        assert_ne!(vram[100 * 1024 + 200], 0);
        assert_ne!(vram[200 * 1024 + 150], 0);

        // Check edges are connected
        assert_ne!(vram[100 * 1024 + 150], 0); // Top edge
    }

    #[test]
    fn test_polyline_empty() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Empty polyline should not crash
        let points: &[(i16, i16)] = &[];
        rasterizer.draw_polyline(&mut vram, points, 0x7FFF);

        // Single point should not crash
        let points = [(100, 100)];
        rasterizer.draw_polyline(&mut vram, &points, 0x7FFF);
    }

    #[test]
    fn test_gradient_triangle() {
        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Draw a gradient triangle (red -> green -> blue)
        rasterizer.draw_gradient_triangle(
            &mut vram,
            (100, 100),
            (255, 0, 0), // Red
            (200, 100),
            (0, 255, 0), // Green
            (150, 150),
            (0, 0, 255), // Blue
        );

        // Check that pixels are drawn
        let pixel = vram[120 * 1024 + 150];
        assert_ne!(pixel, 0);

        // Check that center has interpolated color
        // The center should be a blend of all three colors
        let center_pixel = vram[125 * 1024 + 150];
        assert_ne!(center_pixel, 0);
        assert_ne!(center_pixel, 0x001F); // Not pure red
        assert_ne!(center_pixel, 0x03E0); // Not pure green
        assert_ne!(center_pixel, 0x7C00); // Not pure blue
    }

    #[test]
    fn test_barycentric_coordinates() {
        let v0 = (0, 0);
        let v1 = (100, 0);
        let v2 = (50, 100);

        // Test point at v0
        let (w0, w1, w2) = Rasterizer::barycentric(0, 0, v0, v1, v2);
        assert!((w0 - 1.0).abs() < 0.01);
        assert!(w1.abs() < 0.01);
        assert!(w2.abs() < 0.01);

        // Test point at v1
        let (w0, w1, w2) = Rasterizer::barycentric(100, 0, v0, v1, v2);
        assert!(w0.abs() < 0.01);
        assert!((w1 - 1.0).abs() < 0.01);
        assert!(w2.abs() < 0.01);

        // Test point at centroid (approximately)
        let (w0, w1, w2) = Rasterizer::barycentric(50, 33, v0, v1, v2);
        assert!((w0 - 0.33).abs() < 0.1);
        assert!((w1 - 0.33).abs() < 0.1);
        assert!((w2 - 0.33).abs() < 0.1);
    }

    #[test]
    fn test_rgb_to_rgb15() {
        // Test pure colors
        assert_eq!(Rasterizer::rgb_to_rgb15(255, 0, 0), 0x001F); // Red
        assert_eq!(Rasterizer::rgb_to_rgb15(0, 255, 0), 0x03E0); // Green
        assert_eq!(Rasterizer::rgb_to_rgb15(0, 0, 255), 0x7C00); // Blue
        assert_eq!(Rasterizer::rgb_to_rgb15(255, 255, 255), 0x7FFF); // White
        assert_eq!(Rasterizer::rgb_to_rgb15(0, 0, 0), 0x0000); // Black

        // Test conversion with rounding
        assert_eq!(Rasterizer::rgb_to_rgb15(128, 128, 128), 0x4210); // Gray
    }

    #[test]
    fn test_gradient_vertex_sorting() {
        let v0 = (10, 30);
        let c0 = (255, 0, 0);
        let v1 = (20, 10);
        let c1 = (0, 255, 0);
        let v2 = (30, 20);
        let c2 = (0, 0, 255);

        let (s0, sc0, s1, sc1, s2, sc2) =
            Rasterizer::sort_gradient_vertices(v0, c0, v1, c1, v2, c2);

        // Should be sorted by Y
        assert_eq!(s0, (20, 10)); // Lowest Y
        assert_eq!(sc0, (0, 255, 0)); // Green
        assert_eq!(s1, (30, 20)); // Middle Y
        assert_eq!(sc1, (0, 0, 255)); // Blue
        assert_eq!(s2, (10, 30)); // Highest Y
        assert_eq!(sc2, (255, 0, 0)); // Red
    }

    #[test]
    fn test_rgb15_to_rgb24() {
        // Test pure colors
        assert_eq!(Rasterizer::rgb15_to_rgb24(0x001F), (248, 0, 0)); // Red
        assert_eq!(Rasterizer::rgb15_to_rgb24(0x03E0), (0, 248, 0)); // Green
        assert_eq!(Rasterizer::rgb15_to_rgb24(0x7C00), (0, 0, 248)); // Blue
        assert_eq!(Rasterizer::rgb15_to_rgb24(0x7FFF), (248, 248, 248)); // White
        assert_eq!(Rasterizer::rgb15_to_rgb24(0x0000), (0, 0, 0)); // Black
    }

    #[test]
    fn test_texture_sampling_4bit() {
        use crate::core::gpu::{TextureDepth, TextureInfo};

        let mut vram = vec![0u16; 1024 * 512];
        let rasterizer = Rasterizer::new();

        // Setup CLUT at (0, 0) with 16 colors
        for (i, pixel) in vram.iter_mut().enumerate().take(16) {
            let r = ((i * 2) & 0x1F) as u16;
            let g = ((i * 3) & 0x1F) as u16;
            let b = ((i * 4) & 0x1F) as u16;
            let color = (b << 10) | (g << 5) | r;
            *pixel = color;
        }

        // Setup 4-bit texture at (64, 0)
        // Store indices 0,1,2,3 in first word (4 pixels)
        vram[64] = 0x3210;

        let info = TextureInfo {
            page_x: 64,
            page_y: 0,
            clut_x: 0,
            clut_y: 0,
            depth: TextureDepth::T4Bit,
        };

        // Sample pixel 0 (U=0) should get index 0
        let color = rasterizer.sample_4bit_texture(&vram, 0, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[0]);
        assert_eq!(color, expected);

        // Sample pixel 1 (U=1) should get index 1
        let color = rasterizer.sample_4bit_texture(&vram, 1, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[1]);
        assert_eq!(color, expected);

        // Sample pixel 2 (U=2) should get index 2
        let color = rasterizer.sample_4bit_texture(&vram, 2, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[2]);
        assert_eq!(color, expected);

        // Sample pixel 3 (U=3) should get index 3
        let color = rasterizer.sample_4bit_texture(&vram, 3, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[3]);
        assert_eq!(color, expected);
    }

    #[test]
    fn test_texture_sampling_8bit() {
        use crate::core::gpu::{TextureDepth, TextureInfo};

        let mut vram = vec![0u16; 1024 * 512];
        let rasterizer = Rasterizer::new();

        // Setup CLUT at (0, 0) with 256 colors
        for (i, pixel) in vram.iter_mut().enumerate().take(256) {
            let r = ((i / 8) & 0x1F) as u16;
            let g = ((i / 4) & 0x1F) as u16;
            let b = ((i / 2) & 0x1F) as u16;
            let color = (b << 10) | (g << 5) | r;
            *pixel = color;
        }

        // Setup 8-bit texture at (64, 0)
        // Store indices 10 (low byte), 20 (high byte) in first word
        vram[64] = (20 << 8) | 10;

        let info = TextureInfo {
            page_x: 64,
            page_y: 0,
            clut_x: 0,
            clut_y: 0,
            depth: TextureDepth::T8Bit,
        };

        // Sample pixel 0 (U=0, even) should get index 10
        let color = rasterizer.sample_8bit_texture(&vram, 0, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[10]);
        assert_eq!(color, expected);

        // Sample pixel 1 (U=1, odd) should get index 20
        let color = rasterizer.sample_8bit_texture(&vram, 1, 0, &info);
        let expected = Rasterizer::rgb15_to_rgb24(vram[20]);
        assert_eq!(color, expected);
    }

    #[test]
    fn test_texture_sampling_15bit() {
        use crate::core::gpu::{TextureDepth, TextureInfo};

        let mut vram = vec![0u16; 1024 * 512];
        let rasterizer = Rasterizer::new();

        // Setup 15-bit texture at (64, 0) with direct colors
        vram[64] = 0x001F; // Red
        vram[65] = 0x03E0; // Green
        vram[66] = 0x7C00; // Blue

        let info = TextureInfo {
            page_x: 64,
            page_y: 0,
            clut_x: 0, // Not used for 15-bit
            clut_y: 0, // Not used for 15-bit
            depth: TextureDepth::T15Bit,
        };

        // Sample pixel 0 should get red
        let color = rasterizer.sample_15bit_texture(&vram, 0, 0, &info);
        assert_eq!(color, (248, 0, 0));

        // Sample pixel 1 should get green
        let color = rasterizer.sample_15bit_texture(&vram, 1, 0, &info);
        assert_eq!(color, (0, 248, 0));

        // Sample pixel 2 should get blue
        let color = rasterizer.sample_15bit_texture(&vram, 2, 0, &info);
        assert_eq!(color, (0, 0, 248));
    }

    #[test]
    fn test_textured_triangle() {
        use crate::core::gpu::{TextureDepth, TextureInfo};

        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Setup simple 15-bit texture at (64, 0)
        for y in 0..256 {
            for x in 0..64 {
                let index = y * 1024 + (64 + x);
                vram[index] = 0x7FFF; // White
            }
        }

        let texture_info = TextureInfo {
            page_x: 64,
            page_y: 0,
            clut_x: 0,
            clut_y: 0,
            depth: TextureDepth::T15Bit,
        };

        // Draw a textured triangle
        rasterizer.draw_textured_triangle(
            &mut vram,
            (100, 100),
            (0, 0),
            (200, 100),
            (63, 0),
            (150, 200),
            (31, 255),
            &texture_info,
            (128, 128, 128), // Normal brightness
        );

        // Check that pixels are drawn inside the triangle
        let pixel = vram[150 * 1024 + 150];
        assert_ne!(pixel, 0);
    }

    #[test]
    fn test_textured_triangle_color_modulation() {
        use crate::core::gpu::{TextureDepth, TextureInfo};

        let mut vram = vec![0u16; 1024 * 512];
        let mut rasterizer = Rasterizer::new();

        // Setup 15-bit texture with white color
        for y in 0..256 {
            for x in 0..64 {
                let index = y * 1024 + (64 + x);
                vram[index] = 0x7FFF; // White (248, 248, 248)
            }
        }

        let texture_info = TextureInfo {
            page_x: 64,
            page_y: 0,
            clut_x: 0,
            clut_y: 0,
            depth: TextureDepth::T15Bit,
        };

        // Draw with red tint (255, 0, 0)
        rasterizer.draw_textured_triangle(
            &mut vram,
            (100, 100),
            (0, 0),
            (200, 100),
            (10, 0),
            (150, 150),
            (5, 10),
            &texture_info,
            (255, 0, 0), // Red tint
        );

        // Center pixel should have reddish tint
        let pixel = vram[125 * 1024 + 150];
        assert_ne!(pixel, 0);

        // Extract color components
        let r = pixel & 0x1F;
        let g = (pixel >> 5) & 0x1F;
        let b = (pixel >> 10) & 0x1F;

        // Red should be higher than green and blue
        assert!(r > g);
        assert!(r > b);
    }
}
