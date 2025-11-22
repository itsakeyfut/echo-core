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

//! GPU integration tests

use super::super::*;

#[test]
fn test_gpu_register_mapping() {
    let mut system = System::new();

    // Write to GP0 (0x1F801810)
    system.bus.write32(0x1F801810, 0xA0000000).unwrap();

    // Write to GP1 (0x1F801814)
    system.bus.write32(0x1F801814, 0x03000000).unwrap();

    // Read GPUSTAT (0x1F801814)
    let status = system.bus.read32(0x1F801814).unwrap();
    // Display should be enabled (bit 23 should be 0)
    assert_eq!(status & (1 << 23), 0);
}

#[test]
fn test_gpustat_read() {
    let system = System::new();

    // Read GPU status register
    let status = system.bus.read32(0x1F801814).unwrap();

    // Status register should have valid format
    // Initially display should be disabled (bit 23 = 1)
    assert_ne!(status & (1 << 23), 0);

    // Ready flags should be set (bits 26, 27, 28)
    assert_ne!(status & (1 << 26), 0); // Ready to receive command
    assert_ne!(status & (1 << 27), 0); // Ready to send VRAM
    assert_ne!(status & (1 << 28), 0); // Ready to receive DMA
}

#[test]
fn test_gpuread() {
    let mut system = System::new();

    // Setup VRAM with test data via direct GPU access
    system.gpu.borrow_mut().write_vram(100, 100, 0x1234);
    system.gpu.borrow_mut().write_vram(101, 100, 0x5678);

    // Setup VRAM→CPU transfer via GP0
    system.bus.write32(0x1F801810, 0xC0000000).unwrap(); // Command
    system.bus.write32(0x1F801810, 0x00640064).unwrap(); // Position (100, 100)
    system.bus.write32(0x1F801810, 0x00010002).unwrap(); // Size 2×1

    // Read data via GPUREAD
    let data = system.bus.read32(0x1F801810).unwrap();
    assert_eq!(data & 0xFFFF, 0x1234);
    assert_eq!((data >> 16) & 0xFFFF, 0x5678);
}

#[test]
fn test_system_gpu_integration() {
    let mut system = System::new();

    // Run for a few cycles
    for _ in 0..100 {
        let _ = system.step();
    }

    // System should not crash
    assert!(system.cycles() >= 100);
}

#[test]
fn test_run_frame_ticks_gpu() {
    let mut system = System::new();

    // Create an infinite loop in BIOS for testing:
    // 0xBFC00000: j 0xBFC00000  (jump to self)
    // Encoding: opcode=2 (J), target=0x0F000000 (0xBFC00000 >> 2)
    // Full instruction: 0x0BF00000
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);

    // 0xBFC00004: nop (delay slot)
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();
    let initial_cycles = system.cycles();

    // Run one frame
    system.run_frame().unwrap();

    // Should execute approximately one frame worth of cycles (564,480)
    let cycles_executed = system.cycles() - initial_cycles;
    assert!(cycles_executed >= 564_480);
}

#[test]
fn test_gp0_command_via_bus() {
    let mut system = System::new();

    // Send CPU→VRAM transfer command via bus
    system.bus.write32(0x1F801810, 0xA0000000).unwrap(); // GP0 command
    system.bus.write32(0x1F801810, 0x00000000).unwrap(); // Position (0, 0)
    system.bus.write32(0x1F801810, 0x00010001).unwrap(); // Size 1×1

    // Write pixel data
    system.bus.write32(0x1F801810, 0x7FFF7FFF).unwrap();

    // Verify pixel was written to VRAM
    assert_eq!(system.gpu.borrow().read_vram(0, 0), 0x7FFF);
}

#[test]
fn test_gp1_command_via_bus() {
    let mut system = System::new();

    // Initially display should be disabled
    let status_before = system.bus.read32(0x1F801814).unwrap();
    assert_ne!(status_before & (1 << 23), 0);

    // Enable display via GP1
    system.bus.write32(0x1F801814, 0x03000000).unwrap();

    // Display should now be enabled
    let status_after = system.bus.read32(0x1F801814).unwrap();
    assert_eq!(status_after & (1 << 23), 0);
}

#[test]
fn test_gpu_reset_via_gp1() {
    let mut system = System::new();

    // Enable display
    system.bus.write32(0x1F801814, 0x03000000).unwrap();
    let status_enabled = system.bus.read32(0x1F801814).unwrap();
    assert_eq!(status_enabled & (1 << 23), 0);

    // Reset GPU via GP1(0x00)
    system.bus.write32(0x1F801814, 0x00000000).unwrap();

    // Display should be disabled again after reset
    let status_reset = system.bus.read32(0x1F801814).unwrap();
    assert_ne!(status_reset & (1 << 23), 0);
}

#[test]
fn test_vram_transfer_via_bus() {
    let mut system = System::new();

    // Start CPU→VRAM transfer
    system.bus.write32(0x1F801810, 0xA0000000).unwrap();
    system.bus.write32(0x1F801810, 0x000A000A).unwrap(); // Position (10, 10)
    system.bus.write32(0x1F801810, 0x00020002).unwrap(); // Size 2×2

    // Write 2 u32 words (4 pixels)
    system.bus.write32(0x1F801810, 0xAAAABBBB).unwrap();
    system.bus.write32(0x1F801810, 0xCCCCDDDD).unwrap();

    // Verify pixels written correctly
    assert_eq!(system.gpu.borrow().read_vram(10, 10), 0xBBBB);
    assert_eq!(system.gpu.borrow().read_vram(11, 10), 0xAAAA);
    assert_eq!(system.gpu.borrow().read_vram(10, 11), 0xDDDD);
    assert_eq!(system.gpu.borrow().read_vram(11, 11), 0xCCCC);
}

#[test]
fn test_gpu_memory_mirroring() {
    let mut system = System::new();

    // Test that GPU registers are accessible via different segments

    // Write via KUSEG
    system.bus.write32(0x1F801814, 0x03000000).unwrap();
    let status1 = system.bus.read32(0x1F801814).unwrap();

    // Read via KSEG0
    let status2 = system.bus.read32(0x9F801814).unwrap();

    // Read via KSEG1
    let status3 = system.bus.read32(0xBF801814).unwrap();

    // All should return the same value
    assert_eq!(status1, status2);
    assert_eq!(status2, status3);
}
