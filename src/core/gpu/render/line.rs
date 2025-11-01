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

//! Line rendering implementation
//!
//! Implements line and polyline rasterization using Bresenham's algorithm.

use super::super::types::{Color, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a monochrome line
    ///
    /// Applies the drawing offset to both vertices and rasterizes the line
    /// using Bresenham's algorithm.
    ///
    /// # Arguments
    ///
    /// * `v0` - Start vertex
    /// * `v1` - End vertex
    /// * `color` - Line color
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Notes
    ///
    /// Semi-transparency is currently ignored (will be implemented in #36).
    /// The drawing offset is applied to both endpoints before rasterization.
    pub(in crate::core::gpu) fn render_line(
        &mut self,
        v0: Vertex,
        v1: Vertex,
        color: Color,
        semi_transparent: bool,
    ) {
        // Apply drawing offset
        let x0 = v0.x.wrapping_add(self.draw_offset.0);
        let y0 = v0.y.wrapping_add(self.draw_offset.1);
        let x1 = v1.x.wrapping_add(self.draw_offset.0);
        let y1 = v1.y.wrapping_add(self.draw_offset.1);

        log::trace!(
            "Rendering {}line: ({}, {}) -> ({}, {}) color=({},{},{})",
            if semi_transparent {
                "semi-transparent "
            } else {
                ""
            },
            x0,
            y0,
            x1,
            y1,
            color.r,
            color.g,
            color.b
        );

        // Convert color to 15-bit RGB format
        let color_15bit = color.to_rgb15();

        // For now, ignore semi_transparent (will be implemented in #36)
        let _ = semi_transparent;

        // Rasterize the line
        self.rasterizer
            .draw_line(&mut self.vram, x0, y0, x1, y1, color_15bit);
    }

    /// Render a polyline (connected line segments)
    ///
    /// Applies the drawing offset to all vertices and draws connected line
    /// segments between consecutive vertices.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Slice of vertices defining the polyline
    /// * `color` - Line color
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Notes
    ///
    /// Requires at least 2 vertices. If fewer than 2 vertices are provided,
    /// no drawing occurs.
    pub(in crate::core::gpu) fn render_polyline(
        &mut self,
        vertices: &[Vertex],
        color: Color,
        semi_transparent: bool,
    ) {
        if vertices.len() < 2 {
            return;
        }

        log::trace!(
            "Rendering {}polyline with {} vertices, color=({},{},{})",
            if semi_transparent {
                "semi-transparent "
            } else {
                ""
            },
            vertices.len(),
            color.r,
            color.g,
            color.b
        );

        // Convert color to 15-bit RGB format
        let color_15bit = color.to_rgb15();

        // For now, ignore semi_transparent (will be implemented in #36)
        let _ = semi_transparent;

        // Apply drawing offset to all vertices
        let points: Vec<(i16, i16)> = vertices
            .iter()
            .map(|v| {
                (
                    v.x.wrapping_add(self.draw_offset.0),
                    v.y.wrapping_add(self.draw_offset.1),
                )
            })
            .collect();

        // Rasterize the polyline
        self.rasterizer
            .draw_polyline(&mut self.vram, &points, color_15bit);
    }
}
