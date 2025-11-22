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

//! Basic GPU functionality tests
//! Tests for initialization, reset, register access, and default state

use super::super::*;

#[test]
fn test_gpu_initialization() {
    let gpu = GPU::new();

    // Verify VRAM size
    assert_eq!(gpu.vram.len(), GPU::VRAM_SIZE);

    // All VRAM should be initialized to black
    assert!(gpu.vram.iter().all(|&pixel| pixel == 0x0000));
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
