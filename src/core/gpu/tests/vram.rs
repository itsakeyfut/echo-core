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

//! VRAM operations tests
//! Tests for VRAM read, write, transfers, and addressing

use super::super::*;

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

    // Initially ready to send
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
