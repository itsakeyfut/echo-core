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
//! Implements monochrome (flat-shaded) triangle rasterization.

use super::super::types::{Color, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a monochrome (flat-shaded) triangle
    ///
    /// Applies the drawing offset to all vertices and rasterizes the triangle
    /// using the software rasterizer.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `color` - Flat color for the entire triangle
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Notes
    ///
    /// Semi-transparency is currently ignored (will be implemented in future).
    /// The drawing offset is applied to all vertices before rasterization.
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
            "Rendering {}triangle: ({}, {}), ({}, {}), ({}, {}) color=({},{},{})",
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
            color.b
        );

        // Convert color to 15-bit RGB format
        let color_15bit = color.to_rgb15();

        // For now, ignore semi_transparent (will be implemented in #36)
        let _ = semi_transparent;

        // Rasterize the triangle
        self.rasterizer
            .draw_triangle(&mut self.vram, v0, v1, v2, color_15bit);
    }
}
