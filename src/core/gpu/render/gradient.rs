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

//! Gradient (Gouraud-shaded) rendering implementation
//!
//! Implements gradient triangle and quad rasterization with per-vertex colors.

use super::super::types::{Color, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a gradient (Gouraud-shaded) triangle
    ///
    /// Applies the drawing offset to all vertices and rasterizes the triangle
    /// with color interpolation using barycentric coordinates.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `colors` - Array of 3 colors, one per vertex
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Algorithm
    ///
    /// Colors are interpolated across the triangle interior using barycentric
    /// coordinates, providing smooth Gouraud shading. This creates a gradient
    /// effect commonly used for lighting.
    ///
    /// # Notes
    ///
    /// Semi-transparency is currently ignored (will be implemented in #36).
    /// The drawing offset is applied to all vertices before rasterization.
    pub(in crate::core::gpu) fn render_gradient_triangle(
        &mut self,
        vertices: &[Vertex; 3],
        colors: &[Color; 3],
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
            "Rendering {}gradient triangle: ({}, {}), ({}, {}), ({}, {}) colors=({},{},{}), ({},{},{}), ({},{},{})",
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
            colors[0].r,
            colors[0].g,
            colors[0].b,
            colors[1].r,
            colors[1].g,
            colors[1].b,
            colors[2].r,
            colors[2].g,
            colors[2].b
        );

        let c0 = (colors[0].r, colors[0].g, colors[0].b);
        let c1 = (colors[1].r, colors[1].g, colors[1].b);
        let c2 = (colors[2].r, colors[2].g, colors[2].b);

        // For now, ignore semi_transparent (will be implemented in #36)
        let _ = semi_transparent;

        // Rasterize the gradient triangle
        self.rasterizer
            .draw_gradient_triangle(&mut self.vram, v0, c0, v1, c1, v2, c2);
    }

    /// Render a gradient (Gouraud-shaded) quadrilateral
    ///
    /// Renders a quad as two triangles with gradient shading. The quad is
    /// split into triangles (v0, v1, v2) and (v1, v2, v3).
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 4 vertices defining the quad
    /// * `colors` - Array of 4 colors, one per vertex
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Notes
    ///
    /// The quad is rendered as two gradient triangles. Colors are interpolated
    /// independently for each triangle, which may create a visible seam if the
    /// quad is not coplanar in 3D space.
    pub(in crate::core::gpu) fn render_gradient_quad(
        &mut self,
        vertices: &[Vertex; 4],
        colors: &[Color; 4],
        semi_transparent: bool,
    ) {
        log::trace!(
            "Rendering {}gradient quad as two triangles",
            if semi_transparent {
                "semi-transparent "
            } else {
                ""
            }
        );

        // Render as two triangles: (v0, v1, v2) and (v1, v2, v3)
        self.render_gradient_triangle(
            &[vertices[0], vertices[1], vertices[2]],
            &[colors[0], colors[1], colors[2]],
            semi_transparent,
        );

        self.render_gradient_triangle(
            &[vertices[1], vertices[2], vertices[3]],
            &[colors[1], colors[2], colors[3]],
            semi_transparent,
        );
    }
}
