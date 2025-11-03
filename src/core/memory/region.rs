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

//! Memory region identification and address translation
//!
//! This module handles the PlayStation 1's memory segmentation and address translation.
//! The PSX uses MIPS memory segments with different caching behaviors.

use super::Bus;

/// Memory region identification
///
/// Used to identify which memory region an address belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegion {
    /// Main RAM (2MB)
    RAM,
    /// Scratchpad (1KB)
    Scratchpad,
    /// I/O ports
    IO,
    /// BIOS ROM
    BIOS,
    /// Cache Control registers
    CacheControl,
    /// Expansion regions (1, 2, 3) - typically unused in retail PSX
    Expansion,
    /// Unmapped region
    Unmapped,
}

impl Bus {
    /// Translate virtual address to physical address
    ///
    /// The PlayStation 1 uses MIPS memory segments:
    /// - KUSEG (0x00000000-0x7FFFFFFF): User space, cached
    /// - KSEG0 (0x80000000-0x9FFFFFFF): Kernel space, cached (mirrors physical memory)
    /// - KSEG1 (0xA0000000-0xBFFFFFFF): Kernel space, uncached (mirrors physical memory)
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to translate
    ///
    /// # Returns
    ///
    /// Physical address (with upper 3 bits masked off)
    pub(super) fn translate_address(&self, vaddr: u32) -> u32 {
        // Mask upper 3 bits to get physical address
        // This handles KUSEG, KSEG0, and KSEG1 all at once
        vaddr & 0x1FFF_FFFF
    }

    /// Identify memory region for an address
    ///
    /// Determines which memory region (RAM, Scratchpad, I/O, BIOS, or Unmapped)
    /// a given virtual address belongs to.
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address
    ///
    /// # Returns
    ///
    /// The memory region that contains this address
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::memory::{Bus, MemoryRegion};
    ///
    /// let bus = Bus::new();
    ///
    /// assert_eq!(bus.identify_region(0x00000000), MemoryRegion::RAM);
    /// assert_eq!(bus.identify_region(0x1F800000), MemoryRegion::Scratchpad);
    /// assert_eq!(bus.identify_region(0x1F801000), MemoryRegion::IO);
    /// assert_eq!(bus.identify_region(0xBFC00000), MemoryRegion::BIOS);
    /// assert_eq!(bus.identify_region(0x1FFFFFFF), MemoryRegion::Unmapped);
    /// ```
    pub fn identify_region(&self, vaddr: u32) -> MemoryRegion {
        let paddr = self.translate_address(vaddr);

        if (Self::RAM_START..=Self::RAM_END).contains(&paddr) {
            MemoryRegion::RAM
        } else if (Self::EXP1_LOW_START..=Self::EXP1_LOW_END).contains(&paddr)
            || (Self::EXP2_START..=Self::EXP2_END).contains(&paddr)
            || (Self::EXP3_START..=Self::EXP3_END).contains(&paddr)
        {
            MemoryRegion::Expansion
        } else if (Self::SCRATCHPAD_START..=Self::SCRATCHPAD_END).contains(&paddr) {
            MemoryRegion::Scratchpad
        } else if (Self::IO_START..=Self::IO_END).contains(&paddr) {
            MemoryRegion::IO
        } else if (Self::BIOS_START..=Self::BIOS_END).contains(&paddr) {
            MemoryRegion::BIOS
        } else if paddr == Self::CACHE_CONTROL {
            MemoryRegion::CacheControl
        } else {
            MemoryRegion::Unmapped
        }
    }
}
