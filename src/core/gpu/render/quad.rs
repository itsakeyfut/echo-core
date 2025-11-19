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

//! Quadrilateral rendering implementation
//!
//! Implements monochrome (flat-shaded) quad rasterization by decomposing into triangles.

use super::super::primitives::{Color, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a monochrome (flat-shaded) quadrilateral
    ///
    /// Quads are rendered as two triangles: (v0, v1, v2) and (v0, v2, v3).
    /// Applies the drawing offset and delegates to triangle rendering.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 4 vertices defining the quad (in order)
    /// * `color` - Flat color for the entire quad
    /// * `semi_transparent` - Whether semi-transparency is enabled
    pub(crate) fn render_monochrome_quad(
        &mut self,
        vertices: &[Vertex; 4],
        color: &Color,
        semi_transparent: bool,
    ) {
        // Quads are rendered as two triangles
        let tri1 = [vertices[0], vertices[1], vertices[2]];
        let tri2 = [vertices[0], vertices[2], vertices[3]];

        self.render_monochrome_triangle(&tri1, color, semi_transparent);
        self.render_monochrome_triangle(&tri2, color, semi_transparent);
    }
}
