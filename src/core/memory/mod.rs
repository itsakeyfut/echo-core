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
//! use psrx::core::memory::Bus;
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

use crate::core::cdrom::CDROM;
use crate::core::dma::DMA;
use crate::core::error::{EmulatorError, Result};
use crate::core::gpu::GPU;
use crate::core::interrupt::InterruptController;
use crate::core::system::ControllerPorts;
use crate::core::timer::Timers;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

// Sub-modules
mod cache;
mod io_ports;
mod region;

// Re-export public types
pub use region::MemoryRegion;

// Tests
#[cfg(test)]
mod tests;

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

    /// ICache prefill queue
    ///
    /// When BIOS copies code to RAM (e.g., 0xBFC10000 -> 0xA0000500),
    /// we track these writes and queue them for prefilling the CPU's
    /// instruction cache. This ensures instructions are cached before
    /// RAM is zeroed by BIOS initialization.
    ///
    /// Each entry is (physical_address, instruction_word)
    icache_prefill_queue: Vec<(u32, u32)>,

    /// ICache invalidation queue
    ///
    /// When memory is written that may contain already-cached instructions
    /// (e.g., self-modifying code, runtime patching), we queue the addresses
    /// for cache invalidation to maintain coherency.
    ///
    /// Each entry is a physical_address to invalidate
    icache_invalidate_queue: Vec<u32>,

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

    /// GPU reference (shared via Rc<RefCell>)
    ///
    /// The GPU is shared between the System and Bus to allow memory-mapped
    /// register access while maintaining Rust's safety guarantees.
    gpu: Option<Rc<RefCell<GPU>>>,

    /// Controller Ports reference (shared via Rc<RefCell>)
    ///
    /// The ControllerPorts are shared between the System and Bus to allow
    /// memory-mapped register access while maintaining Rust's safety guarantees.
    controller_ports: Option<Rc<RefCell<ControllerPorts>>>,

    /// Timers reference (shared via Rc<RefCell>)
    ///
    /// The Timers are shared between the System and Bus to allow
    /// memory-mapped register access while maintaining Rust's safety guarantees.
    timers: Option<Rc<RefCell<Timers>>>,

    /// Interrupt Controller reference (shared via Rc<RefCell>)
    ///
    /// The InterruptController is shared between the System and Bus to allow
    /// memory-mapped register access while maintaining Rust's safety guarantees.
    interrupt_controller: Option<Rc<RefCell<InterruptController>>>,

    /// CD-ROM drive reference (shared via Rc<RefCell>)
    ///
    /// The CDROM is shared between the System and Bus to allow
    /// memory-mapped register access while maintaining Rust's safety guarantees.
    cdrom: Option<Rc<RefCell<CDROM>>>,

    /// DMA Controller reference (shared via Rc<RefCell>)
    ///
    /// The DMA is shared between the System and Bus to allow
    /// memory-mapped register access while maintaining Rust's safety guarantees.
    dma: Option<Rc<RefCell<DMA>>>,
}

impl Bus {
    /// RAM size (2MB)
    const RAM_SIZE: usize = 2 * 1024 * 1024;

    /// BIOS size (512KB)
    const BIOS_SIZE: usize = 512 * 1024;

    /// ICache prefill region start (0x000 - include exception vectors and low memory handlers)
    const ICACHE_PREFILL_START: usize = 0x000;

    /// ICache prefill region end (0x10000 - extended to cover all low memory code including exception vectors)
    const ICACHE_PREFILL_END: usize = 0x10000;

    /// RAM physical address range
    const RAM_START: u32 = 0x00000000;
    const RAM_END: u32 = 0x001FFFFF;

    /// Scratchpad physical address range
    /// Note: The actual scratchpad is 1KB (0x000-0x3FF), but the full 4KB region
    /// (0x000-0xFFF) is addressable. Accesses to 0x400-0xFFF mirror 0x000-0x3FF.
    const SCRATCHPAD_START: u32 = 0x1F800000;
    const SCRATCHPAD_END: u32 = 0x1F800FFF;

    /// I/O ports physical address range
    /// Note: 0x1F801000-0x1F801FFF is the main I/O area
    ///       0x1F802000-0x1F802FFF is Expansion Region 2 I/O
    ///       0x1F803000-0x1F9FFFFF is reserved/duplication but accessed by BIOS
    const IO_START: u32 = 0x1F801000;
    const IO_END: u32 = 0x1F9FFFFF;

    /// BIOS ROM physical address range
    const BIOS_START: u32 = 0x1FC00000;
    const BIOS_END: u32 = 0x1FC7FFFF;

    /// Cache Control register address
    const CACHE_CONTROL: u32 = 0x1FFE0130;

    /// Expansion Region 1 physical address range (lower part)
    /// This is the main expansion area, typically unused on retail PSX
    const EXP1_LOW_START: u32 = 0x00200000;
    const EXP1_LOW_END: u32 = 0x1EFFFFFF;

    /// Expansion Region 2 physical address range
    const EXP2_START: u32 = 0x1F000000;
    const EXP2_END: u32 = 0x1F7FFFFF;

    /// Expansion Region 3 physical address range
    const EXP3_START: u32 = 0x1FA00000;
    const EXP3_END: u32 = 0x1FBFFFFF;

    /// GPU GP0/GPUREAD register (command/data and read)
    const GPU_GP0: u32 = 0x1F801810;

    /// GPU GP1/GPUSTAT register (control and status)
    const GPU_GP1: u32 = 0x1F801814;

    /// Controller JOY_TX_DATA / JOY_RX_DATA register
    const JOY_DATA: u32 = 0x1F801040;

    /// Controller JOY_STAT register
    const JOY_STAT: u32 = 0x1F801044;

    /// Controller JOY_MODE register
    const JOY_MODE: u32 = 0x1F801048;

    /// Controller JOY_CTRL register
    const JOY_CTRL: u32 = 0x1F80104A;

    /// Controller JOY_BAUD register
    const JOY_BAUD: u32 = 0x1F80104E;

    /// Timer 0 Counter register
    const TIMER0_COUNTER: u32 = 0x1F801100;
    /// Timer 0 Mode register
    const TIMER0_MODE: u32 = 0x1F801104;
    /// Timer 0 Target register
    const TIMER0_TARGET: u32 = 0x1F801108;

    /// Timer 1 Counter register
    const TIMER1_COUNTER: u32 = 0x1F801110;
    /// Timer 1 Mode register
    const TIMER1_MODE: u32 = 0x1F801114;
    /// Timer 1 Target register
    const TIMER1_TARGET: u32 = 0x1F801118;

    /// Timer 2 Counter register
    const TIMER2_COUNTER: u32 = 0x1F801120;
    /// Timer 2 Mode register
    const TIMER2_MODE: u32 = 0x1F801124;
    /// Timer 2 Target register
    const TIMER2_TARGET: u32 = 0x1F801128;

    /// Interrupt Status register (I_STAT)
    const I_STAT: u32 = 0x1F801070;
    /// Interrupt Mask register (I_MASK)
    const I_MASK: u32 = 0x1F801074;

    /// DMA Channel registers base addresses (0x1F801080-0x1F8010EF)
    /// Each channel has 3 registers: MADR (+0x00), BCR (+0x04), CHCR (+0x08)
    const DMA_CH0_BASE: u32 = 0x1F801080; // MDEC In
    const DMA_CH1_BASE: u32 = 0x1F801090; // MDEC Out
    const DMA_CH2_BASE: u32 = 0x1F8010A0; // GPU
    const DMA_CH3_BASE: u32 = 0x1F8010B0; // CD-ROM
    const DMA_CH4_BASE: u32 = 0x1F8010C0; // SPU
    const DMA_CH5_BASE: u32 = 0x1F8010D0; // PIO
    const DMA_CH6_BASE: u32 = 0x1F8010E0; // OTC

    /// DMA Control Register (DPCR)
    const DMA_DPCR: u32 = 0x1F8010F0;
    /// DMA Interrupt Register (DICR)
    const DMA_DICR: u32 = 0x1F8010F4;

    /// CD-ROM Index/Status register (0x1F801800)
    const CDROM_INDEX: u32 = 0x1F801800;
    /// CD-ROM registers (0x1F801801-0x1F801803)
    const CDROM_REG1: u32 = 0x1F801801;
    const CDROM_REG2: u32 = 0x1F801802;
    const CDROM_REG3: u32 = 0x1F801803;

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
    /// use psrx::core::memory::Bus;
    ///
    /// let bus = Bus::new();
    /// ```
    pub fn new() -> Self {
        Self {
            ram: vec![0u8; Self::RAM_SIZE],
            icache_prefill_queue: Vec::new(),
            icache_invalidate_queue: Vec::new(),
            scratchpad: [0u8; 1024],
            bios: vec![0u8; Self::BIOS_SIZE],
            cache_control: 0,
            gpu: None,
            controller_ports: None,
            timers: None,
            interrupt_controller: None,
            cdrom: None,
            dma: None,
        }
    }

    /// Set GPU reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and GPU for handling
    /// GPU register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `gpu` - Shared reference to the GPU instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::GPU;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let gpu = Rc::new(RefCell::new(GPU::new()));
    /// bus.set_gpu(gpu.clone());
    /// ```
    pub fn set_gpu(&mut self, gpu: Rc<RefCell<GPU>>) {
        self.gpu = Some(gpu);
    }

    /// Set Controller Ports reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and ControllerPorts for handling
    /// controller register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `controller_ports` - Shared reference to ControllerPorts
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::system::ControllerPorts;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let controller_ports = Rc::new(RefCell::new(ControllerPorts::new()));
    /// bus.set_controller_ports(controller_ports.clone());
    /// ```
    pub fn set_controller_ports(&mut self, controller_ports: Rc<RefCell<ControllerPorts>>) {
        self.controller_ports = Some(controller_ports);
    }

    /// Set Timers reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and Timers for handling
    /// timer register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `timers` - Shared reference to Timers
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::timer::Timers;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let timers = Rc::new(RefCell::new(Timers::new()));
    /// bus.set_timers(timers.clone());
    /// ```
    pub fn set_timers(&mut self, timers: Rc<RefCell<Timers>>) {
        self.timers = Some(timers);
    }

    /// Set Interrupt Controller reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and InterruptController for handling
    /// interrupt register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `interrupt_controller` - Shared reference to InterruptController
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::interrupt::InterruptController;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let ic = Rc::new(RefCell::new(InterruptController::new()));
    /// bus.set_interrupt_controller(ic.clone());
    /// ```
    pub fn set_interrupt_controller(
        &mut self,
        interrupt_controller: Rc<RefCell<InterruptController>>,
    ) {
        self.interrupt_controller = Some(interrupt_controller);
    }

    /// Set CD-ROM drive reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and CDROM for handling
    /// CD-ROM register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `cdrom` - Shared reference to CDROM
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::cdrom::CDROM;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let cdrom = Rc::new(RefCell::new(CDROM::new()));
    /// bus.set_cdrom(cdrom.clone());
    /// ```
    pub fn set_cdrom(&mut self, cdrom: Rc<RefCell<CDROM>>) {
        self.cdrom = Some(cdrom);
    }

    /// Set DMA reference for memory-mapped I/O
    ///
    /// Establishes the connection between the Bus and DMA for handling
    /// DMA register accesses at memory-mapped addresses.
    ///
    /// # Arguments
    ///
    /// * `dma` - Shared reference to DMA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::dma::DMA;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let dma = Rc::new(RefCell::new(DMA::new()));
    /// bus.set_dma(dma.clone());
    /// ```
    pub fn set_dma(&mut self, dma: Rc<RefCell<DMA>>) {
        self.dma = Some(dma);
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
    /// use psrx::core::memory::Bus;
    ///
    /// let mut bus = Bus::new();
    /// bus.write32(0x80000000, 0x12345678).unwrap();
    /// bus.reset();
    /// assert_eq!(bus.read32(0x80000000).unwrap(), 0x00000000);
    /// ```
    pub fn reset(&mut self) {
        // Clear RAM (volatile memory)
        // Note: We clear to 0x00000000 (NOP). The BIOS will properly initialize
        // exception vectors and other system structures during boot.
        self.ram.fill(0);

        // Clear icache prefill queue
        self.icache_prefill_queue.clear();

        // Clear icache invalidate queue
        self.icache_invalidate_queue.clear();

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
    /// use psrx::core::memory::Bus;
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
    /// use psrx::core::memory::Bus;
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
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
                Ok(self.scratchpad[offset])
            }
            MemoryRegion::BIOS => {
                let offset = (paddr - Self::BIOS_START) as usize;
                Ok(self.bios[offset])
            }
            MemoryRegion::IO => {
                // Handle CD-ROM registers (8-bit)
                self.read_io_port8(paddr)
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
    /// use psrx::core::memory::Bus;
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
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
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
    /// use psrx::core::memory::Bus;
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
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
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
    /// use psrx::core::memory::Bus;
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
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
                self.scratchpad[offset] = value;
                Ok(())
            }
            MemoryRegion::BIOS => {
                // BIOS is read-only, ignore writes
                log::trace!("Attempt to write to BIOS at 0x{:08X} (ignored)", paddr);
                Ok(())
            }
            MemoryRegion::IO => {
                // Handle CD-ROM registers (8-bit)
                self.write_io_port8(paddr, value)
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
    /// use psrx::core::memory::Bus;
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
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
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
    /// use psrx::core::memory::Bus;
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

                // Queue for icache invalidation (all RAM writes)
                self.queue_icache_invalidation(paddr);

                // Prefill icache for BIOS code copy region
                self.queue_icache_prefill(paddr, value);

                Ok(())
            }
            MemoryRegion::Scratchpad => {
                let offset = ((paddr - Self::SCRATCHPAD_START) & 0x3FF) as usize;
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

    /// Check if any interrupt is pending
    ///
    /// Returns true if the interrupt controller has any pending unmasked interrupts.
    /// This is used by the CPU to determine if it should handle an interrupt.
    ///
    /// # Returns
    ///
    /// true if interrupts are pending, false otherwise (or if IC not initialized)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::memory::Bus;
    /// use psrx::core::interrupt::InterruptController;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let mut bus = Bus::new();
    /// let ic = Rc::new(RefCell::new(InterruptController::new()));
    /// bus.set_interrupt_controller(ic.clone());
    ///
    /// assert!(!bus.is_interrupt_pending());
    /// ```
    pub fn is_interrupt_pending(&self) -> bool {
        if let Some(interrupt_controller) = &self.interrupt_controller {
            interrupt_controller.borrow().is_pending()
        } else {
            false
        }
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
