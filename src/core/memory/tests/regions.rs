// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Memory region and mirroring tests
//!
//! Tests for address translation, segment mirroring (KUSEG, KSEG0, KSEG1),
//! and memory region boundary behavior.

use super::*;

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
