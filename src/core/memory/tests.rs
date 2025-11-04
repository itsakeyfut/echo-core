// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Memory Bus Tests
//!
//! This module contains comprehensive tests for the PlayStation memory bus,
//! including RAM, scratchpad, BIOS, I/O ports, and expansion regions.
//!
//! Tests cover:
//! - Address translation and segment mirroring (KUSEG, KSEG0, KSEG1)
//! - Memory region identification
//! - Read/write operations with various data sizes (8-bit, 16-bit, 32-bit)
//! - Alignment requirements
//! - Boundary conditions
//! - Endianness verification
//! - Expansion region behavior (ROM header and open bus)

use super::*;
use crate::core::memory::MemoryRegion;

#[test]
fn test_address_translation() {
    let bus = Bus::new();

    // KUSEG
    assert_eq!(bus.translate_address(0x00001234), 0x00001234);

    // KSEG0
    assert_eq!(bus.translate_address(0x80001234), 0x00001234);

    // KSEG1
    assert_eq!(bus.translate_address(0xA0001234), 0x00001234);
}

#[test]
fn test_ram_read_write() {
    let mut bus = Bus::new();

    bus.write32(0x80000000, 0x12345678).unwrap();

    // Read from different segments (should all mirror)
    assert_eq!(bus.read32(0x00000000).unwrap(), 0x12345678);
    assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
    assert_eq!(bus.read32(0xA0000000).unwrap(), 0x12345678);
}

#[test]
fn test_bios_read_only() {
    let mut bus = Bus::new();

    // BIOS should not be writable
    bus.write32(0xBFC00000, 0xDEADBEEF).unwrap();

    // Value should remain 0 (initial state)
    assert_eq!(bus.read32(0xBFC00000).unwrap(), 0x00000000);
}

#[test]
fn test_alignment() {
    let bus = Bus::new();

    // Unaligned 32-bit read should fail
    assert!(bus.read32(0x80000001).is_err());

    // Unaligned 16-bit read should fail
    assert!(bus.read16(0x80000001).is_err());

    // 8-bit read can be unaligned
    assert!(bus.read8(0x80000001).is_ok());
}

#[test]
fn test_scratchpad_access() {
    let mut bus = Bus::new();

    bus.write32(0x1F800000, 0xABCDEF00).unwrap();
    assert_eq!(bus.read32(0x1F800000).unwrap(), 0xABCDEF00);
}

#[test]
fn test_memory_region_identification() {
    let bus = Bus::new();

    assert_eq!(bus.identify_region(0x00000000), MemoryRegion::RAM);
    assert_eq!(bus.identify_region(0x1F800000), MemoryRegion::Scratchpad);
    assert_eq!(bus.identify_region(0x1F801000), MemoryRegion::IO);
    assert_eq!(bus.identify_region(0x1FC00000), MemoryRegion::BIOS);
    assert_eq!(bus.identify_region(0x1FFFFFFF), MemoryRegion::Unmapped);
}

#[test]
fn test_endianness() {
    let mut bus = Bus::new();

    // Write individual bytes
    bus.write8(0x80000000, 0x12).unwrap();
    bus.write8(0x80000001, 0x34).unwrap();
    bus.write8(0x80000002, 0x56).unwrap();
    bus.write8(0x80000003, 0x78).unwrap();

    // Read as 32-bit (little endian)
    assert_eq!(bus.read32(0x80000000).unwrap(), 0x78563412);
}

#[test]
fn test_write8_alignment() {
    let mut bus = Bus::new();

    // 8-bit writes can be at any address
    bus.write8(0x80000000, 0xAA).unwrap();
    bus.write8(0x80000001, 0xBB).unwrap();
    bus.write8(0x80000002, 0xCC).unwrap();
    bus.write8(0x80000003, 0xDD).unwrap();

    assert_eq!(bus.read8(0x80000000).unwrap(), 0xAA);
    assert_eq!(bus.read8(0x80000001).unwrap(), 0xBB);
    assert_eq!(bus.read8(0x80000002).unwrap(), 0xCC);
    assert_eq!(bus.read8(0x80000003).unwrap(), 0xDD);
}

#[test]
fn test_write16_alignment() {
    let mut bus = Bus::new();

    // Aligned 16-bit write
    bus.write16(0x80000000, 0x1234).unwrap();
    assert_eq!(bus.read16(0x80000000).unwrap(), 0x1234);

    // Unaligned 16-bit write should fail
    assert!(bus.write16(0x80000001, 0x5678).is_err());
}

#[test]
fn test_write32_alignment() {
    let mut bus = Bus::new();

    // Aligned 32-bit write
    bus.write32(0x80000000, 0x12345678).unwrap();
    assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);

    // Unaligned 32-bit writes should fail
    assert!(bus.write32(0x80000001, 0xABCDEF00).is_err());
    assert!(bus.write32(0x80000002, 0xABCDEF00).is_err());
    assert!(bus.write32(0x80000003, 0xABCDEF00).is_err());
}

#[test]
fn test_ram_boundary() {
    let mut bus = Bus::new();

    // Test at the end of RAM
    let ram_end = 0x80000000 + (Bus::RAM_SIZE as u32) - 4;
    bus.write32(ram_end, 0xDEADBEEF).unwrap();
    assert_eq!(bus.read32(ram_end).unwrap(), 0xDEADBEEF);
}

#[test]
fn test_scratchpad_boundary() {
    let mut bus = Bus::new();

    // Test at the end of scratchpad
    let scratchpad_end = 0x1F800000 + 1024 - 4;
    bus.write32(scratchpad_end, 0xCAFEBABE).unwrap();
    assert_eq!(bus.read32(scratchpad_end).unwrap(), 0xCAFEBABE);
}

#[test]
fn test_io_port_stub() {
    let mut bus = Bus::new();

    // I/O port writes should not fail (stub implementation)
    bus.write32(0x1F801000, 0x12345678).unwrap();

    // I/O port reads should return 0 (stub implementation)
    assert_eq!(bus.read32(0x1F801000).unwrap(), 0);
}

#[test]
fn test_unmapped_access() {
    let bus = Bus::new();

    // Access to unmapped region should fail
    assert!(bus.read32(0x1FFFFFFF).is_err());
}

#[test]
fn test_mixed_size_access() {
    let mut bus = Bus::new();

    // Write 32-bit value
    bus.write32(0x80000000, 0x12345678).unwrap();

    // Read individual bytes
    assert_eq!(bus.read8(0x80000000).unwrap(), 0x78);
    assert_eq!(bus.read8(0x80000001).unwrap(), 0x56);
    assert_eq!(bus.read8(0x80000002).unwrap(), 0x34);
    assert_eq!(bus.read8(0x80000003).unwrap(), 0x12);

    // Read 16-bit values
    assert_eq!(bus.read16(0x80000000).unwrap(), 0x5678);
    assert_eq!(bus.read16(0x80000002).unwrap(), 0x1234);
}

#[test]
fn test_segment_mirroring() {
    let mut bus = Bus::new();

    // Write via KUSEG
    bus.write32(0x00001000, 0xAAAAAAAA).unwrap();

    // Read via KSEG0
    assert_eq!(bus.read32(0x80001000).unwrap(), 0xAAAAAAAA);

    // Write via KSEG1
    bus.write32(0xA0001000, 0xBBBBBBBB).unwrap();

    // Read via KUSEG
    assert_eq!(bus.read32(0x00001000).unwrap(), 0xBBBBBBBB);
}

#[test]
fn test_bios_write_ignored() {
    let mut bus = Bus::new();

    // Set initial BIOS value
    bus.bios[0] = 0xFF;
    bus.bios[1] = 0xFF;
    bus.bios[2] = 0xFF;
    bus.bios[3] = 0xFF;

    // Try to write to BIOS
    bus.write32(0xBFC00000, 0x12345678).unwrap();

    // Verify BIOS value unchanged
    assert_eq!(bus.read32(0xBFC00000).unwrap(), 0xFFFFFFFF);
}

#[test]
fn test_expansion_rom_header_read() {
    let bus = Bus::new();

    // ROM entry point at 0x1F000080 should return 0 (no ROM present)
    assert_eq!(bus.read32(0x1F000080).unwrap(), 0x00000000);

    // ROM header region (0x1F000000-0x1F0000FF) should return 0
    assert_eq!(bus.read32(0x1F000000).unwrap(), 0x00000000);
    assert_eq!(bus.read32(0x1F0000FC).unwrap(), 0x00000000);

    // Test 16-bit reads in ROM header
    assert_eq!(bus.read16(0x1F000080).unwrap(), 0x0000);
    assert_eq!(bus.read16(0x1F0000FE).unwrap(), 0x0000);

    // Test 8-bit reads in ROM header
    assert_eq!(bus.read8(0x1F000080).unwrap(), 0x00);
    assert_eq!(bus.read8(0x1F0000FF).unwrap(), 0x00);
}

#[test]
fn test_expansion_region_open_bus() {
    let bus = Bus::new();

    // Addresses outside ROM header should return open bus values

    // Expansion Region 1 (after ROM header)
    assert_eq!(bus.read32(0x1F000100).unwrap(), 0xFFFFFFFF);
    assert_eq!(bus.read32(0x1F001000).unwrap(), 0xFFFFFFFF);
    assert_eq!(bus.read32(0x1F7FFFFC).unwrap(), 0xFFFFFFFF);

    // Expansion Region 3
    assert_eq!(bus.read32(0x1FA00000).unwrap(), 0xFFFFFFFF);
    assert_eq!(bus.read32(0x1FBFFFFC).unwrap(), 0xFFFFFFFF);

    // Test 16-bit reads (open bus)
    assert_eq!(bus.read16(0x1F000100).unwrap(), 0xFFFF);
    assert_eq!(bus.read16(0x1FA00000).unwrap(), 0xFFFF);

    // Test 8-bit reads (open bus)
    assert_eq!(bus.read8(0x1F000100).unwrap(), 0xFF);
    assert_eq!(bus.read8(0x1FA00000).unwrap(), 0xFF);
}

#[test]
fn test_expansion_region_writes_ignored() {
    let mut bus = Bus::new();

    // Writes to expansion regions should succeed but be ignored

    // Write to ROM header region
    assert!(bus.write32(0x1F000080, 0x12345678).is_ok());
    // Read should still return 0 (not what we wrote)
    assert_eq!(bus.read32(0x1F000080).unwrap(), 0x00000000);

    // Write to expansion region 1
    assert!(bus.write32(0x1F001000, 0xABCDEF00).is_ok());
    // Read should return open bus value (not what we wrote)
    assert_eq!(bus.read32(0x1F001000).unwrap(), 0xFFFFFFFF);

    // Write to expansion region 3
    assert!(bus.write32(0x1FA00000, 0xDEADBEEF).is_ok());
    // Read should return open bus value (not what we wrote)
    assert_eq!(bus.read32(0x1FA00000).unwrap(), 0xFFFFFFFF);

    // Test 16-bit writes
    assert!(bus.write16(0x1F000080, 0x1234).is_ok());
    assert_eq!(bus.read16(0x1F000080).unwrap(), 0x0000);

    // Test 8-bit writes
    assert!(bus.write8(0x1F000080, 0x42).is_ok());
    assert_eq!(bus.read8(0x1F000080).unwrap(), 0x00);
}

#[test]
fn test_expansion_region_identification() {
    let bus = Bus::new();

    // Expansion region 1
    assert_eq!(bus.identify_region(0x1F000000), MemoryRegion::Expansion);
    assert_eq!(bus.identify_region(0x1F000084), MemoryRegion::Expansion);
    assert_eq!(bus.identify_region(0x1F7FFFFF), MemoryRegion::Expansion);

    // Expansion region 3
    assert_eq!(bus.identify_region(0x1FA00000), MemoryRegion::Expansion);
    assert_eq!(bus.identify_region(0x1FBFFFFF), MemoryRegion::Expansion);

    // Not expansion (verify boundaries)
    assert_ne!(bus.identify_region(0x1F800000), MemoryRegion::Expansion); // Scratchpad
    assert_ne!(bus.identify_region(0x1F801000), MemoryRegion::Expansion); // I/O
}

#[test]
fn test_expansion_rom_entry_point_boundary() {
    let bus = Bus::new();

    // Test boundary between ROM header and open bus regions

    // Last address of ROM header (should return 0)
    assert_eq!(bus.read32(0x1F0000FC).unwrap(), 0x00000000);
    assert_eq!(bus.read8(0x1F0000FF).unwrap(), 0x00);

    // First address after ROM header (should return open bus)
    assert_eq!(bus.read32(0x1F000100).unwrap(), 0xFFFFFFFF);
    assert_eq!(bus.read8(0x1F000100).unwrap(), 0xFF);

    // First address before ROM header (should return open bus)
    // Note: The range check is inclusive, so 0x1F000000 is in ROM header
    // We need to check addresses before 0x1F000000 are unmapped
    // (This is outside expansion region 1, so would be unmapped or other region)
}
