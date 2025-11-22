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

//! Basic system functionality tests

use super::super::*;

#[test]
fn test_system_initialization() {
    let system = System::new();
    assert_eq!(system.cycles(), 0);
    assert_eq!(system.pc(), 0xBFC00000);
}

#[test]
fn test_system_timing_manager_created() {
    let system = System::new();
    // Verify timing manager is initialized properly
    assert_eq!(system.timing.global_tick_counter, 0);
    assert_eq!(system.timing.pending_ticks, 0);
    // With GPU events activated, downcount should be set to HBlank interval (2146 cycles)
    // which is the smallest periodic event
    assert_eq!(system.timing.downcount, 2146);
}

#[test]
fn test_system_reset() {
    let mut system = System::new();

    // Setup BIOS with NOP for testing
    system
        .bus_mut()
        .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

    // Execute some instructions to change state
    system.step().unwrap();
    system.step().unwrap();

    assert!(system.cycles() > 0);

    system.reset();
    assert_eq!(system.cycles(), 0);
    assert_eq!(system.pc(), 0xBFC00000);
    assert!(system.running);
}

#[test]
fn test_system_pc_accessor() {
    let system = System::new();
    assert_eq!(system.pc(), 0xBFC00000);
}

#[test]
fn test_system_cycles_accessor() {
    let system = System::new();
    assert_eq!(system.cycles(), 0);
}

#[test]
fn test_dma_registers_accessible() {
    let system = System::new();

    // Verify all DMA channel registers are accessible
    for ch in 0..7 {
        let base = 0x1F801080 + (ch * 0x10);

        // Read MADR (should be 0 initially)
        let madr = system.bus.read32(base).unwrap();
        assert_eq!(madr, 0, "Channel {} MADR should be 0", ch);

        // Read BCR (should be 0 initially)
        let bcr = system.bus.read32(base + 4).unwrap();
        assert_eq!(bcr, 0, "Channel {} BCR should be 0", ch);

        // Read CHCR (should be 0 initially)
        let chcr = system.bus.read32(base + 8).unwrap();
        assert_eq!(chcr, 0, "Channel {} CHCR should be 0", ch);
    }

    // Read DPCR (should have default priority)
    let dpcr = system.bus.read32(0x1F8010F0).unwrap();
    assert_eq!(dpcr, 0x07654321, "DPCR should have default priority");

    // Read DICR (should be 0 initially)
    let dicr = system.bus.read32(0x1F8010F4).unwrap();
    assert_eq!(dicr, 0, "DICR should be 0 initially");
}
