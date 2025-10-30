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

//! Memory bus implementation for PlayStation 1 emulator
//!
//! The Bus is the central component for all memory operations in the emulator.
//! It manages address translation, memory mapping, and routing of read/write
//! operations to appropriate memory regions.
//!
//! # Memory Map
//!
//! | Physical Address Range | Region       | Size   | Access |
//! |------------------------|--------------|--------|--------|
//! | 0x00000000-0x001FFFFF  | RAM          | 2MB    | R/W    |
//! | 0x1F800000-0x1F8003FF  | Scratchpad   | 1KB    | R/W    |
//! | 0x1F801000-0x1F802FFF  | I/O Ports    | 8KB    | R/W    |
//! | 0x1FC00000-0x1FC7FFFF  | BIOS ROM     | 512KB  | R only |
//!
//! # Address Translation
//!
//! The PlayStation 1 uses MIPS memory segments:
//! - KUSEG (0x00000000-0x7FFFFFFF): User space, cached
//! - KSEG0 (0x80000000-0x9FFFFFFF): Kernel space, cached (mirrors physical memory)
//! - KSEG1 (0xA0000000-0xBFFFFFFF): Kernel space, uncached (mirrors physical memory)
//!
//! # Example
//!
//! ```
//! use echo_core::core::memory::Bus;
//!
//! let mut bus = Bus::new();
//!
//! // Write to RAM via KSEG0
//! bus.write32(0x80000000, 0x12345678).unwrap();
//!
//! // Read from same location via different segment (should mirror)
//! assert_eq!(bus.read32(0x00000000).unwrap(), 0x12345678);
//! assert_eq!(bus.read32(0xA0000000).unwrap(), 0x12345678);
//! ```

use crate::core::error::{EmulatorError, Result};
use std::fs::File;
use std::io::Read;

/// Memory bus managing all memory accesses
///
/// The Bus handles all memory operations including RAM, scratchpad,
/// BIOS ROM, and I/O ports. It performs address translation and
/// ensures proper alignment for memory accesses.
pub struct Bus {
    /// Main RAM (2MB)
    ///
    /// Physical address: 0x00000000-0x001FFFFF
    ram: Vec<u8>,

    /// Scratchpad (1KB fast RAM)
    ///
    /// Physical address: 0x1F800000-0x1F8003FF
    /// This is a small, fast RAM area used for time-critical data
    scratchpad: [u8; 1024],

    /// BIOS ROM (512KB)
    ///
    /// Physical address: 0x1FC00000-0x1FC7FFFF
    /// Contains the PlayStation BIOS code
    bios: Vec<u8>,

    /// Cache Control register
    ///
    /// Physical address: 0x1FFE0130 (accessed via 0xFFFE0130)
    /// Controls instruction cache, data cache, and scratchpad enable
    cache_control: u32,
}

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
    /// RAM size (2MB)
    const RAM_SIZE: usize = 2 * 1024 * 1024;

    /// BIOS size (512KB)
    const BIOS_SIZE: usize = 512 * 1024;

    /// RAM physical address range
    const RAM_START: u32 = 0x00000000;
    const RAM_END: u32 = 0x001FFFFF;

    /// Scratchpad physical address range
    const SCRATCHPAD_START: u32 = 0x1F800000;
    const SCRATCHPAD_END: u32 = 0x1F8003FF;

    /// I/O ports physical address range
    const IO_START: u32 = 0x1F801000;
    const IO_END: u32 = 0x1F802FFF;

    /// BIOS ROM physical address range
    const BIOS_START: u32 = 0x1FC00000;
    const BIOS_END: u32 = 0x1FC7FFFF;

    /// Cache Control register address
    const CACHE_CONTROL: u32 = 0x1FFE0130;

    /// Expansion Region 1 physical address range
    const EXP1_START: u32 = 0x1F000000;
    const EXP1_END: u32 = 0x1F7FFFFF;

    /// Expansion Region 3 physical address range
    const EXP3_START: u32 = 0x1FA00000;
    const EXP3_END: u32 = 0x1FBFFFFF;

    /// Create a new Bus instance
    ///
    /// Initializes all memory regions with zeros.
    ///
    /// # Returns
    ///
    /// A new Bus instance with:
    /// - 2MB of RAM initialized to 0
    /// - 1KB of scratchpad initialized to 0
    /// - 512KB of BIOS initialized to 0
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let bus = Bus::new();
    /// ```
    pub fn new() -> Self {
        Self {
            ram: vec![0u8; Self::RAM_SIZE],
            scratchpad: [0u8; 1024],
            bios: vec![0u8; Self::BIOS_SIZE],
            cache_control: 0,
        }
    }

    /// Reset the bus to initial state
    ///
    /// Clears RAM and scratchpad to zero, simulating a power-cycle.
    /// BIOS contents are preserved as they represent read-only ROM.
    ///
    /// This ensures that system reset properly clears volatile memory
    /// while maintaining the loaded BIOS image.
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write32(0x80000000, 0x12345678).unwrap();
    /// bus.reset();
    /// assert_eq!(bus.read32(0x80000000).unwrap(), 0x00000000);
    /// ```
    pub fn reset(&mut self) {
        // Clear RAM (volatile memory)
        self.ram.fill(0);
        // Clear scratchpad (volatile memory)
        self.scratchpad.fill(0);
        // Reset cache control to default
        self.cache_control = 0;
        // BIOS is read-only ROM, so it is not cleared
    }

    /// Load BIOS from file
    ///
    /// Loads a BIOS ROM file into the BIOS region. The file must be
    /// exactly 512KB in size.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the BIOS file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if BIOS was loaded successfully
    /// - `Err(EmulatorError)` if file operations fail or size is incorrect
    ///
    /// # Errors
    ///
    /// Returns `EmulatorError::BiosError` if:
    /// - File cannot be opened
    /// - File size is not 512KB
    /// - File cannot be read
    ///
    /// # Example
    ///
    /// ```no_run
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.load_bios("SCPH1001.BIN").unwrap();
    /// ```
    pub fn load_bios(&mut self, path: &str) -> Result<()> {
        let mut file =
            File::open(path).map_err(|_| EmulatorError::BiosNotFound(path.to_string()))?;

        let metadata = file.metadata()?;

        if metadata.len() != Self::BIOS_SIZE as u64 {
            return Err(EmulatorError::InvalidBiosSize {
                expected: Self::BIOS_SIZE,
                got: metadata.len() as usize,
            });
        }

        file.read_exact(&mut self.bios)?;

        Ok(())
    }

    /// Translate virtual address to physical address
    ///
    /// PlayStation 1 uses MIPS memory segments that mirror physical memory:
    /// - KUSEG (0x00000000-0x7FFFFFFF): Direct mapping
    /// - KSEG0 (0x80000000-0x9FFFFFFF): Cached, mirrors physical 0x00000000-0x1FFFFFFF
    /// - KSEG1 (0xA0000000-0xBFFFFFFF): Uncached, mirrors physical 0x00000000-0x1FFFFFFF
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address
    ///
    /// # Returns
    ///
    /// Physical address after translation
    ///
    /// # Implementation
    ///
    /// All segments map to the same 512MB physical address space:
    /// - 0x00001234 (KUSEG) → 0x00001234
    /// - 0x80001234 (KSEG0) → 0x00001234
    /// - 0xA0001234 (KSEG1) → 0x00001234
    #[inline(always)]
    fn translate_address(&self, vaddr: u32) -> u32 {
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
    /// use echo_core::core::memory::{Bus, MemoryRegion};
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
        } else if (Self::EXP1_START..=Self::EXP1_END).contains(&paddr) {
            MemoryRegion::Expansion
        } else if (Self::SCRATCHPAD_START..=Self::SCRATCHPAD_END).contains(&paddr) {
            MemoryRegion::Scratchpad
        } else if (Self::IO_START..=Self::IO_END).contains(&paddr) {
            MemoryRegion::IO
        } else if (Self::EXP3_START..=Self::EXP3_END).contains(&paddr) {
            MemoryRegion::Expansion
        } else if (Self::BIOS_START..=Self::BIOS_END).contains(&paddr) {
            MemoryRegion::BIOS
        } else if paddr == Self::CACHE_CONTROL {
            MemoryRegion::CacheControl
        } else {
            MemoryRegion::Unmapped
        }
    }

    /// Read 8-bit value from memory
    ///
    /// Reads a single byte from the specified virtual address.
    /// 8-bit reads do not require alignment.
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to read from
    ///
    /// # Returns
    ///
    /// - `Ok(u8)` containing the byte value
    /// - `Err(EmulatorError)` if the address is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write8(0x80000000, 0x42).unwrap();
    /// assert_eq!(bus.read8(0x80000000).unwrap(), 0x42);
    /// ```
    pub fn read8(&self, vaddr: u32) -> Result<u8> {
        let paddr = self.translate_address(vaddr);

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                Ok(self.ram[offset])
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                Ok(self.scratchpad[offset])
            }
            MemoryRegion::BIOS => {
                let offset = (paddr - Self::BIOS_START) as usize;
                Ok(self.bios[offset])
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                log::trace!("I/O port read8 at 0x{:08X}", paddr);
                Ok(0)
            }
            MemoryRegion::CacheControl => {
                // Cache control is 32-bit only, stub 8-bit reads
                log::debug!("Cache control read8 at 0x{:08X} (stubbed)", vaddr);
                Ok(0)
            }
            MemoryRegion::Expansion => {
                // Expansion regions: return 0 for ROM header, 0xFF otherwise
                let paddr = self.translate_address(vaddr);
                if (0x1F000000..=0x1F0000FF).contains(&paddr) {
                    log::trace!("Expansion ROM header read8 at 0x{:08X} -> 0x00", vaddr);
                    Ok(0x00)
                } else {
                    log::trace!("Expansion region read8 at 0x{:08X} -> 0xFF", vaddr);
                    Ok(0xFF)
                }
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Read 16-bit value from memory
    ///
    /// Reads a 16-bit value (little-endian) from the specified virtual address.
    /// The address must be 2-byte aligned (address & 0x1 == 0).
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to read from (must be 2-byte aligned)
    ///
    /// # Returns
    ///
    /// - `Ok(u16)` containing the value
    /// - `Err(EmulatorError::UnalignedAccess)` if address is not 2-byte aligned
    /// - `Err(EmulatorError::InvalidAddress)` if address is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write16(0x80000000, 0x1234).unwrap();
    /// assert_eq!(bus.read16(0x80000000).unwrap(), 0x1234);
    ///
    /// // Unaligned access fails
    /// assert!(bus.read16(0x80000001).is_err());
    /// ```
    pub fn read16(&self, vaddr: u32) -> Result<u16> {
        // Check alignment
        if vaddr & 0x1 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 2,
            });
        }

        let paddr = self.translate_address(vaddr);

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                let bytes = [self.ram[offset], self.ram[offset + 1]];
                Ok(u16::from_le_bytes(bytes))
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                let bytes = [self.scratchpad[offset], self.scratchpad[offset + 1]];
                Ok(u16::from_le_bytes(bytes))
            }
            MemoryRegion::BIOS => {
                let offset = (paddr - Self::BIOS_START) as usize;
                let bytes = [self.bios[offset], self.bios[offset + 1]];
                Ok(u16::from_le_bytes(bytes))
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                log::trace!("I/O port read16 at 0x{:08X}", paddr);
                Ok(0)
            }
            MemoryRegion::CacheControl => {
                // Cache control is 32-bit only, stub 16-bit reads
                log::debug!("Cache control read16 at 0x{:08X} (stubbed)", vaddr);
                Ok(0)
            }
            MemoryRegion::Expansion => {
                // Expansion regions: return 0 for ROM header, 0xFFFF otherwise
                let paddr = self.translate_address(vaddr);
                if (0x1F000000..=0x1F0000FF).contains(&paddr) {
                    log::trace!("Expansion ROM header read16 at 0x{:08X} -> 0x0000", vaddr);
                    Ok(0x0000)
                } else {
                    log::trace!("Expansion region read16 at 0x{:08X} -> 0xFFFF", vaddr);
                    Ok(0xFFFF)
                }
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Read 32-bit value from memory
    ///
    /// Reads a 32-bit value (little-endian) from the specified virtual address.
    /// The address must be 4-byte aligned (address & 0x3 == 0).
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to read from (must be 4-byte aligned)
    ///
    /// # Returns
    ///
    /// - `Ok(u32)` containing the value
    /// - `Err(EmulatorError::UnalignedAccess)` if address is not 4-byte aligned
    /// - `Err(EmulatorError::InvalidAddress)` if address is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write32(0x80000000, 0x12345678).unwrap();
    /// assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
    ///
    /// // Unaligned access fails
    /// assert!(bus.read32(0x80000001).is_err());
    /// ```
    pub fn read32(&self, vaddr: u32) -> Result<u32> {
        // Check alignment
        if vaddr & 0x3 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 4,
            });
        }

        let paddr = self.translate_address(vaddr);

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                let bytes = [
                    self.ram[offset],
                    self.ram[offset + 1],
                    self.ram[offset + 2],
                    self.ram[offset + 3],
                ];
                Ok(u32::from_le_bytes(bytes))
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                let bytes = [
                    self.scratchpad[offset],
                    self.scratchpad[offset + 1],
                    self.scratchpad[offset + 2],
                    self.scratchpad[offset + 3],
                ];
                Ok(u32::from_le_bytes(bytes))
            }
            MemoryRegion::BIOS => {
                let offset = (paddr - Self::BIOS_START) as usize;
                let bytes = [
                    self.bios[offset],
                    self.bios[offset + 1],
                    self.bios[offset + 2],
                    self.bios[offset + 3],
                ];
                Ok(u32::from_le_bytes(bytes))
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                self.read_io_port32(paddr)
            }
            MemoryRegion::CacheControl => {
                // Cache control register (FFFE0130h)
                log::debug!(
                    "Cache control read at 0x{:08X}, returning 0x{:08X}",
                    vaddr,
                    self.cache_control
                );
                Ok(self.cache_control)
            }
            MemoryRegion::Expansion => {
                // Expansion regions: check for special addresses
                let paddr = self.translate_address(vaddr);

                // Expansion ROM entry points should return 0 (no ROM)
                // BIOS checks these addresses and tries to call them as function pointers
                // Returning 0 prevents invalid jumps to 0xFFFFFFFF
                if (0x1F000000..=0x1F0000FF).contains(&paddr) {
                    log::trace!(
                        "Expansion ROM header read32 at 0x{:08X} -> 0x00000000 (no ROM)",
                        vaddr
                    );
                    Ok(0x00000000)
                } else {
                    // Other expansion region addresses return 0xFFFFFFFF
                    log::trace!("Expansion region read32 at 0x{:08X} -> 0xFFFFFFFF", vaddr);
                    Ok(0xFFFFFFFF)
                }
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Write 8-bit value to memory
    ///
    /// Writes a single byte to the specified virtual address.
    /// 8-bit writes do not require alignment.
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to write to
    /// * `value` - Byte value to write
    ///
    /// # Returns
    ///
    /// - `Ok(())` if write was successful
    /// - `Err(EmulatorError)` if the address is invalid or read-only
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write8(0x80000000, 0x42).unwrap();
    /// assert_eq!(bus.read8(0x80000000).unwrap(), 0x42);
    /// ```
    pub fn write8(&mut self, vaddr: u32, value: u8) -> Result<()> {
        let paddr = self.translate_address(vaddr);

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                self.ram[offset] = value;
                Ok(())
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                self.scratchpad[offset] = value;
                Ok(())
            }
            MemoryRegion::BIOS => {
                // BIOS is read-only, ignore writes
                log::trace!("Attempt to write to BIOS at 0x{:08X} (ignored)", paddr);
                Ok(())
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                log::trace!("I/O port write8 at 0x{:08X} = 0x{:02X}", paddr, value);
                Ok(())
            }
            MemoryRegion::CacheControl => {
                // Cache control is 32-bit only, ignore 8-bit writes
                log::debug!(
                    "Cache control write8 at 0x{:08X} = 0x{:02X} (ignored)",
                    vaddr,
                    value
                );
                Ok(())
            }
            MemoryRegion::Expansion => {
                // Expansion regions: ignore writes (no hardware present)
                log::trace!(
                    "Expansion region write8 at 0x{:08X} = 0x{:02X} (ignored)",
                    vaddr,
                    value
                );
                Ok(())
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Write 16-bit value to memory
    ///
    /// Writes a 16-bit value (little-endian) to the specified virtual address.
    /// The address must be 2-byte aligned (address & 0x1 == 0).
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to write to (must be 2-byte aligned)
    /// * `value` - 16-bit value to write
    ///
    /// # Returns
    ///
    /// - `Ok(())` if write was successful
    /// - `Err(EmulatorError::UnalignedAccess)` if address is not 2-byte aligned
    /// - `Err(EmulatorError::InvalidAddress)` if address is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write16(0x80000000, 0x1234).unwrap();
    /// assert_eq!(bus.read16(0x80000000).unwrap(), 0x1234);
    ///
    /// // Unaligned access fails
    /// assert!(bus.write16(0x80000001, 0x1234).is_err());
    /// ```
    pub fn write16(&mut self, vaddr: u32, value: u16) -> Result<()> {
        // Check alignment
        if vaddr & 0x1 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 2,
            });
        }

        let paddr = self.translate_address(vaddr);
        let bytes = value.to_le_bytes();

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                self.ram[offset] = bytes[0];
                self.ram[offset + 1] = bytes[1];
                Ok(())
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                self.scratchpad[offset] = bytes[0];
                self.scratchpad[offset + 1] = bytes[1];
                Ok(())
            }
            MemoryRegion::BIOS => {
                // BIOS is read-only, ignore writes
                log::trace!("Attempt to write to BIOS at 0x{:08X} (ignored)", paddr);
                Ok(())
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                log::trace!("I/O port write16 at 0x{:08X} = 0x{:04X}", paddr, value);
                Ok(())
            }
            MemoryRegion::CacheControl => {
                // Cache control is 32-bit only, ignore 16-bit writes
                log::debug!(
                    "Cache control write16 at 0x{:08X} = 0x{:04X} (ignored)",
                    vaddr,
                    value
                );
                Ok(())
            }
            MemoryRegion::Expansion => {
                // Expansion regions: ignore writes (no hardware present)
                log::trace!(
                    "Expansion region write16 at 0x{:08X} = 0x{:04X} (ignored)",
                    vaddr,
                    value
                );
                Ok(())
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Write 32-bit value to memory
    ///
    /// Writes a 32-bit value (little-endian) to the specified virtual address.
    /// The address must be 4-byte aligned (address & 0x3 == 0).
    ///
    /// # Arguments
    ///
    /// * `vaddr` - Virtual address to write to (must be 4-byte aligned)
    /// * `value` - 32-bit value to write
    ///
    /// # Returns
    ///
    /// - `Ok(())` if write was successful
    /// - `Err(EmulatorError::UnalignedAccess)` if address is not 4-byte aligned
    /// - `Err(EmulatorError::InvalidAddress)` if address is invalid
    ///
    /// # Example
    ///
    /// ```
    /// use echo_core::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write32(0x80000000, 0x12345678).unwrap();
    /// assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
    ///
    /// // Unaligned access fails
    /// assert!(bus.write32(0x80000001, 0x12345678).is_err());
    /// ```
    pub fn write32(&mut self, vaddr: u32, value: u32) -> Result<()> {
        // Check alignment
        if vaddr & 0x3 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 4,
            });
        }

        let paddr = self.translate_address(vaddr);
        let bytes = value.to_le_bytes();

        match self.identify_region(vaddr) {
            MemoryRegion::RAM => {
                let offset = paddr as usize;
                self.ram[offset] = bytes[0];
                self.ram[offset + 1] = bytes[1];
                self.ram[offset + 2] = bytes[2];
                self.ram[offset + 3] = bytes[3];
                Ok(())
            }
            MemoryRegion::Scratchpad => {
                let offset = (paddr - Self::SCRATCHPAD_START) as usize;
                self.scratchpad[offset] = bytes[0];
                self.scratchpad[offset + 1] = bytes[1];
                self.scratchpad[offset + 2] = bytes[2];
                self.scratchpad[offset + 3] = bytes[3];
                Ok(())
            }
            MemoryRegion::BIOS => {
                // BIOS is read-only, ignore writes
                log::trace!("Attempt to write to BIOS at 0x{:08X} (ignored)", paddr);
                Ok(())
            }
            MemoryRegion::IO => {
                // I/O port stub for Phase 1 Week 1
                self.write_io_port32(paddr, value)
            }
            MemoryRegion::CacheControl => {
                // Cache control register (FFFE0130h)
                log::debug!(
                    "Cache control write at 0x{:08X}, value 0x{:08X}",
                    vaddr,
                    value
                );
                self.cache_control = value;
                Ok(())
            }
            MemoryRegion::Expansion => {
                // Expansion regions: ignore writes (no hardware present)
                log::trace!(
                    "Expansion region write32 at 0x{:08X} = 0x{:08X} (ignored)",
                    vaddr,
                    value
                );
                Ok(())
            }
            MemoryRegion::Unmapped => Err(EmulatorError::InvalidMemoryAccess { address: vaddr }),
        }
    }

    /// Read from I/O port (32-bit) - stub implementation
    ///
    /// This is a placeholder implementation for Phase 1 Week 1.
    /// Actual I/O port handling will be implemented in later phases.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    ///
    /// # Returns
    ///
    /// Always returns `Ok(0)` for now
    fn read_io_port32(&self, paddr: u32) -> Result<u32> {
        log::trace!("I/O port read at 0x{:08X}", paddr);
        Ok(0) // Stub implementation
    }

    /// Write to I/O port (32-bit) - stub implementation
    ///
    /// This is a placeholder implementation for Phase 1 Week 1.
    /// Actual I/O port handling will be implemented in later phases.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address of I/O port
    /// * `value` - Value to write
    ///
    /// # Returns
    ///
    /// Always returns `Ok(())` for now
    fn write_io_port32(&mut self, paddr: u32, value: u32) -> Result<()> {
        log::trace!("I/O port write at 0x{:08X} = 0x{:08X}", paddr, value);
        Ok(()) // Stub implementation
    }

    /// Write directly to BIOS memory (test helper)
    ///
    /// This method bypasses the read-only protection of BIOS and allows
    /// direct writes for testing purposes only.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset into BIOS (0-512KB)
    /// * `data` - Data to write
    ///
    /// # Panics
    ///
    /// Panics if offset + data.len() exceeds BIOS size
    #[cfg(test)]
    pub(crate) fn write_bios_for_test(&mut self, offset: usize, data: &[u8]) {
        let end = offset + data.len();
        assert!(
            end <= Self::BIOS_SIZE,
            "BIOS write out of bounds: offset={}, len={}",
            offset,
            data.len()
        );
        self.bios[offset..end].copy_from_slice(data);
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
}
