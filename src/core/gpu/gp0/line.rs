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

//! GP0 line drawing commands
//!
//! Implements parsing for line and polyline rendering commands.

use super::super::types::{Color, Vertex};
use super::super::GPU;

impl GPU {
    /// GP0(0x40): Monochrome Line (Opaque)
    ///
    /// Renders a single line segment with a solid color.
    /// Requires 3 words: command+color, vertex1, vertex2
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x40RRGGBB - Command (0x40) + RGB color
    /// Word 1: YYYYXXXX - Start vertex (X, Y)
    /// Word 2: YYYYXXXX - End vertex (X, Y)
    /// ```
    ///
    /// # References
    ///
    /// - [PSX-SPX: GPU Line Commands](http://problemkaputt.de/psx-spx.htm#gpurenderlinecommands)
    pub(in crate::core::gpu) fn parse_line_opaque(&mut self) {
        if self.command_fifo.len() < 3 {
            return; // Need more words
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertex1 = Vertex::from_u32(v1);
        let vertex2 = Vertex::from_u32(v2);

        self.render_line(vertex1, vertex2, color, false);
    }

    /// GP0(0x42): Monochrome Line (Semi-Transparent)
    ///
    /// Renders a single line segment with semi-transparency enabled.
    /// Requires 3 words: command+color, vertex1, vertex2
    pub(in crate::core::gpu) fn parse_line_semi_transparent(&mut self) {
        if self.command_fifo.len() < 3 {
            return;
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();

        let color = Color::from_u32(cmd);
        let vertex1 = Vertex::from_u32(v1);
        let vertex2 = Vertex::from_u32(v2);

        self.render_line(vertex1, vertex2, color, true);
    }

    /// GP0(0x48): Monochrome Polyline (Opaque)
    ///
    /// Renders connected line segments (polyline) with a solid color.
    /// The polyline is terminated by a vertex with coordinate 0x50005000
    /// or 0x55555555.
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x48RRGGBB - Command (0x48) + RGB color
    /// Word 1: YYYYXXXX - First vertex (X, Y)
    /// Word 2: YYYYXXXX - Second vertex (X, Y)
    /// ...
    /// Word N: 0x50005000 or 0x55555555 - Terminator
    /// ```
    ///
    /// # Notes
    ///
    /// The terminator value signals the end of the polyline.
    /// We wait for the terminator before processing the polyline.
    pub(in crate::core::gpu) fn parse_polyline_opaque(&mut self) {
        if self.command_fifo.len() < 4 {
            return; // Need at least 4 words (command + 2 vertices + terminator)
        }

        // Check if there's a terminator in the FIFO
        let has_terminator = self
            .command_fifo
            .iter()
            .skip(1) // Skip command word
            .any(|&word| word == 0x5000_5000 || word == 0x5555_5555);

        if !has_terminator {
            return; // Wait for terminator
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let color = Color::from_u32(cmd);

        // Collect vertices until terminator
        let mut vertices = Vec::new();
        while let Some(&word) = self.command_fifo.front() {
            // Check for terminator values
            if word == 0x5000_5000 || word == 0x5555_5555 {
                self.command_fifo.pop_front();
                break;
            }

            let v = self.command_fifo.pop_front().unwrap();
            vertices.push(Vertex::from_u32(v));

            // Safety limit to prevent infinite loops
            if vertices.len() >= 256 {
                log::warn!("Polyline exceeded 256 vertices, discarding remainder");
                // Drain remaining vertices and terminator to avoid FIFO desync
                while let Some(word) = self.command_fifo.pop_front() {
                    if word == 0x5000_5000 || word == 0x5555_5555 {
                        break;
                    }
                }
                break;
            }
        }

        // Need at least 2 vertices to draw
        if vertices.len() >= 2 {
            self.render_polyline(&vertices, color, false);
        }
    }

    /// GP0(0x4A): Monochrome Polyline (Semi-Transparent)
    ///
    /// Renders connected line segments with semi-transparency enabled.
    /// Format is identical to 0x48 but with semi-transparency.
    pub(in crate::core::gpu) fn parse_polyline_semi_transparent(&mut self) {
        if self.command_fifo.len() < 4 {
            return; // Need at least 4 words (command + 2 vertices + terminator)
        }

        // Check if there's a terminator in the FIFO
        let has_terminator = self
            .command_fifo
            .iter()
            .skip(1) // Skip command word
            .any(|&word| word == 0x5000_5000 || word == 0x5555_5555);

        if !has_terminator {
            return; // Wait for terminator
        }

        let cmd = self.command_fifo.pop_front().unwrap();
        let color = Color::from_u32(cmd);

        // Collect vertices until terminator
        let mut vertices = Vec::new();
        while let Some(&word) = self.command_fifo.front() {
            // Check for terminator values
            if word == 0x5000_5000 || word == 0x5555_5555 {
                self.command_fifo.pop_front();
                break;
            }

            let v = self.command_fifo.pop_front().unwrap();
            vertices.push(Vertex::from_u32(v));

            // Safety limit
            if vertices.len() >= 256 {
                log::warn!("Polyline exceeded 256 vertices, discarding remainder");
                // Drain remaining vertices and terminator to avoid FIFO desync
                while let Some(word) = self.command_fifo.pop_front() {
                    if word == 0x5000_5000 || word == 0x5555_5555 {
                        break;
                    }
                }
                break;
            }
        }

        if vertices.len() >= 2 {
            self.render_polyline(&vertices, color, true);
        }
    }

    /// GP0(0x50): Shaded Line (Opaque)
    ///
    /// Renders a single line segment with Gouraud shading (gradient color interpolation).
    /// Requires 4 words: command+color1, vertex1, color2, vertex2
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x50RRGGBB - Command (0x50) + Color1 (RGB)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: 0x00RRGGBB - Color2 (RGB)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// ```
    ///
    /// # References
    ///
    /// - [PSX-SPX: GPU Line Commands](http://problemkaputt.de/psx-spx.htm#gpurenderlinecommands)
    pub(in crate::core::gpu) fn parse_shaded_line_opaque(&mut self) {
        if self.command_fifo.len() < 4 {
            return; // Need more words
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();

        let color1 = Color::from_u32(c0v0);
        let vertex1 = Vertex::from_u32(v1);
        let color2 = Color::from_u32(c1v1);
        let vertex2 = Vertex::from_u32(v2);

        self.render_shaded_line(vertex1, color1, vertex2, color2, false);
    }

    /// GP0(0x52): Shaded Line (Semi-Transparent)
    ///
    /// Renders a single line segment with Gouraud shading and semi-transparency enabled.
    /// Requires 4 words: command+color1, vertex1, color2, vertex2
    pub(in crate::core::gpu) fn parse_shaded_line_semi_transparent(&mut self) {
        if self.command_fifo.len() < 4 {
            return;
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let v1 = self.command_fifo.pop_front().unwrap();
        let c1v1 = self.command_fifo.pop_front().unwrap();
        let v2 = self.command_fifo.pop_front().unwrap();

        let color1 = Color::from_u32(c0v0);
        let vertex1 = Vertex::from_u32(v1);
        let color2 = Color::from_u32(c1v1);
        let vertex2 = Vertex::from_u32(v2);

        self.render_shaded_line(vertex1, color1, vertex2, color2, true);
    }

    /// GP0(0x58): Shaded Polyline (Opaque)
    ///
    /// Renders connected line segments with Gouraud shading (per-vertex colors).
    /// The polyline is terminated by 0x50005000 or 0x55555555.
    ///
    /// # Command Format
    ///
    /// ```text
    /// Word 0: 0x58RRGGBB - Command (0x58) + Color1 (RGB)
    /// Word 1: YYYYXXXX - Vertex1 (X, Y)
    /// Word 2: 0x00RRGGBB - Color2 (RGB)
    /// Word 3: YYYYXXXX - Vertex2 (X, Y)
    /// ...
    /// Word N: 0x50005000 or 0x55555555 - Terminator
    /// ```
    ///
    /// # Notes
    ///
    /// The terminator value signals the end of the polyline.
    /// We wait for the terminator before processing the polyline.
    pub(in crate::core::gpu) fn parse_shaded_polyline_opaque(&mut self) {
        if self.command_fifo.len() < 5 {
            return; // Need at least 5 words (command+color1 + vertex1 + color2 + vertex2 + terminator)
        }

        // Check if there's a terminator in the FIFO
        let has_terminator = self
            .command_fifo
            .iter()
            .skip(1) // Skip command word
            .any(|&word| word == 0x5000_5000 || word == 0x5555_5555);

        if !has_terminator {
            return; // Wait for terminator
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let first_color = Color::from_u32(c0v0);
        let first_vertex = Vertex::from_u32(self.command_fifo.pop_front().unwrap());

        // Collect color-vertex pairs until terminator
        let mut vertices = vec![first_vertex];
        let mut colors = vec![first_color];

        while let Some(&word) = self.command_fifo.front() {
            // Check for terminator values
            if word == 0x5000_5000 || word == 0x5555_5555 {
                self.command_fifo.pop_front();
                break;
            }

            // Read color+vertex pair
            let color_word = self.command_fifo.pop_front().unwrap();

            // Check if next word exists and is a terminator before committing the color
            if let Some(&vertex_word) = self.command_fifo.front() {
                if vertex_word == 0x5000_5000 || vertex_word == 0x5555_5555 {
                    // Terminator follows color - malformed command
                    // Don't add color without matching vertex to maintain colors.len() == vertices.len()
                    log::warn!(
                        "Shaded polyline has color without matching vertex before terminator"
                    );
                    break;
                }
                let vertex_word = self.command_fifo.pop_front().unwrap();
                colors.push(Color::from_u32(color_word));
                vertices.push(Vertex::from_u32(vertex_word));
            } else {
                // No more words after color - shouldn't happen as we checked for terminator
                log::warn!("Shaded polyline color without vertex at end of FIFO");
                break;
            }

            // Safety limit to prevent infinite loops
            if vertices.len() >= 256 {
                log::warn!("Shaded polyline exceeded 256 vertices, discarding remainder");
                // Drain remaining words and terminator to avoid FIFO desync
                while let Some(word) = self.command_fifo.pop_front() {
                    if word == 0x5000_5000 || word == 0x5555_5555 {
                        break;
                    }
                }
                break;
            }
        }

        // Need at least 2 vertices to draw
        if vertices.len() >= 2 && colors.len() >= 2 {
            self.render_shaded_polyline(&vertices, &colors, false);
        }
    }

    /// GP0(0x5A): Shaded Polyline (Semi-Transparent)
    ///
    /// Renders connected line segments with Gouraud shading and semi-transparency enabled.
    /// Format is identical to 0x58 but with semi-transparency.
    pub(in crate::core::gpu) fn parse_shaded_polyline_semi_transparent(&mut self) {
        if self.command_fifo.len() < 5 {
            return; // Need at least 5 words (command+color1 + vertex1 + color2 + vertex2 + terminator)
        }

        // Check if there's a terminator in the FIFO
        let has_terminator = self
            .command_fifo
            .iter()
            .skip(1) // Skip command word
            .any(|&word| word == 0x5000_5000 || word == 0x5555_5555);

        if !has_terminator {
            return; // Wait for terminator
        }

        let c0v0 = self.command_fifo.pop_front().unwrap();
        let first_color = Color::from_u32(c0v0);
        let first_vertex = Vertex::from_u32(self.command_fifo.pop_front().unwrap());

        // Collect color-vertex pairs until terminator
        let mut vertices = vec![first_vertex];
        let mut colors = vec![first_color];

        while let Some(&word) = self.command_fifo.front() {
            // Check for terminator values
            if word == 0x5000_5000 || word == 0x5555_5555 {
                self.command_fifo.pop_front();
                break;
            }

            // Read color+vertex pair
            let color_word = self.command_fifo.pop_front().unwrap();

            // Check if next word exists and is a terminator before committing the color
            if let Some(&vertex_word) = self.command_fifo.front() {
                if vertex_word == 0x5000_5000 || vertex_word == 0x5555_5555 {
                    // Terminator follows color - malformed command
                    // Don't add color without matching vertex to maintain colors.len() == vertices.len()
                    log::warn!(
                        "Shaded polyline has color without matching vertex before terminator"
                    );
                    break;
                }
                let vertex_word = self.command_fifo.pop_front().unwrap();
                colors.push(Color::from_u32(color_word));
                vertices.push(Vertex::from_u32(vertex_word));
            } else {
                // No more words after color - shouldn't happen as we checked for terminator
                log::warn!("Shaded polyline color without vertex at end of FIFO");
                break;
            }

            // Safety limit
            if vertices.len() >= 256 {
                log::warn!("Shaded polyline exceeded 256 vertices, discarding remainder");
                // Drain remaining words and terminator to avoid FIFO desync
                while let Some(word) = self.command_fifo.pop_front() {
                    if word == 0x5000_5000 || word == 0x5555_5555 {
                        break;
                    }
                }
                break;
            }
        }

        if vertices.len() >= 2 && colors.len() >= 2 {
            self.render_shaded_polyline(&vertices, &colors, true);
        }
    }
}
