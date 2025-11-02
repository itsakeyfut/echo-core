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

//! Triangle rendering implementation
//!
//! Implements monochrome (flat-shaded) triangle rasterization with optional semi-transparency.

use super::super::types::{BlendMode, Color, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a monochrome (flat-shaded) triangle
    ///
    /// Applies the drawing offset to all vertices and rasterizes the triangle
    /// using the software rasterizer. If semi-transparency is enabled, the triangle
    /// is blended with the existing background using the current blend mode.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `color` - Flat color for the entire triangle
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Semi-Transparency
    ///
    /// When semi-transparency is enabled, the GPU's current semi-transparency mode
    /// (from draw_mode.semi_transparency) determines the blending formula:
    /// - Mode 0 (Average): 0.5×Background + 0.5×Foreground
    /// - Mode 1 (Additive): 1.0×Background + 1.0×Foreground
    /// - Mode 2 (Subtractive): 1.0×Background - 1.0×Foreground
    /// - Mode 3 (AddQuarter): 1.0×Background + 0.25×Foreground
    ///
    /// # Notes
    ///
    /// The drawing offset is applied to all vertices before rasterization.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // This is a private method used internally by the GPU
    /// use psrx::core::gpu::{GPU, Vertex, Color};
    ///
    /// let mut gpu = GPU::new();
    ///
    /// // Draw opaque red triangle
    /// let vertices = [
    ///     Vertex { x: 100, y: 100 },
    ///     Vertex { x: 200, y: 100 },
    ///     Vertex { x: 150, y: 200 },
    /// ];
    /// let color = Color { r: 255, g: 0, b: 0 };
    /// gpu.render_monochrome_triangle(&vertices, &color, false);
    ///
    /// // Draw semi-transparent black triangle on top
    /// let color2 = Color { r: 0, g: 0, b: 0 };
    /// gpu.render_monochrome_triangle(&vertices, &color2, true);
    /// ```
    pub(in crate::core::gpu) fn render_monochrome_triangle(
        &mut self,
        vertices: &[Vertex; 3],
        color: &Color,
        semi_transparent: bool,
    ) {
        // Apply drawing offset
        let v0 = (
            vertices[0].x.wrapping_add(self.draw_offset.0),
            vertices[0].y.wrapping_add(self.draw_offset.1),
        );
        let v1 = (
            vertices[1].x.wrapping_add(self.draw_offset.0),
            vertices[1].y.wrapping_add(self.draw_offset.1),
        );
        let v2 = (
            vertices[2].x.wrapping_add(self.draw_offset.0),
            vertices[2].y.wrapping_add(self.draw_offset.1),
        );

        log::trace!(
            "Rendering {}triangle: ({}, {}), ({}, {}), ({}, {}) color=({},{},{}){}",
            if semi_transparent {
                "semi-transparent "
            } else {
                ""
            },
            v0.0,
            v0.1,
            v1.0,
            v1.1,
            v2.0,
            v2.1,
            color.r,
            color.g,
            color.b,
            if semi_transparent {
                format!(" mode={}", self.draw_mode.semi_transparency)
            } else {
                String::new()
            }
        );

        // Convert color to 15-bit RGB format
        let color_15bit = color.to_rgb15();

        // Rasterize the triangle with or without blending
        if semi_transparent {
            // Use blending mode from draw_mode
            let blend_mode = BlendMode::from_bits(self.draw_mode.semi_transparency);
            self.rasterizer.draw_triangle_blended(
                &mut self.vram,
                v0,
                v1,
                v2,
                color_15bit,
                blend_mode,
            );
        } else {
            // Opaque rendering
            self.rasterizer
                .draw_triangle(&mut self.vram, v0, v1, v2, color_15bit);
        }
    }
}
