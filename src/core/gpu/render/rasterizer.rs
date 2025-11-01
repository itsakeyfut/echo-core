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
            let v3_x =
                v0.0 + ((v1.1 - v0.1) as i32 * (v2.0 - v0.0) as i32 / (v2.1 - v0.1) as i32) as i16;
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
}
