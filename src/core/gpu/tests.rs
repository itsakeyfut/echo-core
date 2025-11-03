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

//! GPU module tests

use super::*;

#[test]
fn test_gpu_initialization() {
    let gpu = GPU::new();

    // Verify VRAM size
    assert_eq!(gpu.vram.len(), GPU::VRAM_SIZE);

    // All VRAM should be initialized to black
    assert!(gpu.vram.iter().all(|&pixel| pixel == 0x0000));
}

#[test]
fn test_vram_read_write() {
    let mut gpu = GPU::new();

    // Write a pixel
    gpu.write_vram(100, 100, 0x7FFF); // White
    assert_eq!(gpu.read_vram(100, 100), 0x7FFF);

    // Test bounds (corners)
    gpu.write_vram(0, 0, 0x1234);
    assert_eq!(gpu.read_vram(0, 0), 0x1234);

    gpu.write_vram(1023, 511, 0x5678);
    assert_eq!(gpu.read_vram(1023, 511), 0x5678);
}

#[test]
fn test_vram_index_calculation() {
    let gpu = GPU::new();

    // Top-left corner
    assert_eq!(gpu.vram_index(0, 0), 0);

    // One row down
    assert_eq!(gpu.vram_index(0, 1), GPU::VRAM_WIDTH);

    // Bottom-right corner
    assert_eq!(gpu.vram_index(1023, 511), (GPU::VRAM_WIDTH * 511) + 1023);

    // Test wrapping (coordinates beyond bounds wrap around)
    assert_eq!(gpu.vram_index(1024, 0), gpu.vram_index(0, 0));
    assert_eq!(gpu.vram_index(0, 512), gpu.vram_index(0, 0));
}

#[test]
fn test_default_state() {
    let gpu = GPU::new();

    // Check default drawing area (full VRAM)
    assert_eq!(gpu.draw_area.left, 0);
    assert_eq!(gpu.draw_area.top, 0);
    assert_eq!(gpu.draw_area.right, 1023);
    assert_eq!(gpu.draw_area.bottom, 511);

    // Check display is initially disabled
    assert!(gpu.display_mode.display_disabled);

    // Check default resolution (320×240, NTSC)
    assert_eq!(gpu.display_mode.horizontal_res, HorizontalRes::R320);
    assert_eq!(gpu.display_mode.vertical_res, VerticalRes::R240);
    assert_eq!(gpu.display_mode.video_mode, VideoMode::NTSC);

    // Check default display area
    assert_eq!(gpu.display_area.width, 320);
    assert_eq!(gpu.display_area.height, 240);
}

#[test]
fn test_status_register() {
    let gpu = GPU::new();
    let status = gpu.status();

    // Status register should be a valid 32-bit value
    assert_eq!(status & 0x1FFF_FFFF, status); // Check no reserved bits

    // Display should be disabled initially
    assert_ne!(status & (1 << 23), 0);

    // Ready flags should be set
    assert_ne!(status & (1 << 26), 0); // Ready to receive command
    assert_ne!(status & (1 << 27), 0); // Ready to send VRAM
    assert_ne!(status & (1 << 28), 0); // Ready to receive DMA
}

#[test]
fn test_gpu_reset() {
    let mut gpu = GPU::new();

    // Modify some state
    gpu.write_vram(500, 250, 0xFFFF);
    gpu.draw_offset = (100, 100);
    gpu.display_mode.display_disabled = false;

    // Reset
    gpu.reset();

    // Verify state is reset
    assert_eq!(gpu.read_vram(500, 250), 0x0000);
    assert_eq!(gpu.draw_offset, (0, 0));
    assert!(gpu.display_mode.display_disabled);

    // Verify default drawing area
    assert_eq!(gpu.draw_area.left, 0);
    assert_eq!(gpu.draw_area.right, 1023);
}

#[test]
fn test_vram_wrapping() {
    let mut gpu = GPU::new();

    // Write using wrapped coordinates
    gpu.write_vram(1024, 512, 0xABCD); // Should wrap to (0, 0)
    assert_eq!(gpu.read_vram(0, 0), 0xABCD);

    gpu.write_vram(1025, 513, 0x1234); // Should wrap to (1, 1)
    assert_eq!(gpu.read_vram(1, 1), 0x1234);
}

#[test]
fn test_multiple_pixel_operations() {
    let mut gpu = GPU::new();

    // Write a pattern
    for i in 0..10 {
        gpu.write_vram(i * 10, i * 10, 0x1000 + i);
    }

    // Verify pattern
    for i in 0..10 {
        assert_eq!(gpu.read_vram(i * 10, i * 10), 0x1000 + i);
    }
}

#[test]
fn test_vram_size_constants() {
    assert_eq!(GPU::VRAM_WIDTH, 1024);
    assert_eq!(GPU::VRAM_HEIGHT, 512);
    assert_eq!(GPU::VRAM_SIZE, 1024 * 512);
    assert_eq!(GPU::VRAM_SIZE, 524_288);
}

#[test]
fn test_gpu_tick() {
    let mut gpu = GPU::new();

    // Tick should not panic
    gpu.tick(100);
    gpu.tick(1000);
}

#[test]
fn test_default_trait() {
    let gpu1 = GPU::new();
    let gpu2 = GPU::default();

    // Both should have the same initial state
    assert_eq!(gpu1.vram.len(), gpu2.vram.len());
    assert_eq!(gpu1.read_vram(0, 0), gpu2.read_vram(0, 0));
}

// GP1 Command Tests

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

// GP0 VRAM Transfer Tests

#[test]
fn test_cpu_to_vram_transfer() {
    let mut gpu = GPU::new();

    // Start transfer: position (10, 20), size 2x2
    gpu.write_gp0(0xA0000000);
    gpu.write_gp0(0x0014000A); // y=20, x=10
    gpu.write_gp0(0x00020002); // height=2, width=2

    // Write 2 u32 words (4 pixels total for 2x2)
    gpu.write_gp0(0x7FFF7FFF); // Two white pixels
    gpu.write_gp0(0x00000000); // Two black pixels

    // Verify pixels written correctly
    assert_eq!(gpu.read_vram(10, 20), 0x7FFF);
    assert_eq!(gpu.read_vram(11, 20), 0x7FFF);
    assert_eq!(gpu.read_vram(10, 21), 0x0000);
    assert_eq!(gpu.read_vram(11, 21), 0x0000);

    // Transfer should be complete
    assert!(gpu.vram_transfer.is_none());
}

#[test]
fn test_cpu_to_vram_transfer_wrapping() {
    let mut gpu = GPU::new();

    // Test coordinate wrapping at VRAM boundary
    gpu.write_gp0(0xA0000000);
    gpu.write_gp0(0x000003FF); // position (1023, 0)
    gpu.write_gp0(0x00010002); // size 2x1

    // Write 1 u32 word (2 pixels)
    gpu.write_gp0(0x12345678);

    // Verify wrapping: second pixel wraps to x=0 same row
    // VRAM coordinates wrap independently from transfer coordinates
    assert_eq!(gpu.read_vram(1023, 0), 0x5678);
    assert_eq!(gpu.read_vram(0, 0), 0x1234); // Wrapped to x=0 same row
}

#[test]
fn test_cpu_to_vram_transfer_odd_width() {
    let mut gpu = GPU::new();

    // Test transfer with odd width (3 pixels = 2 u32 words)
    gpu.write_gp0(0xA0000000);
    gpu.write_gp0(0x00000000); // position (0, 0)
    gpu.write_gp0(0x00010003); // size 3x1

    // Write 2 u32 words (4 pixels, but only 3 are in transfer)
    gpu.write_gp0(0xAAAABBBB);
    gpu.write_gp0(0xCCCCDDDD);

    // Verify only 3 pixels written
    assert_eq!(gpu.read_vram(0, 0), 0xBBBB);
    assert_eq!(gpu.read_vram(1, 0), 0xAAAA);
    assert_eq!(gpu.read_vram(2, 0), 0xDDDD);

    // Transfer should be complete after 3 pixels
    assert!(gpu.vram_transfer.is_none());
}

#[test]
fn test_vram_to_cpu_transfer() {
    let mut gpu = GPU::new();

    // Setup VRAM with test pattern
    gpu.write_vram(100, 100, 0x1234);
    gpu.write_vram(101, 100, 0x5678);
    gpu.write_vram(102, 100, 0x9ABC);
    gpu.write_vram(103, 100, 0xDEF0);

    // Start read transfer: position (100, 100), size 4x1
    gpu.write_gp0(0xC0000000);
    gpu.write_gp0(0x00640064); // position (100, 100)
    gpu.write_gp0(0x00010004); // size 4x1

    // Read data (2 pixels per read)
    let data1 = gpu.read_gpuread();
    assert_eq!(data1 & 0xFFFF, 0x1234);
    assert_eq!((data1 >> 16) & 0xFFFF, 0x5678);

    let data2 = gpu.read_gpuread();
    assert_eq!(data2 & 0xFFFF, 0x9ABC);
    assert_eq!((data2 >> 16) & 0xFFFF, 0xDEF0);

    // Transfer should be complete
    assert!(gpu.vram_transfer.is_none());
    assert!(!gpu.status.ready_to_send_vram);
}

#[test]
fn test_vram_to_cpu_transfer_odd_width() {
    let mut gpu = GPU::new();

    // Setup VRAM with test pattern
    gpu.write_vram(50, 50, 0xAAAA);
    gpu.write_vram(51, 50, 0xBBBB);
    gpu.write_vram(52, 50, 0xCCCC);

    // Start read transfer: position (50, 50), size 3x1
    gpu.write_gp0(0xC0000000);
    gpu.write_gp0(0x00320032); // position (50, 50)
    gpu.write_gp0(0x00010003); // size 3x1

    // Read first 2 pixels
    let data1 = gpu.read_gpuread();
    assert_eq!(data1 & 0xFFFF, 0xAAAA);
    assert_eq!((data1 >> 16) & 0xFFFF, 0xBBBB);

    // Read remaining pixel (second pixel should be 0 as transfer ends)
    let data2 = gpu.read_gpuread();
    assert_eq!(data2 & 0xFFFF, 0xCCCC);
    assert_eq!((data2 >> 16) & 0xFFFF, 0); // No more data

    // Transfer should be complete
    assert!(gpu.vram_transfer.is_none());
}

#[test]
fn test_vram_to_cpu_status_flag() {
    let mut gpu = GPU::new();

    // Initially not ready to send
    assert!(gpu.status.ready_to_send_vram);

    // Start transfer
    gpu.write_gp0(0xC0000000);
    gpu.write_gp0(0x00000000);
    gpu.write_gp0(0x00010001); // 1x1 transfer

    // Should be ready to send
    assert!(gpu.status.ready_to_send_vram);

    // Read data
    let _ = gpu.read_gpuread();

    // Should no longer be ready after transfer complete
    assert!(!gpu.status.ready_to_send_vram);
}

#[test]
fn test_vram_to_vram_transfer() {
    let mut gpu = GPU::new();

    // Write source data
    gpu.write_vram(0, 0, 0xAAAA);
    gpu.write_vram(1, 0, 0xBBBB);
    gpu.write_vram(0, 1, 0xCCCC);
    gpu.write_vram(1, 1, 0xDDDD);

    // Copy 2x2 rectangle from (0,0) to (10,10)
    gpu.write_gp0(0x80000000);
    gpu.write_gp0(0x00000000); // src (0, 0)
    gpu.write_gp0(0x000A000A); // dst (10, 10)
    gpu.write_gp0(0x00020002); // size 2x2

    // Verify destination
    assert_eq!(gpu.read_vram(10, 10), 0xAAAA);
    assert_eq!(gpu.read_vram(11, 10), 0xBBBB);
    assert_eq!(gpu.read_vram(10, 11), 0xCCCC);
    assert_eq!(gpu.read_vram(11, 11), 0xDDDD);

    // Source should be unchanged
    assert_eq!(gpu.read_vram(0, 0), 0xAAAA);
    assert_eq!(gpu.read_vram(1, 0), 0xBBBB);
}

#[test]
fn test_vram_to_vram_transfer_overlapping() {
    let mut gpu = GPU::new();

    // Write source data in a line
    for i in 0..5 {
        gpu.write_vram(i, 0, 0x1000 + i);
    }

    // Copy overlapping region: (0,0) to (2,0), size 3x1
    // This tests that we use a temporary buffer
    gpu.write_gp0(0x80000000);
    gpu.write_gp0(0x00000000); // src (0, 0)
    gpu.write_gp0(0x00000002); // dst (2, 0)
    gpu.write_gp0(0x00010003); // size 3x1

    // Verify copy worked correctly despite overlap
    assert_eq!(gpu.read_vram(2, 0), 0x1000);
    assert_eq!(gpu.read_vram(3, 0), 0x1001);
    assert_eq!(gpu.read_vram(4, 0), 0x1002);
}

#[test]
fn test_vram_to_vram_transfer_wrapping() {
    let mut gpu = GPU::new();

    // Write at edge of VRAM
    gpu.write_vram(1023, 511, 0xABCD);
    gpu.write_vram(0, 0, 0x1234);

    // Copy from edge, should wrap
    gpu.write_gp0(0x80000000);
    gpu.write_gp0(0x01FF03FF); // src (1023, 511)
    gpu.write_gp0(0x00640064); // dst (100, 100)
    gpu.write_gp0(0x00020002); // size 2x2

    // Verify wrapped copy
    assert_eq!(gpu.read_vram(100, 100), 0xABCD);
    // Other pixels will be from wrapped coordinates
}

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

#[test]
fn test_cpu_to_vram_size_alignment() {
    let mut gpu = GPU::new();

    // Test that size of 0 wraps to maximum (1024×512 per PSX hardware behavior)
    gpu.write_gp0(0xA0000000);
    gpu.write_gp0(0x00000000); // position (0, 0)
    gpu.write_gp0(0x00000000); // size 0x0 (wraps to 1024×512)

    // Write 1 data word (2 pixels)
    gpu.write_gp0(0x12345678);

    // Verify first two pixels were written
    assert_eq!(gpu.read_vram(0, 0), 0x5678);
    assert_eq!(gpu.read_vram(1, 0), 0x1234);

    // Transfer should still be in progress (1024×512 is much larger than 2 pixels)
    assert!(gpu.vram_transfer.is_some());
}

#[test]
fn test_vram_to_cpu_multiline() {
    let mut gpu = GPU::new();

    // Write 2x2 pattern
    gpu.write_vram(0, 0, 0xAAAA);
    gpu.write_vram(1, 0, 0xBBBB);
    gpu.write_vram(0, 1, 0xCCCC);
    gpu.write_vram(1, 1, 0xDDDD);

    // Read 2x2 area
    gpu.write_gp0(0xC0000000);
    gpu.write_gp0(0x00000000); // position (0, 0)
    gpu.write_gp0(0x00020002); // size 2x2

    // Read first row
    let data1 = gpu.read_gpuread();
    assert_eq!(data1 & 0xFFFF, 0xAAAA);
    assert_eq!((data1 >> 16) & 0xFFFF, 0xBBBB);

    // Read second row
    let data2 = gpu.read_gpuread();
    assert_eq!(data2 & 0xFFFF, 0xCCCC);
    assert_eq!((data2 >> 16) & 0xFFFF, 0xDDDD);

    assert!(gpu.vram_transfer.is_none());
}

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
fn test_monochrome_triangle_parsing() {
    let mut gpu = GPU::new();

    // Monochrome triangle command
    gpu.write_gp0(0x20FF0000); // Red triangle
    gpu.write_gp0(0x00640032); // V1: (50, 100)
    gpu.write_gp0(0x00C80096); // V2: (200, 150)
    gpu.write_gp0(0x00320064); // V3: (50, 100)

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
    gpu.write_gp0(0x00640000); // V2: (100, 0)
    gpu.write_gp0(0x00640064); // V3: (100, 100)
    gpu.write_gp0(0x00000064); // V4: (0, 100)

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

// ============================================================================
// Rasterizer Tests
// ============================================================================

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

// ========== Line Drawing Tests ==========

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

// ========== Gradient Triangle Tests ==========

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

// ============================================================================
// Drawing Mode Command Tests
// ============================================================================

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

// VBlank and HBlank Tests

#[test]
fn test_vblank_timing() {
    let mut gpu = GPU::new();

    // Run until VBlank
    let mut vblank_count = 0;
    for _ in 0..1_000_000 {
        let (vblank, _) = gpu.tick(1);
        if vblank {
            vblank_count += 1;
        }
    }

    // Should have multiple VBlanks (at least 1)
    assert!(
        vblank_count > 0,
        "Expected at least one VBlank in 1 million cycles"
    );
}

#[test]
fn test_scanline_counting() {
    let mut gpu = GPU::new();

    // Initially at scanline 0
    assert_eq!(gpu.get_scanline(), 0);

    // Tick one scanline worth of dots
    gpu.tick(GPU::DOTS_PER_SCANLINE as u32);

    // Should be at scanline 1 now
    assert_eq!(gpu.get_scanline(), 1);
}

#[test]
fn test_vblank_flag_in_status() {
    let mut gpu = GPU::new();

    // Initially not in VBlank
    let status_before = gpu.status();
    assert_eq!(
        status_before & (1 << 31),
        0,
        "VBlank flag should be 0 initially"
    );

    // Manually set to VBlank region for testing
    gpu.scanline = GPU::VBLANK_START;
    gpu.in_vblank = true;

    // VBlank flag should be set in status
    let status_vblank = gpu.status();
    assert_ne!(
        status_vblank & (1 << 31),
        0,
        "VBlank flag should be 1 when in VBlank"
    );

    // Move out of VBlank
    gpu.scanline = 0;
    gpu.in_vblank = false;

    // VBlank flag should be clear
    let status_after = gpu.status();
    assert_eq!(
        status_after & (1 << 31),
        0,
        "VBlank flag should be 0 outside VBlank"
    );
}

#[test]
fn test_vblank_region() {
    let mut gpu = GPU::new();

    // Tick to just before VBlank (one cycle before the scanline boundary)
    let cycles_to_vblank_start =
        (GPU::VBLANK_START as u32 * GPU::DOTS_PER_SCANLINE as u32) - gpu.dots as u32 - 1;
    gpu.tick(cycles_to_vblank_start);

    // Should not be in VBlank yet
    assert!(!gpu.is_in_vblank(), "Should not be in VBlank yet");
    assert_eq!(gpu.get_scanline(), GPU::VBLANK_START - 1);

    // Tick one more cycle to cross the scanline boundary and enter VBlank
    let (vblank_irq, _) = gpu.tick(1);

    // Should now be in VBlank and VBlank interrupt should trigger
    assert!(vblank_irq, "VBlank interrupt should be triggered");
    assert!(gpu.is_in_vblank(), "Should be in VBlank now");
    assert_eq!(gpu.get_scanline(), GPU::VBLANK_START);
}

#[test]
fn test_scanline_wraparound() {
    let mut gpu = GPU::new();

    // Tick to end of frame
    let cycles_to_end = GPU::SCANLINES_PER_FRAME as u32 * GPU::DOTS_PER_SCANLINE as u32;
    gpu.tick(cycles_to_end);

    // Should wrap back to scanline 0
    assert_eq!(gpu.get_scanline(), 0, "Scanline should wrap to 0");
    assert!(!gpu.is_in_vblank(), "Should not be in VBlank at scanline 0");
}

#[test]
fn test_hblank_signal() {
    let mut gpu = GPU::new();

    // HBlank should trigger at end of each scanline
    let mut hblank_count = 0;

    // Tick for multiple scanlines
    for _ in 0..10 {
        let (_, hblank) = gpu.tick(GPU::DOTS_PER_SCANLINE as u32);
        if hblank {
            hblank_count += 1;
        }
    }

    // Should have 10 HBlank signals (one per scanline)
    assert_eq!(
        hblank_count, 10,
        "Should have one HBlank signal per scanline"
    );
}

#[test]
fn test_vblank_only_triggers_once() {
    let mut gpu = GPU::new();

    // Tick to VBlank region
    let cycles_to_vblank =
        GPU::VBLANK_START as u32 * GPU::DOTS_PER_SCANLINE as u32 - gpu.dots as u32;
    let (first_vblank, _) = gpu.tick(cycles_to_vblank + GPU::DOTS_PER_SCANLINE as u32);

    assert!(
        first_vblank,
        "VBlank interrupt should trigger when entering VBlank"
    );

    // Continue ticking within VBlank region
    let mut vblank_count = 0;
    for _ in 0..5 {
        let (vblank, _) = gpu.tick(GPU::DOTS_PER_SCANLINE as u32);
        if vblank {
            vblank_count += 1;
        }
    }

    // VBlank should not trigger again while still in VBlank region
    assert_eq!(
        vblank_count, 0,
        "VBlank should not re-trigger while in VBlank region"
    );
}

#[test]
fn test_is_in_vblank() {
    let mut gpu = GPU::new();

    // Initially not in VBlank
    assert!(!gpu.is_in_vblank());

    // Manually set scanline to VBlank region
    gpu.scanline = GPU::VBLANK_START + 5;
    gpu.in_vblank = true;

    assert!(gpu.is_in_vblank());

    // Move out of VBlank
    gpu.scanline = 50;
    gpu.in_vblank = false;

    assert!(!gpu.is_in_vblank());
}
