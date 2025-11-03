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

//! GP0 rectangle drawing commands
//!
//! Implements parsing and rendering for rectangle primitives:
//! - Monochrome rectangles (solid color)
//! - Textured rectangles (sprite rendering)
//! - Variable size and fixed size (1×1, 8×8, 16×16)

use super::super::types::{Color, TexCoord, TextureInfo, Vertex};
use super::super::GPU;

impl GPU {
    // =========================================================================
    // Monochrome (Solid Color) Rectangles
    // =========================================================================

    /// GP0(0x60): Monochrome Rectangle (Variable Size, Opaque)
    ///
    /// Renders a solid-color rectangle of variable dimensions.
    /// Requires 3 words: command+color, vertex, width+height
    pub(in crate::core::gpu) fn parse_monochrome_rect_variable_opaque(&mut self) {
        if self.command_fifo.len() < 3 {
            return; // Need more words
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        self.render_monochrome_rect(pos.x, pos.y, width, height, &color, false);
    }

    /// GP0(0x62): Monochrome Rectangle (Variable Size, Semi-Transparent)
    ///
    /// Renders a solid-color rectangle with semi-transparency enabled.
    /// Requires 3 words: command+color, vertex, width+height
    pub(in crate::core::gpu) fn parse_monochrome_rect_variable_semi_transparent(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        self.render_monochrome_rect(pos.x, pos.y, width, height, &color, true);
    }

    /// GP0(0x68): Monochrome Rectangle (1×1, Opaque)
    ///
    /// Renders a single pixel in solid color.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_1x1_opaque(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 1, 1, &color, false);
    }

    /// GP0(0x6A): Monochrome Rectangle (1×1, Semi-Transparent)
    ///
    /// Renders a single pixel with semi-transparency.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_1x1_semi_transparent(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 1, 1, &color, true);
    }

    /// GP0(0x70): Monochrome Rectangle (8×8, Opaque)
    ///
    /// Renders an 8×8 pixel rectangle in solid color.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_8x8_opaque(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 8, 8, &color, false);
    }

    /// GP0(0x72): Monochrome Rectangle (8×8, Semi-Transparent)
    ///
    /// Renders an 8×8 pixel rectangle with semi-transparency.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_8x8_semi_transparent(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 8, 8, &color, true);
    }

    /// GP0(0x78): Monochrome Rectangle (16×16, Opaque)
    ///
    /// Renders a 16×16 pixel rectangle in solid color.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_16x16_opaque(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 16, 16, &color, false);
    }

    /// GP0(0x7A): Monochrome Rectangle (16×16, Semi-Transparent)
    ///
    /// Renders a 16×16 pixel rectangle with semi-transparency.
    /// Requires 2 words: command+color, vertex
    pub(in crate::core::gpu) fn parse_monochrome_rect_16x16_semi_transparent(&mut self) {
        if self.command_fifo.len() < 2 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);

        self.render_monochrome_rect(pos.x, pos.y, 16, 16, &color, true);
    }

    // =========================================================================
    // Textured Rectangles
    // =========================================================================

    /// GP0(0x64): Textured Rectangle (Variable Size, Opaque, Raw Texture)
    ///
    /// Renders a textured rectangle with raw texture colors (no modulation).
    /// Requires 4 words: command+color, vertex, texcoord+clut, width+height
    pub(in crate::core::gpu) fn parse_textured_rect_variable_opaque(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        log::info!(
            "GP0(0x64) Textured Rect: pos=({}, {}), size={}x{}, texcoord=({}, {}), clut=({}, {})",
            pos.x, pos.y, width, height, texcoord.u, texcoord.v, clut_x, clut_y
        );

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            width,
            height,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            false,
        );
    }

    /// GP0(0x65): Textured Rectangle (Variable Size, Opaque, Modulated)
    ///
    /// Renders a textured rectangle with color modulation.
    /// Requires 4 words: command+color, vertex, texcoord+clut, width+height
    pub(in crate::core::gpu) fn parse_textured_rect_variable_opaque_modulated(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            width,
            height,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            true,
        );
    }

    /// GP0(0x66): Textured Rectangle (Variable Size, Semi-Transparent, Raw Texture)
    ///
    /// Renders a textured rectangle with semi-transparency, no modulation.
    /// Requires 4 words: command+color, vertex, texcoord+clut, width+height
    pub(in crate::core::gpu) fn parse_textured_rect_variable_semi_transparent(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            width,
            height,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            false,
        );
    }

    /// GP0(0x67): Textured Rectangle (Variable Size, Semi-Transparent, Modulated)
    ///
    /// Renders a textured rectangle with semi-transparency and color modulation.
    /// Requires 4 words: command+color, vertex, texcoord+clut, width+height
    pub(in crate::core::gpu) fn parse_textured_rect_variable_semi_transparent_modulated(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();
        let size = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;
        let width = (size >> 16) as u16;
        let height = (size & 0xFFFF) as u16;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            width,
            height,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            true,
        );
    }

    /// GP0(0x6C): Textured Rectangle (1×1, Opaque, Raw Texture)
    ///
    /// Renders a 1×1 textured rectangle (single texel).
    /// Requires 3 words: command+color, vertex, texcoord+clut
    pub(in crate::core::gpu) fn parse_textured_rect_1x1_opaque(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            1,
            1,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            false,
        );
    }

    /// GP0(0x6D): Textured Rectangle (1×1, Opaque, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_1x1_opaque_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            1,
            1,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            true,
        );
    }

    /// GP0(0x6E): Textured Rectangle (1×1, Semi-Transparent, Raw Texture)
    pub(in crate::core::gpu) fn parse_textured_rect_1x1_semi_transparent(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            1,
            1,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            false,
        );
    }

    /// GP0(0x6F): Textured Rectangle (1×1, Semi-Transparent, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_1x1_semi_transparent_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            1,
            1,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            true,
        );
    }

    /// GP0(0x74): Textured Rectangle (8×8, Opaque, Raw Texture)
    pub(in crate::core::gpu) fn parse_textured_rect_8x8_opaque(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            8,
            8,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            false,
        );
    }

    /// GP0(0x75): Textured Rectangle (8×8, Opaque, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_8x8_opaque_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            8,
            8,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            true,
        );
    }

    /// GP0(0x76): Textured Rectangle (8×8, Semi-Transparent, Raw Texture)
    pub(in crate::core::gpu) fn parse_textured_rect_8x8_semi_transparent(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            8,
            8,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            false,
        );
    }

    /// GP0(0x77): Textured Rectangle (8×8, Semi-Transparent, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_8x8_semi_transparent_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            8,
            8,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            true,
        );
    }

    /// GP0(0x7C): Textured Rectangle (16×16, Opaque, Raw Texture)
    pub(in crate::core::gpu) fn parse_textured_rect_16x16_opaque(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            16,
            16,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            false,
        );
    }

    /// GP0(0x7D): Textured Rectangle (16×16, Opaque, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_16x16_opaque_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            16,
            16,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            false,
            true,
        );
    }

    /// GP0(0x7E): Textured Rectangle (16×16, Semi-Transparent, Raw Texture)
    pub(in crate::core::gpu) fn parse_textured_rect_16x16_semi_transparent(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            16,
            16,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            false,
        );
    }

    /// GP0(0x7F): Textured Rectangle (16×16, Semi-Transparent, Modulated)
    pub(in crate::core::gpu) fn parse_textured_rect_16x16_semi_transparent_modulated(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let vertex = self.command_fifo.pop_front().unwrap();
        let texcoord_clut = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let pos = Vertex::from_u32(vertex);
        let texcoord = TexCoord::from_u32(texcoord_clut);
        let clut_x = ((texcoord_clut >> 16) & 0x3F) * 16;
        let clut_y = (texcoord_clut >> 22) & 0x1FF;

        let texture_info = TextureInfo {
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            page_x: self.draw_mode.texture_page_x_base,
            page_y: self.draw_mode.texture_page_y_base,
            depth: self.draw_mode.texture_depth.into(),
        };

        self.render_textured_rect(
            pos.x,
            pos.y,
            16,
            16,
            texcoord.u,
            texcoord.v,
            &texture_info,
            &color,
            true,
            true,
        );
    }

    // =========================================================================
    // Rendering Functions
    // =========================================================================

    /// Render a monochrome (solid color) rectangle
    ///
    /// # Arguments
    ///
    /// * `x` - Top-left X coordinate
    /// * `y` - Top-left Y coordinate
    /// * `width` - Rectangle width in pixels
    /// * `height` - Rectangle height in pixels
    /// * `color` - Fill color
    /// * `semi_transparent` - Enable semi-transparency blending
    fn render_monochrome_rect(
        &mut self,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        color: &Color,
        semi_transparent: bool,
    ) {
        self.rasterizer.draw_rectangle(
            &mut self.vram,
            &self.draw_mode,
            &self.draw_area,
            self.draw_offset,
            x,
            y,
            width,
            height,
            color,
            semi_transparent,
        );
    }

    /// Render a textured rectangle
    ///
    /// # Arguments
    ///
    /// * `x` - Top-left X coordinate
    /// * `y` - Top-left Y coordinate
    /// * `width` - Rectangle width in pixels
    /// * `height` - Rectangle height in pixels
    /// * `tex_u` - Texture U coordinate (top-left)
    /// * `tex_v` - Texture V coordinate (top-left)
    /// * `texture_info` - Texture page and CLUT information
    /// * `color` - Modulation color (if modulated is true)
    /// * `semi_transparent` - Enable semi-transparency blending
    /// * `modulated` - Enable color modulation (multiply texture by color)
    #[allow(clippy::too_many_arguments)]
    fn render_textured_rect(
        &mut self,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        tex_u: u8,
        tex_v: u8,
        texture_info: &TextureInfo,
        color: &Color,
        semi_transparent: bool,
        modulated: bool,
    ) {
        self.rasterizer.draw_textured_rectangle(
            &mut self.vram,
            &self.draw_mode,
            &self.draw_area,
            self.draw_offset,
            x,
            y,
            width,
            height,
            tex_u,
            tex_v,
            texture_info,
            color,
            semi_transparent,
            modulated,
        );
    }
}
