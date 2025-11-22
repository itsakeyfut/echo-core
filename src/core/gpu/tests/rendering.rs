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

//! Rendering primitive tests
//! Tests for rendering triangles, lines, rectangles, and other primitives

use super::super::*;

#[test]
fn test_color_conversion() {
    let color = Color {
        r: 255,
        g: 128,
        b: 64,
    };
    let rgb15 = color.to_rgb15();

    // Verify 15-bit conversion
    assert_eq!(rgb15 & 0x1F, 31); // R: 255 >> 3 = 31
    assert_eq!((rgb15 >> 5) & 0x1F, 16); // G: 128 >> 3 = 16
    assert_eq!((rgb15 >> 10) & 0x1F, 8); // B: 64 >> 3 = 8
}

#[test]
fn test_color_from_u32() {
    let color = Color::from_u32(0x00FF8040);
    assert_eq!(color.r, 0x40);
    assert_eq!(color.g, 0x80);
    assert_eq!(color.b, 0xFF);
}

#[test]
fn test_vertex_from_u32() {
    let v = Vertex::from_u32(0x00640032); // x=50, y=100
    assert_eq!(v.x, 50);
    assert_eq!(v.y, 100);
}

#[test]
fn test_vertex_from_u32_negative() {
    // Test negative coordinates (signed 16-bit)
    let v = Vertex::from_u32(0xFFFFFFFF);
    assert_eq!(v.x, -1);
    assert_eq!(v.y, -1);
}

#[test]
fn test_drawing_offset_applied() {
    let mut gpu = GPU::new();
    gpu.draw_offset = (10, 20);

    let vertices = [
        Vertex { x: 0, y: 0 },
        Vertex { x: 10, y: 0 },
        Vertex { x: 5, y: 10 },
    ];

    let color = Color { r: 255, g: 0, b: 0 };

    // Should not crash
    gpu.render_monochrome_triangle(&vertices, &color, false);
}

#[test]
fn test_quad_splits_into_two_triangles() {
    let mut gpu = GPU::new();

    // Render a quad - internally it should split into two triangles
    let vertices = [
        Vertex { x: 0, y: 0 },
        Vertex { x: 100, y: 0 },
        Vertex { x: 100, y: 100 },
        Vertex { x: 0, y: 100 },
    ];

    let color = Color {
        r: 255,
        g: 255,
        b: 255,
    };

    // Should not crash - actual rendering stub is called twice internally
    gpu.render_monochrome_quad(&vertices, &color, false);
}

#[test]
fn test_triangle_rasterization() {
    let mut gpu = GPU::new();

    // Draw a simple triangle
    let vertices = [
        Vertex { x: 10, y: 10 },
        Vertex { x: 50, y: 10 },
        Vertex { x: 30, y: 50 },
    ];
    let color = Color { r: 255, g: 0, b: 0 }; // Red

    gpu.render_monochrome_triangle(&vertices, &color, false);

    // Verify some pixels inside the triangle are set
    let pixel = gpu.read_vram(30, 20);
    assert_ne!(pixel, 0); // Should be red (non-zero)

    // Verify the color is approximately correct (red in 5-5-5 RGB)
    let expected_red = 0x001F; // Red = 31 in 5-bit (255 >> 3)
    assert_eq!(pixel & 0x1F, expected_red & 0x1F);
}

#[test]
fn test_rasterizer_clipping() {
    let mut gpu = GPU::new();

    // Set a restricted drawing area
    gpu.draw_area = DrawingArea {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    gpu.update_rasterizer_clip_rect();

    // Draw triangle partially outside clip area
    let vertices = [
        Vertex { x: 50, y: 50 },
        Vertex { x: 150, y: 150 },
        Vertex { x: 250, y: 150 },
    ];
    let color = Color { r: 0, g: 255, b: 0 }; // Green

    gpu.render_monochrome_triangle(&vertices, &color, false);

    // Pixel outside clip area should not be drawn
    assert_eq!(gpu.read_vram(50, 100), 0);

    // Pixel inside both triangle and clip area should be drawn
    let pixel = gpu.read_vram(150, 150);
    assert_ne!(pixel, 0);
}

#[test]
fn test_degenerate_triangle() {
    let mut gpu = GPU::new();

    // Zero-height triangle (all vertices on same scanline)
    let vertices = [
        Vertex { x: 10, y: 10 },
        Vertex { x: 20, y: 10 },
        Vertex { x: 15, y: 10 },
    ];
    let color = Color { r: 255, g: 0, b: 0 };

    // Should not crash
    gpu.render_monochrome_triangle(&vertices, &color, false);

    // No pixels should be drawn (degenerate triangle)
    assert_eq!(gpu.read_vram(10, 10), 0);
    assert_eq!(gpu.read_vram(15, 10), 0);
    assert_eq!(gpu.read_vram(20, 10), 0);
}

#[test]
fn test_framebuffer_generation() {
    let mut gpu = GPU::new();

    // Set display area to 320×240
    gpu.display_area = DisplayArea {
        x: 0,
        y: 0,
        width: 320,
        height: 240,
    };

    // Draw a white pixel at (10, 10)
    gpu.write_vram(10, 10, 0x7FFF); // White in 15-bit RGB

    // Generate framebuffer
    let fb = gpu.get_framebuffer();
    assert_eq!(fb.len(), 320 * 240 * 3);

    // Check the white pixel was converted correctly
    let index = (10 * 320 + 10) * 3;
    assert_eq!(fb[index], 248); // R: 31 << 3 = 248
    assert_eq!(fb[index + 1], 248); // G: 31 << 3 = 248
    assert_eq!(fb[index + 2], 248); // B: 31 << 3 = 248

    // Check a black pixel
    let black_index = (20 * 320 + 20) * 3;
    assert_eq!(fb[black_index], 0);
    assert_eq!(fb[black_index + 1], 0);
    assert_eq!(fb[black_index + 2], 0);
}

#[test]
fn test_framebuffer_color_conversion() {
    let mut gpu = GPU::new();

    gpu.display_area = DisplayArea {
        x: 0,
        y: 0,
        width: 320,
        height: 240,
    };

    // Test red
    gpu.write_vram(0, 0, 0x001F); // Pure red in 15-bit
    let fb = gpu.get_framebuffer();
    assert_eq!(fb[0], 248); // R
    assert_eq!(fb[1], 0); // G
    assert_eq!(fb[2], 0); // B

    // Test green
    gpu.write_vram(1, 0, 0x03E0); // Pure green in 15-bit
    let fb = gpu.get_framebuffer();
    let idx = 3;
    assert_eq!(fb[idx], 0); // R
    assert_eq!(fb[idx + 1], 248); // G
    assert_eq!(fb[idx + 2], 0); // B

    // Test blue
    gpu.write_vram(2, 0, 0x7C00); // Pure blue in 15-bit
    let fb = gpu.get_framebuffer();
    let idx = 6;
    assert_eq!(fb[idx], 0); // R
    assert_eq!(fb[idx + 1], 0); // G
    assert_eq!(fb[idx + 2], 248); // B
}

#[test]
fn test_drawing_offset() {
    let mut gpu = GPU::new();

    // Set a drawing offset
    gpu.draw_offset = (50, 50);

    // Draw a triangle at (0, 0)
    let vertices = [
        Vertex { x: 0, y: 0 },
        Vertex { x: 20, y: 0 },
        Vertex { x: 10, y: 20 },
    ];
    let color = Color { r: 255, g: 0, b: 0 };

    gpu.render_monochrome_triangle(&vertices, &color, false);

    // Triangle should be drawn at (50, 50) due to offset
    let pixel = gpu.read_vram(60, 55); // Center of offset triangle
    assert_ne!(pixel, 0);

    // Original position should be empty
    let pixel_orig = gpu.read_vram(10, 10);
    assert_eq!(pixel_orig, 0);
}

#[test]
fn test_large_triangle() {
    let mut gpu = GPU::new();

    // Draw a large triangle covering significant portion of VRAM
    let vertices = [
        Vertex { x: 0, y: 0 },
        Vertex { x: 500, y: 0 },
        Vertex { x: 250, y: 400 },
    ];
    let color = Color {
        r: 128,
        g: 128,
        b: 128,
    };

    gpu.render_monochrome_triangle(&vertices, &color, false);

    // Check several points inside the triangle are drawn
    assert_ne!(gpu.read_vram(250, 100), 0);
    assert_ne!(gpu.read_vram(250, 200), 0);
    assert_ne!(gpu.read_vram(100, 50), 0);
}

#[test]
fn test_negative_coordinate_triangle() {
    let mut gpu = GPU::new();

    // Triangle with negative coordinates (should be clipped)
    let vertices = [
        Vertex { x: -50, y: -50 },
        Vertex { x: 50, y: 50 },
        Vertex { x: 100, y: -10 },
    ];
    let color = Color {
        r: 255,
        g: 255,
        b: 0,
    };

    // Should not crash
    gpu.render_monochrome_triangle(&vertices, &color, false);

    // Visible portion should be drawn
    let pixel = gpu.read_vram(50, 20);
    assert_ne!(pixel, 0);
}

#[test]
fn test_multiple_triangles() {
    let mut gpu = GPU::new();

    // Draw multiple triangles with different colors
    let vertices1 = [
        Vertex { x: 10, y: 10 },
        Vertex { x: 50, y: 10 },
        Vertex { x: 30, y: 50 },
    ];
    let color1 = Color { r: 255, g: 0, b: 0 }; // Red

    let vertices2 = [
        Vertex { x: 60, y: 10 },
        Vertex { x: 100, y: 10 },
        Vertex { x: 80, y: 50 },
    ];
    let color2 = Color { r: 0, g: 255, b: 0 }; // Green

    gpu.render_monochrome_triangle(&vertices1, &color1, false);
    gpu.render_monochrome_triangle(&vertices2, &color2, false);

    // Check both triangles are drawn
    let pixel1 = gpu.read_vram(30, 20);
    assert_ne!(pixel1, 0);

    let pixel2 = gpu.read_vram(80, 20);
    assert_ne!(pixel2, 0);

    // Colors should be different
    assert_ne!(pixel1, pixel2);
}

#[test]
fn test_line_rendering() {
    let mut gpu = GPU::new();

    let v0 = Vertex { x: 10, y: 10 };
    let v1 = Vertex { x: 50, y: 50 };
    let color = Color {
        r: 255,
        g: 255,
        b: 255,
    };

    gpu.render_line(v0, v1, color, false);

    // Check start and end points
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);

    // Check a point on the line
    assert_ne!(gpu.read_vram(30, 30), 0);
}

#[test]
fn test_line_with_drawing_offset() {
    let mut gpu = GPU::new();
    gpu.draw_offset = (100, 100);

    let v0 = Vertex { x: 10, y: 10 };
    let v1 = Vertex { x: 50, y: 50 };
    let color = Color {
        r: 255,
        g: 255,
        b: 255,
    };

    gpu.render_line(v0, v1, color, false);

    // Line should be drawn at offset position
    assert_ne!(gpu.read_vram(110, 110), 0); // 10 + 100
    assert_ne!(gpu.read_vram(150, 150), 0); // 50 + 100
}

#[test]
fn test_polyline_rendering() {
    let mut gpu = GPU::new();

    let vertices = vec![
        Vertex { x: 10, y: 10 },
        Vertex { x: 50, y: 10 },
        Vertex { x: 50, y: 50 },
        Vertex { x: 10, y: 50 },
        Vertex { x: 10, y: 10 },
    ];
    let color = Color { r: 255, g: 0, b: 0 };

    gpu.render_polyline(&vertices, color, false);

    // Check corners of the square
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);
    assert_ne!(gpu.read_vram(10, 50), 0);

    // Check edges
    assert_ne!(gpu.read_vram(30, 10), 0); // Top edge
    assert_ne!(gpu.read_vram(50, 30), 0); // Right edge
}

#[test]
fn test_gradient_triangle_rendering() {
    let mut gpu = GPU::new();

    let vertices = [
        Vertex { x: 100, y: 100 },
        Vertex { x: 200, y: 100 },
        Vertex { x: 150, y: 200 },
    ];
    let colors = [
        Color { r: 255, g: 0, b: 0 }, // Red
        Color { r: 0, g: 255, b: 0 }, // Green
        Color { r: 0, g: 0, b: 255 }, // Blue
    ];

    gpu.render_gradient_triangle(&vertices, &colors, false);

    // Check that pixels are drawn
    assert_ne!(gpu.read_vram(100, 100), 0); // Vertex 0
    assert_ne!(gpu.read_vram(200, 100), 0); // Vertex 1
    assert_ne!(gpu.read_vram(150, 200), 0); // Vertex 2

    // Check center has interpolated color (not any pure color)
    let center = gpu.read_vram(150, 133);
    assert_ne!(center, 0);
    assert_ne!(center, 0x001F); // Not pure red
    assert_ne!(center, 0x03E0); // Not pure green
    assert_ne!(center, 0x7C00); // Not pure blue
}

#[test]
fn test_gradient_triangle_with_offset() {
    let mut gpu = GPU::new();
    gpu.draw_offset = (50, 50);

    let vertices = [
        Vertex { x: 10, y: 10 },
        Vertex { x: 50, y: 10 },
        Vertex { x: 30, y: 50 },
    ];
    let colors = [
        Color { r: 255, g: 0, b: 0 },
        Color { r: 0, g: 255, b: 0 },
        Color { r: 0, g: 0, b: 255 },
    ];

    gpu.render_gradient_triangle(&vertices, &colors, false);

    // Check with offset applied
    assert_ne!(gpu.read_vram(60, 60), 0); // 10 + 50
    assert_ne!(gpu.read_vram(100, 60), 0); // 50 + 50
}

#[test]
fn test_gradient_quad_rendering() {
    let mut gpu = GPU::new();

    let vertices = [
        Vertex { x: 100, y: 100 },
        Vertex { x: 200, y: 100 },
        Vertex { x: 200, y: 200 },
        Vertex { x: 100, y: 200 },
    ];
    let colors = [
        Color { r: 255, g: 0, b: 0 }, // Red
        Color { r: 0, g: 255, b: 0 }, // Green
        Color { r: 0, g: 0, b: 255 }, // Blue
        Color {
            r: 255,
            g: 255,
            b: 0,
        }, // Yellow
    ];

    gpu.render_gradient_quad(&vertices, &colors, false);

    // Check corners
    assert_ne!(gpu.read_vram(100, 100), 0);
    assert_ne!(gpu.read_vram(200, 100), 0);
    assert_ne!(gpu.read_vram(200, 200), 0);
    assert_ne!(gpu.read_vram(100, 200), 0);

    // Check center is filled
    assert_ne!(gpu.read_vram(150, 150), 0);
}

#[test]
fn test_gradient_smooth_interpolation() {
    let mut gpu = GPU::new();

    // Create a gradient with distinct colors (avoid pure black which is 0x0000)
    let vertices = [
        Vertex { x: 100, y: 100 },
        Vertex { x: 200, y: 100 },
        Vertex { x: 150, y: 200 },
    ];
    let colors = [
        Color { r: 255, g: 0, b: 0 }, // Red
        Color { r: 0, g: 255, b: 0 }, // Green
        Color { r: 0, g: 0, b: 255 }, // Blue
    ];

    gpu.render_gradient_triangle(&vertices, &colors, false);

    // Verify vertices have colors
    assert_ne!(gpu.read_vram(100, 100), 0); // Red vertex
    assert_ne!(gpu.read_vram(200, 100), 0); // Green vertex
    assert_ne!(gpu.read_vram(150, 200), 0); // Blue vertex

    // Verify center has interpolated color
    let center = gpu.read_vram(150, 133);
    assert_ne!(center, 0);
    assert_ne!(center, 0x001F); // Not pure red
    assert_ne!(center, 0x03E0); // Not pure green
    assert_ne!(center, 0x7C00); // Not pure blue
}

#[test]
fn test_shaded_line_rendering() {
    let mut gpu = GPU::new();

    let v0 = Vertex { x: 10, y: 10 };
    let c0 = Color { r: 255, g: 0, b: 0 }; // Red
    let v1 = Vertex { x: 50, y: 50 };
    let c1 = Color { r: 0, g: 0, b: 255 }; // Blue

    gpu.render_shaded_line(v0, c0, v1, c1, false);

    // Check start and end points are drawn
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);

    // Check a point on the line (should have interpolated color)
    assert_ne!(gpu.read_vram(30, 30), 0);
}

#[test]
fn test_shaded_line_with_drawing_offset() {
    let mut gpu = GPU::new();
    gpu.draw_offset = (100, 100);

    let v0 = Vertex { x: 10, y: 10 };
    let c0 = Color { r: 255, g: 0, b: 0 };
    let v1 = Vertex { x: 50, y: 50 };
    let c1 = Color { r: 0, g: 255, b: 0 };

    gpu.render_shaded_line(v0, c0, v1, c1, false);

    // Line should be drawn at offset position
    assert_ne!(gpu.read_vram(110, 110), 0); // 10 + 100
    assert_ne!(gpu.read_vram(150, 150), 0); // 50 + 100
}

#[test]
fn test_shaded_polyline_rendering() {
    let mut gpu = GPU::new();

    let vertices = vec![
        Vertex { x: 10, y: 10 },
        Vertex { x: 50, y: 10 },
        Vertex { x: 50, y: 50 },
    ];
    let colors = vec![
        Color { r: 255, g: 0, b: 0 }, // Red
        Color { r: 0, g: 255, b: 0 }, // Green
        Color { r: 0, g: 0, b: 255 }, // Blue
    ];

    gpu.render_shaded_polyline(&vertices, &colors, false);

    // Check vertices
    assert_ne!(gpu.read_vram(10, 10), 0);
    assert_ne!(gpu.read_vram(50, 10), 0);
    assert_ne!(gpu.read_vram(50, 50), 0);

    // Check points on edges (should have interpolated colors)
    assert_ne!(gpu.read_vram(30, 10), 0); // On first segment
    assert_ne!(gpu.read_vram(50, 30), 0); // On second segment
}
