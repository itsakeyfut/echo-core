// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Basic DMA functionality tests (initialization, reset, registers)

use super::super::*;

#[test]
fn test_dma_initialization() {
    let dma = DMA::new();

    // All channels should be inactive initially
    for i in 0..7 {
        assert!(!dma.channels[i].is_active());
        assert_eq!(dma.channels[i].base_address, 0);
        assert_eq!(dma.channels[i].block_control, 0);
        assert_eq!(dma.channels[i].channel_control, 0);
    }

    // Control register should have default priority
    assert_eq!(dma.read_control(), 0x0765_4321);

    // Interrupt register should be cleared
    assert_eq!(dma.read_interrupt(), 0);
}

#[test]
fn test_dpcr_access() {
    let mut dma = DMA::new();

    // Default value
    assert_eq!(dma.read_control(), 0x0765_4321);

    // Write new value
    dma.write_control(0x1234_5678);
    assert_eq!(dma.read_control(), 0x1234_5678);
}

#[test]
fn test_dicr_access() {
    let mut dma = DMA::new();

    // Initial value
    assert_eq!(dma.read_interrupt(), 0);

    // Write configuration bits (bits 0-5 are reserved and should be preserved as 0)
    // Write without setting force flag (bit 15) to avoid triggering master flag (bit 31)
    dma.write_interrupt(0x00FF_7FC0);
    assert_eq!(dma.read_interrupt(), 0x00FF_7FC0); // Bits 6-14 and 16-23 are writable

    // Test that force flag (bit 15) causes master flag (bit 31) to be set
    dma.write_interrupt(0x0000_8000); // Set only force flag
    assert_eq!(dma.read_interrupt(), 0x8000_8000); // Master flag (bit 31) should be set

    // Clear force flag and verify master flag is cleared
    dma.write_interrupt(0x0000_0000); // Clear force flag
    assert_eq!(dma.read_interrupt(), 0x0000_0000);

    // Set up configuration with channel enables and master enable
    dma.write_interrupt(0x00FF_FFC0); // Set all config bits including force flag
    assert_eq!(dma.read_interrupt(), 0x80FF_FFC0); // Master flag (bit 31) set due to force flag

    // Test write-1-to-clear for bits 24-30 (interrupt flags)
    // Note: Since bits 6-23 are always updated, we need to re-write config to preserve it
    dma.write_interrupt(0x7FFF_FFC0); // Clear all interrupt flags and re-write config
    assert_eq!(dma.read_interrupt(), 0x80FF_FFC0); // Flags cleared, config preserved, master flag set
}
