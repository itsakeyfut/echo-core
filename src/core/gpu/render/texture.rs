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

//! Textured primitive rendering implementation
//!
//! Implements texture-mapped triangle and quadrilateral rasterization with support
//! for 4-bit, 8-bit, and 15-bit texture formats.

use super::super::primitives::{Color, TexCoord, TextureInfo, Vertex};
use super::super::GPU;

impl GPU {
    /// Render a textured triangle
    ///
    /// Applies the drawing offset to all vertices and rasterizes the triangle
    /// with texture mapping using the software rasterizer.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 3 vertices defining the triangle
    /// * `texcoords` - Array of 3 texture coordinates corresponding to vertices
    /// * `texture_info` - Texture page and CLUT information
    /// * `color` - Color tint to modulate with texture
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Texture Mapping
    ///
    /// The texture coordinates are interpolated across the triangle using
    /// barycentric coordinates. The texture is sampled from VRAM at the
    /// specified texture page with the given color depth (4-bit, 8-bit, or 15-bit).
    ///
    /// # Color Modulation
    ///
    /// The color parameter acts as a tint/modulation color that is multiplied
    /// with the sampled texture color. For normal brightness, use (128, 128, 128).
    ///
    /// # Notes
    ///
    /// Semi-transparency is currently ignored (will be implemented in issue #36).
    /// The drawing offset is applied to all vertices before rasterization.
    pub(in crate::core::gpu) fn render_textured_triangle(
        &mut self,
        vertices: &[Vertex; 3],
        texcoords: &[TexCoord; 3],
        texture_info: &TextureInfo,
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

        let t0 = (texcoords[0].u, texcoords[0].v);
        let t1 = (texcoords[1].u, texcoords[1].v);
        let t2 = (texcoords[2].u, texcoords[2].v);

        log::trace!(
            "Rendering {}textured triangle: v=({},{}),({},{}),({},{}) t=({},{}),({},{}),({},{}) color=({},{},{})",
            if semi_transparent { "semi-transparent " } else { "" },
            v0.0, v0.1, v1.0, v1.1, v2.0, v2.1,
            t0.0, t0.1, t1.0, t1.1, t2.0, t2.1,
            color.r, color.g, color.b
        );

        // For now, ignore semi_transparent (will be implemented in #36)
        let _ = semi_transparent;

        // Rasterize the textured triangle with texture window
        self.rasterizer.draw_textured_triangle(
            &mut self.vram,
            v0,
            t0,
            v1,
            t1,
            v2,
            t2,
            texture_info,
            &self.texture_window,
            (color.r, color.g, color.b),
        );
    }

    /// Render a textured quadrilateral
    ///
    /// Splits the quad into two triangles and renders them as textured primitives.
    /// The quad is split along the v0-v2 diagonal.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of 4 vertices defining the quad (in order: v0, v1, v2, v3)
    /// * `texcoords` - Array of 4 texture coordinates corresponding to vertices
    /// * `texture_info` - Texture page and CLUT information
    /// * `color` - Color tint to modulate with texture
    /// * `semi_transparent` - Whether semi-transparency is enabled
    ///
    /// # Quad Splitting
    ///
    /// The quad is split into two triangles:
    /// - Triangle 1: (v0, v1, v2)
    /// - Triangle 2: (v1, v2, v3)
    ///
    /// This matches the PlayStation GPU's quadrilateral rendering behavior.
    ///
    /// # Notes
    ///
    /// Semi-transparency is currently ignored (will be implemented in issue #36).
    pub(in crate::core::gpu) fn render_textured_quad(
        &mut self,
        vertices: &[Vertex; 4],
        texcoords: &[TexCoord; 4],
        texture_info: &TextureInfo,
        color: &Color,
        semi_transparent: bool,
    ) {
        // Split quad into two triangles: (v0,v1,v2) and (v1,v2,v3)
        let tri1_verts = [vertices[0], vertices[1], vertices[2]];
        let tri1_texcoords = [texcoords[0], texcoords[1], texcoords[2]];

        let tri2_verts = [vertices[1], vertices[2], vertices[3]];
        let tri2_texcoords = [texcoords[1], texcoords[2], texcoords[3]];

        self.render_textured_triangle(
            &tri1_verts,
            &tri1_texcoords,
            texture_info,
            color,
            semi_transparent,
        );
        self.render_textured_triangle(
            &tri2_verts,
            &tri2_texcoords,
            texture_info,
            color,
            semi_transparent,
        );
    }
}
