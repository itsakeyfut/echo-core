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
    /// Applies the drawing offset to all vertices and logs the rendering operation.
    /// Actual rasterization will be implemented in issue #33.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `color` - Flat color for the entire triangle
    /// * `semi_transparent` - Whether semi-transparency is enabled
    pub(in crate::core::gpu) fn render_monochrome_triangle(
        &mut self,
        vertices: &[Vertex; 3],
        color: &Color,
        semi_transparent: bool,
    ) {
        // Apply drawing offset
        let v0 = Vertex {
            x: vertices[0].x.wrapping_add(self.draw_offset.0),
            y: vertices[0].y.wrapping_add(self.draw_offset.1),
        };
        let v1 = Vertex {
            x: vertices[1].x.wrapping_add(self.draw_offset.0),
            y: vertices[1].y.wrapping_add(self.draw_offset.1),
        };
        let v2 = Vertex {
            x: vertices[2].x.wrapping_add(self.draw_offset.0),
            y: vertices[2].y.wrapping_add(self.draw_offset.1),
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
}
