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

//! DMA integration tests

use super::super::*;

#[test]
fn test_dma_integration() {
    let mut system = System::new();

    // Setup a simple instruction loop in BIOS
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
    system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap(); // All channels enabled with priorities

    // Setup GPU DMA transfer (OTC mode for simplicity)
    // Use Channel 6 (OTC) which is simpler to test
    system.bus.write32(0x1F8010E0, 0x00000100).unwrap(); // MADR = 0x100
    system.bus.write32(0x1F8010E4, 0x00000010).unwrap(); // BCR = 16 entries
    system.bus.write32(0x1F8010E8, 0x11000002).unwrap(); // CHCR = start + trigger

    // Enable DMA interrupts in DICR
    system.bus.write32(0x1F8010F4, 0x00FF0000).unwrap(); // Enable all channel interrupts

    // Run a few cycles to trigger DMA
    for _ in 0..5 {
        system.step().unwrap();
    }

    // Check that DMA transfer completed
    let chcr = system.bus.read32(0x1F8010E8).unwrap();
    assert_eq!(
        chcr & 0x01000000,
        0,
        "DMA channel 6 should be inactive after transfer"
    );

    // Check that DMA created the ordering table in RAM
    // OTC writes backwards: first entry points to previous, last entry is 0x00FFFFFF
    // With MADR=0x100 and count=16, entries are at 0x100, 0xFC, 0xF8, ... 0xC4
    // First entry at 0x100 should link to 0xFC
    let first_entry = system.bus.read32(0x00000100).unwrap();
    assert_eq!(
        first_entry, 0x000000FC,
        "OTC first entry should link to previous address"
    );

    // Last entry at 0xC4 (0x100 - 15*4 = 0x100 - 0x3C) should be end marker
    let last_entry = system.bus.read32(0x000000C4).unwrap();
    assert_eq!(
        last_entry, 0x00FFFFFF,
        "OTC last entry should be end marker"
    );
}

#[test]
fn test_dma_gpu_transfer() {
    let mut system = System::new();

    // Setup a simple instruction loop
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
    system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap();

    // Setup test data in RAM for GPU transfer
    system.bus.write32(0x00001000, 0xA0000000).unwrap(); // GP0 fill command
    system.bus.write32(0x00001004, 0x00640064).unwrap(); // Position
    system.bus.write32(0x00001008, 0x00020002).unwrap(); // Size
    system.bus.write32(0x0000100C, 0x12345678).unwrap(); // Color data

    // Setup GPU DMA transfer (Channel 2, block mode)
    system.bus.write32(0x1F8010A0, 0x00001000).unwrap(); // MADR = 0x1000
    system.bus.write32(0x1F8010A4, 0x00010004).unwrap(); // BCR = 4 words, 1 block
    system.bus.write32(0x1F8010A8, 0x11000201).unwrap(); // CHCR = to GPU, sync mode 0, start, trigger

    // Run a few cycles to process DMA
    for _ in 0..10 {
        system.step().unwrap();
    }

    // Verify DMA channel is no longer active
    let chcr = system.bus.read32(0x1F8010A8).unwrap();
    assert_eq!(
        chcr & 0x01000000,
        0,
        "GPU DMA should be complete and inactive"
    );
}

#[test]
fn test_dma_interrupt() {
    use crate::core::interrupt::interrupts;

    let mut system = System::new();

    // Setup a simple instruction loop
    let jump_bytes = 0x0BF00000u32.to_le_bytes();
    system.bus_mut().write_bios_for_test(0, &jump_bytes);
    system
        .bus_mut()
        .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

    system.reset();

    // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
    system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap();

    // Enable DMA interrupts in DICR (master enable + channel 6 enable)
    system.bus.write32(0x1F8010F4, 0x00C00000).unwrap(); // Bit 23 (master) + bit 22 (ch6)

    // Enable DMA interrupt in interrupt controller
    system
        .interrupt_controller
        .borrow_mut()
        .write_mask(interrupts::DMA as u32);

    // Setup OTC DMA transfer
    system.bus.write32(0x1F8010E0, 0x00001000).unwrap(); // MADR = 0x1000
    system.bus.write32(0x1F8010E4, 0x00000008).unwrap(); // BCR = 8 entries
    system.bus.write32(0x1F8010E8, 0x11000002).unwrap(); // CHCR = start + trigger

    // Run a few cycles to trigger DMA
    for _ in 0..5 {
        system.step().unwrap();
    }

    // Verify DMA interrupt was raised
    let i_stat = system.interrupt_controller.borrow().read_status();
    assert_ne!(
        i_stat & interrupts::DMA as u32,
        0,
        "DMA interrupt should be set in I_STAT"
    );

    // Verify DICR has channel 6 flag set
    let dicr = system.bus.read32(0x1F8010F4).unwrap();
    assert_ne!(
        dicr & (1 << 30),
        0,
        "DICR should have channel 6 interrupt flag set"
    );
    assert_ne!(dicr & (1 << 31), 0, "DICR master flag should be set");
}
