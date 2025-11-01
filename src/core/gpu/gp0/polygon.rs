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

//! GP0 polygon drawing commands
//!
//! Implements parsing for triangle and quadrilateral rendering commands,
//! including both flat-shaded, Gouraud-shaded, and textured primitives.

use super::super::types::{Color, TexCoord, TextureInfo, Vertex};
use super::super::GPU;

impl GPU {
    /// GP0(0x20): Monochrome Triangle (Opaque)
    ///
    /// Renders a flat-shaded triangle with a single color.
    /// Requires 4 words: command+color, vertex1, vertex2, vertex3
    pub(in crate::core::gpu) fn parse_monochrome_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 4 {
            return; // Need more words
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_monochrome_triangle(&vertices, &color, false);
    }

    /// GP0(0x22): Monochrome Triangle (Semi-Transparent)
    ///
    /// Renders a flat-shaded triangle with semi-transparency enabled.
    /// Requires 4 words: command+color, vertex1, vertex2, vertex3
    pub(in crate::core::gpu) fn parse_monochrome_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_monochrome_triangle(&vertices, &color, true);
    }

    /// GP0(0x28): Monochrome Quad (Opaque)
    ///
    /// Renders a flat-shaded quadrilateral with a single color.
    /// Requires 5 words: command+color, vertex1, vertex2, vertex3, vertex4
    pub(in crate::core::gpu) fn parse_monochrome_quad_opaque(&mut self) {
        if self.command_fifo.len() < 5 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_monochrome_quad(&vertices, &color, false);
    }

    /// GP0(0x2A): Monochrome Quad (Semi-Transparent)
    ///
    /// Renders a flat-shaded quadrilateral with semi-transparency enabled.
    /// Requires 5 words: command+color, vertex1, vertex2, vertex3, vertex4
    pub(in crate::core::gpu) fn parse_monochrome_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 5 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_monochrome_quad(&vertices, &color, true);
    }

    /// GP0(0x30): Gouraud-Shaded Triangle (Opaque)
    ///
    /// Renders a triangle with per-vertex colors (Gouraud shading).
    /// Requires 6 words: (color1, vertex1, color2, vertex2, color3, vertex3)
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x30RRGGBB - Command (0x30) + Color1 (RGB)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: 0x00RRGGBB - Color2 (RGB)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// Word 4: 0x00RRGGBB - Color3 (RGB)
    /// Word 5: YYYYXXXX - Vertex3 (X, Y)
    /// ```
    ///
    /// # References
    ///
    /// - [PSX-SPX: GPU Polygon Commands](http://problemkaputt.de/psx-spx.htm#gpurenderpolygoncommands)
    pub(in crate::core::gpu) fn parse_shaded_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let c2v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let colors = [
            Color::from_u32(c0v0),
            Color::from_u32(c1v1),
            Color::from_u32(c2v2),
        ];
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_gradient_triangle(&vertices, &colors, false);
    }

    /// GP0(0x32): Gouraud-Shaded Triangle (Semi-Transparent)
    ///
    /// Renders a triangle with per-vertex colors and semi-transparency enabled.
    /// Requires 6 words: (color1, vertex1, color2, vertex2, color3, vertex3)
    pub(in crate::core::gpu) fn parse_shaded_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let c2v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();

        let colors = [
            Color::from_u32(c0v0),
            Color::from_u32(c1v1),
            Color::from_u32(c2v2),
        ];
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];

        self.render_gradient_triangle(&vertices, &colors, true);
    }

    /// GP0(0x38): Gouraud-Shaded Quad (Opaque)
    ///
    /// Renders a quadrilateral with per-vertex colors (Gouraud shading).
    /// Requires 8 words: (color1, vertex1, color2, vertex2, color3, vertex3, color4, vertex4)
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x38RRGGBB - Command (0x38) + Color1 (RGB)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: 0x00RRGGBB - Color2 (RGB)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// Word 4: 0x00RRGGBB - Color3 (RGB)
    /// Word 5: YYYYXXXX - Vertex3 (X, Y)
    /// Word 6: 0x00RRGGBB - Color4 (RGB)
    /// Word 7: YYYYXXXX - Vertex4 (X, Y)
    /// ```
    pub(in crate::core::gpu) fn parse_shaded_quad_opaque(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let c2v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let c3v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let colors = [
            Color::from_u32(c0v0),
            Color::from_u32(c1v1),
            Color::from_u32(c2v2),
            Color::from_u32(c3v3),
        ];
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_gradient_quad(&vertices, &colors, false);
    }

    /// GP0(0x3A): Gouraud-Shaded Quad (Semi-Transparent)
    ///
    /// Renders a quadrilateral with per-vertex colors and semi-transparency enabled.
    /// Requires 8 words: (color1, vertex1, color2, vertex2, color3, vertex3, color4, vertex4)
    pub(in crate::core::gpu) fn parse_shaded_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let c2v2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let c3v3 = self.command_fifo.pop_front().unwrap();
        let v4 = self.command_fifo.pop_front().unwrap();

        let colors = [
            Color::from_u32(c0v0),
            Color::from_u32(c1v1),
            Color::from_u32(c2v2),
            Color::from_u32(c3v3),
        ];
        let vertices = [
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
            Vertex::from_u32(v4),
        ];

        self.render_gradient_quad(&vertices, &colors, true);
    }

    /// GP0(0x24): Textured Triangle (Opaque)
    ///
    /// Renders a textured triangle with texture mapping.
    /// Requires 7 words: command+color, vertex1+texcoord1, clut, vertex2+texcoord2, tpage, vertex3+texcoord3
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x24RRGGBB - Command (0x24) + Color (RGB tint)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: CLUTVVUU - CLUT info (bits 16-31) + TexCoord1 (U, V)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// Word 4: PAGEVVUU - Texture Page (bits 16-31) + TexCoord2 (U, V)
    /// Word 5: YYYYXXXX - Vertex3 (X, Y)
    /// Word 6: ----VVUU - TexCoord3 (U, V)
    /// ```
    ///
    /// # CLUT and Texture Page Encoding
    ///
    /// Word 2 (CLUT):
    /// - Bits 16-21: CLUT X coordinate / 16 (multiply by 16 to get actual X)
    /// - Bits 22-30: CLUT Y coordinate
    ///
    /// Word 4 (Texture Page):
    /// - Bits 16-19: Texture page X base (N×64)
    /// - Bit 20: Texture page Y base (0=Y0-255, 1=Y256-511)
    /// - Bits 21-22: Semi-transparency mode (ignored for opaque)
    /// - Bits 23-24: Texture depth (0=4bit, 1=8bit, 2=15bit)
    ///
    /// # References
    ///
    /// - [PSX-SPX: GPU Texture Commands](http://problemkaputt.de/psx-spx.htm#gputextureattributes)
    pub(in crate::core::gpu) fn parse_textured_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 7 {
            return; // Need more words
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v0 = self.command_fifo.pop_front().unwrap();
        let t0clut = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let t1page = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let t2 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v0),
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
        ];
        let texcoords = [
            TexCoord::from_u32(t0clut),
            TexCoord::from_u32(t1page),
            TexCoord::from_u32(t2),
        ];

        // Extract CLUT coordinates from word 2
        let clut_x = ((t0clut >> 16) & 0x3F) * 16;
        let clut_y = (t0clut >> 22) & 0x1FF;

        // Extract texture page information from word 4
        let page_x = ((t1page >> 16) & 0xF) * 64;
        let page_y = ((t1page >> 20) & 1) * 256;
        let tex_depth = ((t1page >> 23) & 0x3) as u8;

        let texture_info = TextureInfo {
            page_x: page_x as u16,
            page_y: page_y as u16,
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            depth: tex_depth.into(),
        };

        self.render_textured_triangle(&vertices, &texcoords, &texture_info, &color, false);
    }

    /// GP0(0x26): Textured Triangle (Semi-Transparent)
    ///
    /// Renders a textured triangle with semi-transparency enabled.
    /// Same format as 0x24, but with semi-transparency blending applied.
    pub(in crate::core::gpu) fn parse_textured_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 7 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v0 = self.command_fifo.pop_front().unwrap();
        let t0clut = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let t1page = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let t2 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v0),
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
        ];
        let texcoords = [
            TexCoord::from_u32(t0clut),
            TexCoord::from_u32(t1page),
            TexCoord::from_u32(t2),
        ];

        let clut_x = ((t0clut >> 16) & 0x3F) * 16;
        let clut_y = (t0clut >> 22) & 0x1FF;
        let page_x = ((t1page >> 16) & 0xF) * 64;
        let page_y = ((t1page >> 20) & 1) * 256;
        let tex_depth = ((t1page >> 23) & 0x3) as u8;

        let texture_info = TextureInfo {
            page_x: page_x as u16,
            page_y: page_y as u16,
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            depth: tex_depth.into(),
        };

        self.render_textured_triangle(&vertices, &texcoords, &texture_info, &color, true);
    }

    /// GP0(0x2C): Textured Quadrilateral (Opaque)
    ///
    /// Renders a textured quadrilateral with texture mapping.
    /// Requires 9 words: command+color, 4×(vertex+texcoord), with CLUT and texture page info
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x2CRRGGBB - Command (0x2C) + Color (RGB tint)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: CLUTVVUU - CLUT info + TexCoord1 (U, V)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// Word 4: PAGEVVUU - Texture Page + TexCoord2 (U, V)
    /// Word 5: YYYYXXXX - Vertex3 (X, Y)
    /// Word 6: ----VVUU - TexCoord3 (U, V)
    /// Word 7: YYYYXXXX - Vertex4 (X, Y)
    /// Word 8: ----VVUU - TexCoord4 (U, V)
    /// ```
    pub(in crate::core::gpu) fn parse_textured_quad_opaque(&mut self) {
        if self.command_fifo.len() < 9 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v0 = self.command_fifo.pop_front().unwrap();
        let t0clut = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let t1page = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let t2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let t3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v0),
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];
        let texcoords = [
            TexCoord::from_u32(t0clut),
            TexCoord::from_u32(t1page),
            TexCoord::from_u32(t2),
            TexCoord::from_u32(t3),
        ];

        let clut_x = ((t0clut >> 16) & 0x3F) * 16;
        let clut_y = (t0clut >> 22) & 0x1FF;
        let page_x = ((t1page >> 16) & 0xF) * 64;
        let page_y = ((t1page >> 20) & 1) * 256;
        let tex_depth = ((t1page >> 23) & 0x3) as u8;

        let texture_info = TextureInfo {
            page_x: page_x as u16,
            page_y: page_y as u16,
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            depth: tex_depth.into(),
        };

        self.render_textured_quad(&vertices, &texcoords, &texture_info, &color, false);
    }

    /// GP0(0x2E): Textured Quadrilateral (Semi-Transparent)
    ///
    /// Renders a textured quadrilateral with semi-transparency enabled.
    /// Same format as 0x2C, but with semi-transparency blending applied.
    pub(in crate::core::gpu) fn parse_textured_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 9 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v0 = self.command_fifo.pop_front().unwrap();
        let t0clut = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let t1page = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();
        let t2 = self.command_fifo.pop_front().unwrap();
        let v3 = self.command_fifo.pop_front().unwrap();
        let t3 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertices = [
            Vertex::from_u32(v0),
            Vertex::from_u32(v1),
            Vertex::from_u32(v2),
            Vertex::from_u32(v3),
        ];
        let texcoords = [
            TexCoord::from_u32(t0clut),
            TexCoord::from_u32(t1page),
            TexCoord::from_u32(t2),
            TexCoord::from_u32(t3),
        ];

        let clut_x = ((t0clut >> 16) & 0x3F) * 16;
        let clut_y = (t0clut >> 22) & 0x1FF;
        let page_x = ((t1page >> 16) & 0xF) * 64;
        let page_y = ((t1page >> 20) & 1) * 256;
        let tex_depth = ((t1page >> 23) & 0x3) as u8;

        let texture_info = TextureInfo {
            page_x: page_x as u16,
            page_y: page_y as u16,
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            depth: tex_depth.into(),
        };

        self.render_textured_quad(&vertices, &texcoords, &texture_info, &color, true);
    }
}
