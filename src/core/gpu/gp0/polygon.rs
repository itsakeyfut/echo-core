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
//! Implements parsing for triangle and quadrilateral rendering commands.

use super::super::types::{Color, Vertex};
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

    /// GP0(0x30): Gouraud-Shaded Triangle (Opaque) - Placeholder
    ///
    /// Will be implemented in issue #33
    pub(in crate::core::gpu) fn parse_shaded_triangle_opaque(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..6 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded triangle rendering not yet implemented (issue #33)");
    }

    /// GP0(0x32): Gouraud-Shaded Triangle (Semi-Transparent) - Placeholder
    ///
    /// Will be implemented in issue #33
    pub(in crate::core::gpu) fn parse_shaded_triangle_semi_transparent(&mut self) {
        if self.command_fifo.len() < 6 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..6 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded triangle rendering not yet implemented (issue #33)");
    }

    /// GP0(0x38): Gouraud-Shaded Quad (Opaque) - Placeholder
    ///
    /// Will be implemented in issue #33
    pub(in crate::core::gpu) fn parse_shaded_quad_opaque(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..8 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded quad rendering not yet implemented (issue #33)");
    }

    /// GP0(0x3A): Gouraud-Shaded Quad (Semi-Transparent) - Placeholder
    ///
    /// Will be implemented in issue #33
    pub(in crate::core::gpu) fn parse_shaded_quad_semi_transparent(&mut self) {
        if self.command_fifo.len() < 8 {
            return;
        }

        // Consume command words to prevent stalling
        for _ in 0..8 {
            self.command_fifo.pop_front();
        }

        log::warn!("Shaded quad rendering not yet implemented (issue #33)");
    }
}
