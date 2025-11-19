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

//! GP1 display configuration commands
//!
//! Implements display settings including resolution, area, and video mode.

use super::super::registers::{ColorDepth, HorizontalRes, VerticalRes, VideoMode};
use super::super::GPU;

impl GPU {
    /// GP1(0x03): Display Enable
    ///
    /// Enables or disables the display output.
    ///
    /// # Arguments
    ///
    /// * `value` - Bit 0: 0=Enable, 1=Disable (inverted logic)
    pub(crate) fn gp1_display_enable(&mut self, value: u32) {
        let enabled = (value & 1) == 0;
        self.display_mode.display_disabled = !enabled;
        self.status.display_disabled = !enabled;

        log::debug!("Display {}", if enabled { "enabled" } else { "disabled" });
    }

    /// GP1(0x05): Start of Display Area
    ///
    /// Sets the top-left corner of the display area in VRAM.
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-9: X coordinate, Bits 10-18: Y coordinate
    pub(crate) fn gp1_display_area_start(&mut self, value: u32) {
        let x = (value & 0x3FF) as u16;
        let y = ((value >> 10) & 0x1FF) as u16;

        self.display_area.x = x;
        self.display_area.y = y;

        log::debug!("Display area start: ({}, {})", x, y);
    }

    /// GP1(0x06): Horizontal Display Range
    ///
    /// Sets the horizontal display range on screen (scanline timing).
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-11: X1 start, Bits 12-23: X2 end
    pub(crate) fn gp1_horizontal_display_range(&mut self, value: u32) {
        let x1 = (value & 0xFFF) as u16;
        let x2 = ((value >> 12) & 0xFFF) as u16;

        // Store as width
        self.display_area.width = x2.saturating_sub(x1);

        log::debug!(
            "Horizontal display range: {} to {} (width: {})",
            x1,
            x2,
            self.display_area.width
        );
    }

    /// GP1(0x07): Vertical Display Range
    ///
    /// Sets the vertical display range on screen (scanline timing).
    ///
    /// # Arguments
    ///
    /// * `value` - Bits 0-9: Y1 start, Bits 10-19: Y2 end
    pub(crate) fn gp1_vertical_display_range(&mut self, value: u32) {
        let y1 = (value & 0x3FF) as u16;
        let y2 = ((value >> 10) & 0x3FF) as u16;

        // Store as height
        self.display_area.height = y2.saturating_sub(y1);

        log::debug!(
            "Vertical display range: {} to {} (height: {})",
            y1,
            y2,
            self.display_area.height
        );
    }

    /// GP1(0x08): Display Mode
    ///
    /// Sets the display mode including resolution, video mode, and color depth.
    ///
    /// # Arguments
    ///
    /// * `value` - Display mode configuration bits:
    ///   - Bits 0-1: Horizontal resolution 1
    ///   - Bit 2: Vertical resolution (0=240, 1=480)
    ///   - Bit 3: Video mode (0=NTSC, 1=PAL)
    ///   - Bit 4: Color depth (0=15bit, 1=24bit)
    ///   - Bit 5: Interlace (0=Off, 1=On)
    ///   - Bit 6: Horizontal resolution 2
    ///   - Bit 7: Reverse flag
    pub(crate) fn gp1_display_mode(&mut self, value: u32) {
        // Horizontal resolution
        let hr1 = (value & 3) as u8;
        let hr2 = ((value >> 6) & 1) as u8;
        self.display_mode.horizontal_res = match (hr2, hr1) {
            (0, 0) => HorizontalRes::R256,
            (0, 1) => HorizontalRes::R320,
            (0, 2) => HorizontalRes::R512,
            (0, 3) => HorizontalRes::R640,
            (1, 0) => HorizontalRes::R368,
            (1, 1) => HorizontalRes::R384,
            (1, _) => HorizontalRes::R368, // Reserved combinations default to 368
            _ => HorizontalRes::R320,
        };

        // Update status register horizontal resolution bits
        self.status.horizontal_res_1 = hr1;
        self.status.horizontal_res_2 = hr2;

        // Vertical resolution
        let vres = ((value >> 2) & 1) != 0;
        self.display_mode.vertical_res = if vres {
            VerticalRes::R480
        } else {
            VerticalRes::R240
        };
        self.status.vertical_res = vres;

        // Video mode (NTSC/PAL)
        let video_mode = ((value >> 3) & 1) != 0;
        self.display_mode.video_mode = if video_mode {
            VideoMode::PAL
        } else {
            VideoMode::NTSC
        };
        self.status.video_mode = video_mode;

        // Color depth
        let color_depth = ((value >> 4) & 1) != 0;
        self.display_mode.display_area_color_depth = if color_depth {
            ColorDepth::C24Bit
        } else {
            ColorDepth::C15Bit
        };
        self.status.display_area_color_depth = color_depth;

        // Interlace
        let interlaced = ((value >> 5) & 1) != 0;
        self.display_mode.interlaced = interlaced;
        self.status.vertical_interlace = interlaced;

        // Reverse flag (rarely used)
        self.status.reverse_flag = ((value >> 7) & 1) != 0;

        log::debug!(
            "Display mode: {:?} {:?} {:?} {:?} interlaced={}",
            self.display_mode.horizontal_res,
            self.display_mode.vertical_res,
            self.display_mode.video_mode,
            self.display_mode.display_area_color_depth,
            interlaced
        );
    }
}
