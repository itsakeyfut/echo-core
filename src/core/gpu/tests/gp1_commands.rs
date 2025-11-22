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

//! GP1 command tests
//! Tests for GP1 control commands (display control, DMA, etc.)

use super::super::*;

#[test]
fn test_gp1_reset_gpu() {
    let mut gpu = GPU::new();
    gpu.display_mode.display_disabled = false;
    gpu.status.display_disabled = false;
    gpu.command_fifo.push_back(0x12345678);

    gpu.write_gp1(0x00000000);

    assert!(gpu.display_mode.display_disabled);
    assert!(gpu.status.display_disabled);
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_gp1_reset_preserves_vram() {
    let mut gpu = GPU::new();

    // Write some data to VRAM
    gpu.write_vram(100, 100, 0xABCD);
    gpu.write_vram(500, 250, 0x1234);
    gpu.write_vram(1023, 511, 0x5678);

    // Reset via GP1 command
    gpu.write_gp1(0x00000000);

    // VRAM should be preserved (not cleared)
    assert_eq!(gpu.read_vram(100, 100), 0xABCD);
    assert_eq!(gpu.read_vram(500, 250), 0x1234);
    assert_eq!(gpu.read_vram(1023, 511), 0x5678);

    // But state should be reset
    assert!(gpu.display_mode.display_disabled);
    assert!(gpu.command_fifo.is_empty());
}

#[test]
fn test_gp1_reset_command_buffer() {
    let mut gpu = GPU::new();
    gpu.command_fifo.push_back(0x12345678);
    gpu.command_fifo.push_back(0x9ABCDEF0);

    gpu.write_gp1(0x01000000);

    assert!(gpu.command_fifo.is_empty());
    assert!(gpu.vram_transfer.is_none());
}

#[test]
fn test_gp1_acknowledge_interrupt() {
    let mut gpu = GPU::new();
    gpu.status.interrupt_request = true;

    gpu.write_gp1(0x02000000);

    assert!(!gpu.status.interrupt_request);
}

#[test]
fn test_gp1_display_enable() {
    let mut gpu = GPU::new();

    // Enable display (bit 0 = 0)
    gpu.write_gp1(0x03000000);
    assert!(!gpu.display_mode.display_disabled);
    assert!(!gpu.status.display_disabled);

    // Disable display (bit 0 = 1)
    gpu.write_gp1(0x03000001);
    assert!(gpu.display_mode.display_disabled);
    assert!(gpu.status.display_disabled);
}

#[test]
fn test_gp1_dma_direction() {
    let mut gpu = GPU::new();

    // Test all DMA directions
    gpu.write_gp1(0x04000000);
    assert_eq!(gpu.status.dma_direction, 0);

    gpu.write_gp1(0x04000001);
    assert_eq!(gpu.status.dma_direction, 1);

    gpu.write_gp1(0x04000002);
    assert_eq!(gpu.status.dma_direction, 2);

    gpu.write_gp1(0x04000003);
    assert_eq!(gpu.status.dma_direction, 3);
}

#[test]
fn test_gp1_display_area_start() {
    let mut gpu = GPU::new();

    // Set display area start to (8, 16)
    gpu.write_gp1(0x05000008 | (0x10 << 10));
    assert_eq!(gpu.display_area.x, 8);
    assert_eq!(gpu.display_area.y, 16);

    // Test with different coordinates
    gpu.write_gp1(0x05000100 | (0x80 << 10));
    assert_eq!(gpu.display_area.x, 256);
    assert_eq!(gpu.display_area.y, 128);
}

#[test]
fn test_gp1_horizontal_display_range() {
    let mut gpu = GPU::new();

    // Set horizontal range from 100 to 400 (width = 300)
    gpu.write_gp1(0x06000064 | (0x190 << 12));
    assert_eq!(gpu.display_area.width, 300);

    // Test with different values
    gpu.write_gp1(0x06000000 | (0x280 << 12));
    assert_eq!(gpu.display_area.width, 640);
}

#[test]
fn test_gp1_vertical_display_range() {
    let mut gpu = GPU::new();

    // Set vertical range from 16 to 256 (height = 240)
    gpu.write_gp1(0x07000010 | (0x100 << 10));
    assert_eq!(gpu.display_area.height, 240);

    // Test with different values
    gpu.write_gp1(0x07000020 | (0x200 << 10));
    assert_eq!(gpu.display_area.height, 480);
}

#[test]
fn test_gp1_display_mode_320x240_ntsc() {
    let mut gpu = GPU::new();

    // 320x240 NTSC 15-bit non-interlaced
    // Bits: HR1=1, VRes=0, VideoMode=0, ColorDepth=0, Interlace=0, HR2=0
    gpu.write_gp1(0x08000001);

    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);
    assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R240);
    assert_eq!(gpu.display_mode.video_mode, VideoMode::NTSC);
    assert_eq!(
        gpu.display_mode.display_area_color_depth,
        ColorDepth::C15Bit
    );
    assert!(!gpu.display_mode.interlaced);

    // Check status bits are updated
    assert_eq!(gpu.status.horizontal_res_1, 1);
    assert_eq!(gpu.status.horizontal_res_2, 0);
    assert!(!gpu.status.vertical_res);
    assert!(!gpu.status.video_mode);
    assert!(!gpu.status.display_area_color_depth);
    assert!(!gpu.status.vertical_interlace);
}

#[test]
fn test_gp1_display_mode_640x480_pal_interlaced() {
    let mut gpu = GPU::new();

    // 640x480 PAL 24-bit interlaced
    // Bits: HR1=3, VRes=1, VideoMode=1, ColorDepth=1, Interlace=1, HR2=0
    // Value = 0x03 | (1<<2) | (1<<3) | (1<<4) | (1<<5) = 0x3F
    gpu.write_gp1(0x0800003F);

    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R640);
    assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R480);
    assert_eq!(gpu.display_mode.video_mode, VideoMode::PAL);
    assert_eq!(
        gpu.display_mode.display_area_color_depth,
        ColorDepth::C24Bit
    );
    assert!(gpu.display_mode.interlaced);

    // Check status bits are updated
    assert_eq!(gpu.status.horizontal_res_1, 3);
    assert_eq!(gpu.status.horizontal_res_2, 0);
    assert!(gpu.status.vertical_res);
    assert!(gpu.status.video_mode);
    assert!(gpu.status.display_area_color_depth);
    assert!(gpu.status.vertical_interlace);
}

#[test]
fn test_gp1_display_mode_368_horizontal() {
    let mut gpu = GPU::new();

    // 368 width mode (HR2=1, HR1=0)
    // Bits: HR1=0, HR2=1
    // Value = 0x00 | (1<<6) = 0x40
    gpu.write_gp1(0x08000040);

    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R368);
    assert_eq!(gpu.status.horizontal_res_1, 0);
    assert_eq!(gpu.status.horizontal_res_2, 1);
}

#[test]
fn test_gp1_display_mode_384_horizontal() {
    let mut gpu = GPU::new();

    // 384 width mode (HR2=1, HR1=1)
    // Bits: HR1=1, HR2=1
    // Value = 0x01 | (1<<6) = 0x41
    gpu.write_gp1(0x08000041);

    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R384);
    assert_eq!(gpu.status.horizontal_res_1, 1);
    assert_eq!(gpu.status.horizontal_res_2, 1);
}

#[test]
fn test_gp1_display_mode_all_resolutions() {
    let mut gpu = GPU::new();

    // Test 256 width
    gpu.write_gp1(0x08000000);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R256);

    // Test 320 width
    gpu.write_gp1(0x08000001);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);

    // Test 512 width
    gpu.write_gp1(0x08000002);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R512);

    // Test 640 width
    gpu.write_gp1(0x08000003);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R640);

    // Test 368 width (HR2=1, HR1=0)
    gpu.write_gp1(0x08000040);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R368);

    // Test 384 width (HR2=1, HR1=1)
    gpu.write_gp1(0x08000041);
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R384);
}

#[test]
fn test_gp1_display_mode_reverse_flag() {
    let mut gpu = GPU::new();

    // Set reverse flag (bit 7)
    gpu.write_gp1(0x08000080);
    assert!(gpu.status.reverse_flag);

    // Clear reverse flag
    gpu.write_gp1(0x08000000);
    assert!(!gpu.status.reverse_flag);
}

#[test]
fn test_gp1_get_gpu_info() {
    let mut gpu = GPU::new();

    // Test various info types (should not panic)
    gpu.write_gp1(0x10000002); // Texture window
    gpu.write_gp1(0x10000003); // Draw area top left
    gpu.write_gp1(0x10000004); // Draw area bottom right
    gpu.write_gp1(0x10000005); // Draw offset
    gpu.write_gp1(0x10000007); // GPU version
}

#[test]
fn test_gp1_unknown_command() {
    let mut gpu = GPU::new();

    // Unknown command should not panic
    gpu.write_gp1(0xFF000000);
}

#[test]
fn test_gp1_reset_clears_transfer() {
    let mut gpu = GPU::new();

    // Set up a VRAM transfer
    gpu.vram_transfer = Some(VRAMTransfer {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        current_x: 50,
        current_y: 50,
        direction: VRAMTransferDirection::CpuToVram,
    });

    // Reset command buffer should clear transfer
    gpu.write_gp1(0x01000000);
    assert!(gpu.vram_transfer.is_none());
}

#[test]
fn test_gp1_display_area_boundaries() {
    let mut gpu = GPU::new();

    // Test maximum coordinates
    gpu.write_gp1(0x050003FF | (0x1FF << 10)); // Max X=1023, Y=511
    assert_eq!(gpu.display_area.x, 1023);
    assert_eq!(gpu.display_area.y, 511);

    // Test zero coordinates
    gpu.write_gp1(0x05000000);
    assert_eq!(gpu.display_area.x, 0);
    assert_eq!(gpu.display_area.y, 0);
}

#[test]
fn test_gp1_display_range_saturation() {
    let mut gpu = GPU::new();

    // Test horizontal range where x2 < x1 (should saturate to 0)
    gpu.write_gp1(0x06000200 | (0x100 << 12));
    assert_eq!(gpu.display_area.width, 0);

    // Test vertical range where y2 < y1 (should saturate to 0)
    gpu.write_gp1(0x07000200 | (0x100 << 10));
    assert_eq!(gpu.display_area.height, 0);
}

#[test]
fn test_vram_transfer_interrupt_by_gp1_reset() {
    let mut gpu = GPU::new();

    // Start a VRAM transfer
    gpu.write_gp0(0xA0000000);
    gpu.write_gp0(0x00000000);
    gpu.write_gp0(0x00010001);
    assert!(gpu.vram_transfer.is_some());

    // Reset command buffer via GP1
    gpu.write_gp1(0x01000000);

    // Transfer should be cancelled
    assert!(gpu.vram_transfer.is_none());
    assert!(gpu.command_fifo.is_empty());
}
