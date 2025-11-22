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

//! GPU timing and synchronization tests
//! Tests for VBlank, HBlank, scanline counting, and timing behavior

use super::super::*;

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
