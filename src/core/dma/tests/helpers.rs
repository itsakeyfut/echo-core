// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Helper functions for DMA tests

use super::super::*;

#[test]
fn test_ram_word_access() {
    let dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Test write
    dma.write_ram_u32(&mut ram, 0x1000, 0x12345678);

    // Test read
    let value = dma.read_ram_u32(&ram, 0x1000);
    assert_eq!(value, 0x12345678);

    // Verify byte order (little-endian)
    assert_eq!(ram[0x1000], 0x78);
    assert_eq!(ram[0x1001], 0x56);
    assert_eq!(ram[0x1002], 0x34);
    assert_eq!(ram[0x1003], 0x12);
}

#[test]
fn test_ram_word_access_with_masking() {
    let dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Test that address masking works correctly
    dma.write_ram_u32(&mut ram, 0xFFFF_FFFF, 0xDEADBEEF);

    // Should write to 0x001F_FFFC (masked address)
    let value = dma.read_ram_u32(&ram, 0x001F_FFFC);
    assert_eq!(value, 0xDEADBEEF);
}
