// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Basic memory bus tests
//!
//! Tests for basic memory functionality including initialization and
//! memory region identification.

use super::*;

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
fn test_unmapped_access() {
    let bus = Bus::new();

    // Access to unmapped region should fail
    assert!(bus.read32(0x1FFFFFFF).is_err());
}
