// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Helper functions for memory tests

use super::*;

/// Creates a new Bus instance for testing
#[allow(dead_code)]
pub fn create_test_bus() -> Bus {
    Bus::new()
}

/// Sets up a Bus with predefined BIOS values
#[allow(dead_code)]
pub fn create_bus_with_bios_data() -> Bus {
    let mut bus = Bus::new();
    // Set some known BIOS values for testing
    bus.bios[0] = 0xFF;
    bus.bios[1] = 0xFF;
    bus.bios[2] = 0xFF;
    bus.bios[3] = 0xFF;
    bus
}
