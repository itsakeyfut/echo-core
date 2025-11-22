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

//! GP0 command tests
//! Tests for GP0 drawing commands, command buffering, and parsing

use super::super::*;

#[test]
fn test_gp0_command_buffering() {
    let mut gpu = GPU::new();

    // Send partial command (should buffer)
    gpu.write_gp0(0xA0000000);
    assert_eq!(gpu.command_fifo.len(), 1);
    assert!(gpu.vram_transfer.is_none());

    // Send second word
    gpu.write_gp0(0x00000000);
    assert_eq!(gpu.command_fifo.len(), 2);
    assert!(gpu.vram_transfer.is_none());

    // Send third word - command should execute
    gpu.write_gp0(0x00010001);
    assert_eq!(gpu.command_fifo.len(), 0);
    assert!(gpu.vram_transfer.is_some());
}

#[test]
fn test_gp0_unknown_command() {
    let mut gpu = GPU::new();

    // Send unknown command (should be ignored)
    gpu.write_gp0(0xFF000000);

    // FIFO should be empty (command removed)
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_monochrome_triangle_parsing() {
    let mut gpu = GPU::new();

    // Monochrome triangle command
    gpu.write_gp0(0x20FF0000); // Red triangle
    gpu.write_gp0(0x00640032); // V1: (50, 100)
    gpu.write_gp0(0x00C80096); // V2: (150, 200)
    gpu.write_gp0(0x00320064); // V3: (100, 50)

    // Command should be processed (no crash, FIFO empty)
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_monochrome_triangle_semi_transparent() {
    let mut gpu = GPU::new();

    // Semi-transparent triangle command
    gpu.write_gp0(0x2200FF00); // Green semi-transparent triangle
    gpu.write_gp0(0x00000000); // V1: (0, 0)
    gpu.write_gp0(0x00640000); // V2: (100, 0)
    gpu.write_gp0(0x00320064); // V3: (50, 100)

    // Command should be processed
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_monochrome_quad_parsing() {
    let mut gpu = GPU::new();

    gpu.write_gp0(0x2800FF00); // Green quad
    gpu.write_gp0(0x00000000); // V1: (0, 0)
    gpu.write_gp0(0x00640000); // V2: (0, 100)
    gpu.write_gp0(0x00640064); // V3: (100, 100)
    gpu.write_gp0(0x00000064); // V4: (100, 0)

    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_monochrome_quad_semi_transparent() {
    let mut gpu = GPU::new();

    gpu.write_gp0(0x2A0000FF); // Blue semi-transparent quad
    gpu.write_gp0(0x000A000A); // V1: (10, 10)
    gpu.write_gp0(0x0032000A); // V2: (50, 10)
    gpu.write_gp0(0x00320032); // V3: (50, 50)
    gpu.write_gp0(0x000A0032); // V4: (10, 50)

    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_partial_command_buffering() {
    let mut gpu = GPU::new();

    // Send only 2 words of a triangle command (needs 4)
    gpu.write_gp0(0x20FF0000); // Command + color
    gpu.write_gp0(0x00000000); // V1

    // Should be buffered, not processed
    assert_eq!(gpu.command_fifo.len(), 2);

    // Send remaining words
    gpu.write_gp0(0x00640000); // V2
    gpu.write_gp0(0x00320064); // V3

    // Now command should be processed
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_gp0_line_command() {
    let mut gpu = GPU::new();

    // GP0(0x40): Line command
    // Word 0: 0x40FFFFFF (white line)
    // Word 1: 0x000A000A (10, 10)
    // Word 2: 0x0032 0032 (50, 50)
    gpu.write_gp0(0x40FF_FFFF);
    gpu.write_gp0(0x000A_000A);
    gpu.write_gp0(0x0032_0032);

    // Check line was drawn
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);
    assert_ne!(gpu.read_vram(30, 30), 0);
}

#[test]
fn test_gp0_polyline_command() {
    let mut gpu = GPU::new();

    // GP0(0x48): Polyline command (opaque)
    gpu.write_gp0(0x48FF_0000); // Red polyline
    gpu.write_gp0(0x000A_000A); // Vertex (10, 10) - X=10, Y=10
    gpu.write_gp0(0x0032_000A); // Vertex (10, 50) - X=10, Y=50
    gpu.write_gp0(0x0032_0032); // Vertex (50, 50) - X=50, Y=50
    gpu.write_gp0(0x5000_5000); // Terminator

    // Check vertices
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(10, 50), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);
}

#[test]
fn test_gp0_shaded_triangle_command() {
    let mut gpu = GPU::new();

    // GP0(0x30): Shaded triangle (opaque)
    // Word 0: 0x30FF0000 (command + red color)
    // Word 1: 0x00640064 (vertex1: X=100, Y=100)
    // Word 2: 0x0000FF00 (green color)
    // Word 3: 0x006400C8 (vertex2: X=200, Y=100)
    // Word 4: 0x000000FF (blue color)
    // Word 5: 0x00C80096 (vertex3: X=150, Y=200)
    gpu.write_gp0(0x30FF_0000);
    gpu.write_gp0(0x0064_0064);
    gpu.write_gp0(0x0000_FF00);
    gpu.write_gp0(0x0064_00C8);
    gpu.write_gp0(0x0000_00FF);
    gpu.write_gp0(0x00C8_0096);

    // Check pixels are drawn
    assert_ne!(gpu.read_vram(100, 100), 0);
    assert_ne!(gpu.read_vram(200, 100), 0);
    assert_ne!(gpu.read_vram(150, 150), 0);
}

#[test]
fn test_gp0_shaded_quad_command() {
    let mut gpu = GPU::new();

    // GP0(0x38): Shaded quad (opaque)
    gpu.write_gp0(0x38FF_0000); // Command + red
    gpu.write_gp0(0x0064_0064); // (100, 100)
    gpu.write_gp0(0x0000_FF00); // Green
    gpu.write_gp0(0x00C8_0064); // (200, 100)
    gpu.write_gp0(0x0000_00FF); // Blue
    gpu.write_gp0(0x00C8_00C8); // (200, 200)
    gpu.write_gp0(0x00FF_FF00); // Yellow
    gpu.write_gp0(0x0064_00C8); // (100, 200)

    // Check corners
    assert_ne!(gpu.read_vram(100, 100), 0);
    assert_ne!(gpu.read_vram(200, 100), 0);
    assert_ne!(gpu.read_vram(200, 200), 0);
    assert_ne!(gpu.read_vram(100, 200), 0);

    // Check center
    assert_ne!(gpu.read_vram(150, 150), 0);
}

#[test]
fn test_gp0_shaded_line_command() {
    let mut gpu = GPU::new();

    // GP0(0x50): Shaded Line command (opaque)
    // Word 0: 0x50FF0000 (command + red color)
    // Word 1: 0x000A000A (vertex1: 10, 10)
    // Word 2: 0x000000FF (color2: blue)
    // Word 3: 0x00320032 (vertex2: 50, 50)
    gpu.write_gp0(0x50FF_0000);
    gpu.write_gp0(0x000A_000A);
    gpu.write_gp0(0x0000_00FF);
    gpu.write_gp0(0x0032_0032);

    // Check line was drawn
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);
    assert_ne!(gpu.read_vram(30, 30), 0);
}

#[test]
fn test_gp0_shaded_line_semi_transparent() {
    let mut gpu = GPU::new();

    // GP0(0x52): Shaded Line command (semi-transparent)
    gpu.write_gp0(0x52FF_FF00); // Yellow
    gpu.write_gp0(0x0014_0014); // (20, 20)
    gpu.write_gp0(0x0000_FFFF); // Cyan
    gpu.write_gp0(0x003C_003C); // (60, 60)

    // Check line was drawn
    assert_ne!(gpu.read_vram(20, 20), 0);
    assert_ne!(gpu.read_vram(60, 60), 0);
}

#[test]
fn test_gp0_shaded_polyline_command() {
    let mut gpu = GPU::new();

    // GP0(0x58): Shaded Polyline command (opaque)
    gpu.write_gp0(0x58FF_0000); // Red
    gpu.write_gp0(0x000A_000A); // Vertex1 (10, 10)
    gpu.write_gp0(0x0000_FF00); // Green
    gpu.write_gp0(0x0032_000A); // Vertex2 (10, 50)
    gpu.write_gp0(0x0000_00FF); // Blue
    gpu.write_gp0(0x0032_0032); // Vertex3 (50, 50)
    gpu.write_gp0(0x5555_5555); // Terminator

    // Check vertices
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(10, 50), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);
}

#[test]
fn test_gp0_shaded_polyline_terminator() {
    let mut gpu = GPU::new();

    // Test with alternative terminator 0x50005000
    gpu.write_gp0(0x5800_00FF); // Blue
    gpu.write_gp0(0x0014_0014); // (20, 20)
    gpu.write_gp0(0x00FF_0000); // Red
    gpu.write_gp0(0x0028_0014); // (20, 40)
    gpu.write_gp0(0x5000_5000); // Alternative terminator

    // Check vertices
    assert_ne!(gpu.read_vram(20, 20), 0);
    assert_ne!(gpu.read_vram(20, 40), 0);
}

#[test]
fn test_shaded_polyline_semi_transparent() {
    let mut gpu = GPU::new();

    // GP0(0x5A): Shaded Polyline (semi-transparent)
    gpu.write_gp0(0x5AFF_FF00); // Yellow
    gpu.write_gp0(0x000F_000F); // (15, 15)
    gpu.write_gp0(0x00FF_00FF); // Magenta
    gpu.write_gp0(0x0023_000F); // (15, 35)
    gpu.write_gp0(0x0000_FFFF); // Cyan
    gpu.write_gp0(0x0023_0023); // (35, 35)
    gpu.write_gp0(0x5555_5555); // Terminator

    // Check vertices
    assert_ne!(gpu.read_vram(15, 15), 0);
    assert_ne!(gpu.read_vram(15, 35), 0);
    assert_ne!(gpu.read_vram(35, 35), 0);
}

#[test]
fn test_shaded_line_parsing_incomplete_data() {
    let mut gpu = GPU::new();

    // Send incomplete shaded line command (only 2 words instead of 4)
    gpu.write_gp0(0x50FF_0000);
    gpu.write_gp0(0x000A_000A);

    // Should not crash, command should remain in FIFO waiting for more data
    assert!(!gpu.command_fifo.is_empty());
}

#[test]
fn test_shaded_polyline_parsing_incomplete_data() {
    let mut gpu = GPU::new();

    // Send shaded polyline without terminator
    gpu.write_gp0(0x58FF_0000);
    gpu.write_gp0(0x000A_000A);
    gpu.write_gp0(0x0000_FF00);
    gpu.write_gp0(0x0032_000A);

    // Should not process until terminator arrives
    // Command should remain in FIFO
    assert!(!gpu.command_fifo.is_empty());
}

#[test]
fn test_draw_mode_setting() {
    let mut gpu = GPU::new();

    // Test GP0(E1h) - Draw Mode Setting
    // Set texture page to (128, 256) with 4-bit color, semi-transparency mode 1
    // Page X = 2 (2*64 = 128), Page Y = 1 (1*256 = 256)
    // Semi-transparency = 1 (B+F), Texture depth = 0 (4-bit), Dithering = 1
    // Bit layout: page_x(0-3)=2, page_y(4)=1, semi(5-6)=1, depth(7-8)=0, dither(9)=1
    gpu.write_gp0(0xE1000232); // 0x2 | 0x10 | 0x20 | 0x200

    assert_eq!(gpu.draw_mode.texture_page_x_base, 128);
    assert_eq!(gpu.draw_mode.texture_page_y_base, 256);
    assert_eq!(gpu.draw_mode.semi_transparency, 1);
    assert_eq!(gpu.draw_mode.texture_depth, 0);
    assert!(gpu.draw_mode.dithering);

    // Verify GPUSTAT mirrors the draw mode settings
    assert_eq!(gpu.status.texture_page_x_base, 2); // Raw value, not multiplied by 64
    assert_eq!(gpu.status.texture_page_y_base, 1); // Raw value (0 or 1)
    assert_eq!(gpu.status.semi_transparency, 1);
    assert_eq!(gpu.status.texture_depth, 0);
    assert!(gpu.status.dithering);
    assert!(!gpu.status.draw_to_display);
    assert!(!gpu.status.texture_disable);
}

#[test]
fn test_draw_mode_texture_depth() {
    let mut gpu = GPU::new();

    // Test 8-bit texture depth
    gpu.write_gp0(0xE1000080); // depth = 1 (8-bit)
    assert_eq!(gpu.draw_mode.texture_depth, 1);

    // Test 15-bit texture depth
    gpu.write_gp0(0xE1000100); // depth = 2 (15-bit)
    assert_eq!(gpu.draw_mode.texture_depth, 2);
}

#[test]
fn test_texture_window() {
    let mut gpu = GPU::new();

    // Test GP0(E2h) - Texture Window Setting
    // Mask (8, 8), Offset (16, 16)
    // mask_x=8, mask_y=8, offset_x=16, offset_y=16
    gpu.write_gp0(0xE2000008 | (8 << 5) | (16 << 10) | (16 << 15));

    assert_eq!(gpu.texture_window.mask_x, 8);
    assert_eq!(gpu.texture_window.mask_y, 8);
    assert_eq!(gpu.texture_window.offset_x, 16);
    assert_eq!(gpu.texture_window.offset_y, 16);
}

#[test]
fn test_draw_area_clipping() {
    let mut gpu = GPU::new();

    // Test GP0(E3h) and GP0(E4h) - Drawing Area
    // Set draw area to (100,100)-(200,200)

    // Top-left: x=100, y=100
    gpu.write_gp0(0xE3000064 | (100 << 10));
    assert_eq!(gpu.draw_area.left, 100);
    assert_eq!(gpu.draw_area.top, 100);

    // Bottom-right: x=200, y=200
    gpu.write_gp0(0xE40000C8 | (200 << 10));
    assert_eq!(gpu.draw_area.right, 200);
    assert_eq!(gpu.draw_area.bottom, 200);
}

#[test]
fn test_draw_offset() {
    let mut gpu = GPU::new();

    // Test GP0(E5h) - Drawing Offset
    // Test positive offset (10, 20)
    let x = 10u32;
    let y = 20u32;
    gpu.write_gp0(0xE5000000 | x | (y << 11));
    assert_eq!(gpu.draw_offset.0, 10);
    assert_eq!(gpu.draw_offset.1, 20);
}

#[test]
fn test_draw_offset_negative() {
    let mut gpu = GPU::new();

    // Test negative offset (-20, -30)
    // Need to encode as 11-bit signed values
    let x = ((-20i16) as u16 as u32) & 0x7FF;
    let y = ((-30i16) as u16 as u32) & 0x7FF;
    gpu.write_gp0(0xE5000000 | x | (y << 11));

    assert_eq!(gpu.draw_offset.0, -20);
    assert_eq!(gpu.draw_offset.1, -30);
}

#[test]
fn test_draw_offset_sign_extension() {
    let mut gpu = GPU::new();

    // Test sign extension at boundary (1023 = 0x3FF, should stay positive)
    gpu.write_gp0(0xE50003FF); // x = 1023, y = 0
    assert_eq!(gpu.draw_offset.0, 1023);

    // Test sign extension at boundary (-1024 = 0x400, should be negative)
    gpu.write_gp0(0xE5000400); // x = -1024 (0x400 with sign extension)
    assert_eq!(gpu.draw_offset.0, -1024);
}

#[test]
fn test_mask_settings() {
    let mut gpu = GPU::new();

    // Test GP0(E6h) - Mask Bit Setting
    // Test set mask bit enabled
    gpu.write_gp0(0xE6000001); // Bit 0 = 1 (set mask)
    assert!(gpu.status.set_mask_bit);

    // Test check mask bit enabled
    gpu.write_gp0(0xE6000002); // Bit 1 = 1 (check mask)
    assert!(!gpu.status.draw_pixels); // draw_pixels is inverted

    // Test both enabled
    gpu.write_gp0(0xE6000003); // Both bits set
    assert!(gpu.status.set_mask_bit);
    assert!(!gpu.status.draw_pixels);

    // Test both disabled
    gpu.write_gp0(0xE6000000); // Both bits clear
    assert!(!gpu.status.set_mask_bit);
    assert!(gpu.status.draw_pixels);
}

#[test]
fn test_draw_area_updates_rasterizer() {
    let mut gpu = GPU::new();

    // Set custom drawing area
    gpu.write_gp0(0xE3000032 | (50 << 10)); // Top-left: (50, 50)
    gpu.write_gp0(0xE40000C8 | (150 << 10)); // Bottom-right: (200, 150)

    // Verify the rasterizer clip rect is updated
    // (We can't directly check this, but we can verify drawing respects it)
    // This is implicitly tested by the rasterizer tests
    assert_eq!(gpu.draw_area.left, 50);
    assert_eq!(gpu.draw_area.right, 200);
    assert_eq!(gpu.draw_area.top, 50);
    assert_eq!(gpu.draw_area.bottom, 150);
}

#[test]
fn test_texture_window_default() {
    let gpu = GPU::new();

    // Default texture window should have all zeros
    assert_eq!(gpu.texture_window.mask_x, 0);
    assert_eq!(gpu.texture_window.mask_y, 0);
    assert_eq!(gpu.texture_window.offset_x, 0);
    assert_eq!(gpu.texture_window.offset_y, 0);
}

#[test]
fn test_multiple_draw_mode_changes() {
    let mut gpu = GPU::new();

    // Change draw mode multiple times
    gpu.write_gp0(0xE1000001); // Page X = 1 (64)
    assert_eq!(gpu.draw_mode.texture_page_x_base, 64);

    gpu.write_gp0(0xE1000002); // Page X = 2 (128)
    assert_eq!(gpu.draw_mode.texture_page_x_base, 128);

    gpu.write_gp0(0xE1000003); // Page X = 3 (192)
    assert_eq!(gpu.draw_mode.texture_page_x_base, 192);
}
