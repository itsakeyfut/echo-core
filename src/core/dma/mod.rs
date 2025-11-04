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

//! DMA (Direct Memory Access) Controller
//!
//! This module implements the PlayStation's DMA controller, which provides high-speed
//! data transfers between memory and peripherals without CPU intervention.
//!
//! # DMA Channels
//!
//! The PSX has 7 DMA channels, each dedicated to a specific peripheral:
//!
//! | Channel | Device      | Base Address |
//! |---------|-------------|--------------|
//! | 0       | MDEC In     | 0x1F801080   |
//! | 1       | MDEC Out    | 0x1F801090   |
//! | 2       | GPU         | 0x1F8010A0   |
//! | 3       | CD-ROM      | 0x1F8010B0   |
//! | 4       | SPU         | 0x1F8010C0   |
//! | 5       | PIO         | 0x1F8010D0   |
//! | 6       | OTC         | 0x1F8010E0   |
//!
//! # Channel Registers
//!
//! Each channel has three 32-bit registers:
//! - **MADR** (+0x00): Memory address register
//! - **BCR** (+0x04): Block control register
//! - **CHCR** (+0x08): Channel control register
//!
//! # Global Registers
//!
//! - **DPCR** (0x1F8010F0): DMA control register (channel priorities)
//! - **DICR** (0x1F8010F4): DMA interrupt register
//!
//! # Transfer Modes
//!
//! DMA supports three synchronization modes:
//! - **Mode 0** (Immediate): Transfer entire block at once
//! - **Mode 1** (Block): Transfer in blocks with device sync
//! - **Mode 2** (Linked-list): Follow linked list in memory (GPU only)
//!
//! # References
//!
//! - [PSX-SPX: DMA Controller](http://problemkaputt.de/psx-spx.htm#dmacontroller)

use crate::core::cdrom::CDROM;
use crate::core::gpu::GPU;

#[cfg(test)]
mod tests;

/// DMA Controller with 7 channels
///
/// The DMA controller manages data transfers between memory and peripherals,
/// allowing high-speed transfers without CPU intervention.
///
/// # Examples
///
/// ```
/// use psrx::core::dma::DMA;
///
/// let mut dma = DMA::new();
/// assert_eq!(dma.read_control(), 0x07654321);
/// ```
pub struct DMA {
    /// 7 DMA channels (MDEC In/Out, GPU, CD-ROM, SPU, PIO, OTC)
    channels: [DMAChannel; 7],

    /// DMA Control Register (DPCR) at 0x1F8010F0
    ///
    /// Contains channel priority and enable bits.
    /// Default: 0x07654321 (channel priorities in order)
    control: u32,

    /// DMA Interrupt Register (DICR) at 0x1F8010F4
    ///
    /// Controls interrupt generation and flags for DMA completion.
    interrupt: u32,
}

/// Single DMA channel
///
/// Each channel manages transfers for one specific peripheral device.
#[derive(Clone)]
pub struct DMAChannel {
    /// Memory Address Register (MADR)
    ///
    /// Base address in RAM for the DMA transfer.
    base_address: u32,

    /// Block Control Register (BCR)
    ///
    /// Controls block size and count:
    /// - Bits 0-15: Block size (words)
    /// - Bits 16-31: Block count
    block_control: u32,

    /// Channel Control Register (CHCR)
    ///
    /// Controls transfer direction, sync mode, and activation:
    /// - Bit 0: Direction (0=to RAM, 1=from RAM)
    /// - Bit 1: Address step (0=forward, 1=backward)
    /// - Bit 8: Chopping enable
    /// - Bits 9-10: Sync mode (0=immediate, 1=block, 2=linked-list)
    /// - Bit 24: Start/busy flag
    /// - Bit 28: Manual trigger
    channel_control: u32,

    /// Channel ID (0-6)
    channel_id: u8,
}

impl DMAChannel {
    /// Direction: Device to RAM
    const TRANSFER_TO_RAM: u32 = 0;

    /// Direction: RAM to Device
    const TRANSFER_FROM_RAM: u32 = 1;

    /// Create a new DMA channel
    ///
    /// # Arguments
    ///
    /// * `channel_id` - Channel number (0-6)
    fn new(channel_id: u8) -> Self {
        Self {
            base_address: 0,
            block_control: 0,
            channel_control: 0,
            channel_id,
        }
    }

    /// Check if channel is active (bit 24 of CHCR)
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        (self.channel_control & 0x0100_0000) != 0
    }

    /// Get transfer direction (bit 0 of CHCR)
    ///
    /// Returns 0 for device→RAM, 1 for RAM→device
    #[inline(always)]
    pub fn direction(&self) -> u32 {
        self.channel_control & 1
    }

    /// Get synchronization mode (bits 9-10 of CHCR)
    ///
    /// - 0: Immediate (transfer all at once)
    /// - 1: Block (sync with device)
    /// - 2: Linked-list (GPU only)
    #[inline(always)]
    pub fn sync_mode(&self) -> u32 {
        (self.channel_control >> 9) & 3
    }

    /// Check if manual trigger is enabled (bit 28 of CHCR)
    #[inline(always)]
    pub fn trigger(&self) -> bool {
        (self.channel_control & 0x1000_0000) != 0
    }

    /// Deactivate the channel (clear bit 24 of CHCR)
    fn deactivate(&mut self) {
        log::trace!("DMA channel {} deactivated", self.channel_id);
        self.channel_control &= !0x0100_0000;
    }
}

impl DMA {
    /// Channel 0: MDEC In (compression input)
    #[allow(dead_code)]
    const CH_MDEC_IN: usize = 0;

    /// Channel 1: MDEC Out (decompression output)
    #[allow(dead_code)]
    const CH_MDEC_OUT: usize = 1;

    /// Channel 2: GPU (graphics)
    pub const CH_GPU: usize = 2;

    /// Channel 3: CD-ROM (disc drive)
    pub const CH_CDROM: usize = 3;

    /// Channel 4: SPU (sound)
    #[allow(dead_code)]
    const CH_SPU: usize = 4;

    /// Channel 5: PIO (expansion port)
    #[allow(dead_code)]
    const CH_PIO: usize = 5;

    /// Channel 6: OTC (ordering table clear)
    pub const CH_OTC: usize = 6;

    /// Create a new DMA controller
    ///
    /// All channels start inactive with default priority ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// use psrx::core::dma::DMA;
    ///
    /// let dma = DMA::new();
    /// ```
    pub fn new() -> Self {
        Self {
            channels: [
                DMAChannel::new(0),
                DMAChannel::new(1),
                DMAChannel::new(2),
                DMAChannel::new(3),
                DMAChannel::new(4),
                DMAChannel::new(5),
                DMAChannel::new(6),
            ],
            control: 0x0765_4321, // Default channel priority
            interrupt: 0,
        }
    }

    /// Process DMA transfers for all active channels
    ///
    /// Should be called periodically (e.g., once per scanline) to handle
    /// active DMA transfers.
    ///
    /// # Arguments
    ///
    /// * `ram` - Main system RAM
    /// * `gpu` - GPU reference for GPU transfers
    /// * `cdrom` - CD-ROM reference for CD-ROM transfers
    ///
    /// # Returns
    ///
    /// `true` if any transfer generated an interrupt
    pub fn tick(&mut self, ram: &mut [u8], gpu: &mut GPU, cdrom: &mut CDROM) -> bool {
        let mut irq = false;

        // Check each channel in priority order
        for ch_id in 0..7 {
            if self.channels[ch_id].is_active() && self.channels[ch_id].trigger() {
                irq |= self.execute_transfer(ch_id, ram, gpu, cdrom);
            }
        }

        irq
    }

    /// Execute a DMA transfer for the specified channel
    ///
    /// # Arguments
    ///
    /// * `ch_id` - Channel ID (0-6)
    /// * `ram` - Main system RAM
    /// * `gpu` - GPU reference
    /// * `cdrom` - CD-ROM reference
    ///
    /// # Returns
    ///
    /// `true` if transfer completed and generated an interrupt
    fn execute_transfer(
        &mut self,
        ch_id: usize,
        ram: &mut [u8],
        gpu: &mut GPU,
        cdrom: &mut CDROM,
    ) -> bool {
        log::debug!(
            "DMA{} transfer: addr=0x{:08X} bcr=0x{:08X} chcr=0x{:08X}",
            ch_id,
            self.channels[ch_id].base_address,
            self.channels[ch_id].block_control,
            self.channels[ch_id].channel_control
        );

        match ch_id {
            Self::CH_GPU => self.transfer_gpu(ram, gpu),
            Self::CH_CDROM => self.transfer_cdrom(ram, cdrom),
            Self::CH_OTC => self.transfer_otc(ram),
            _ => {
                log::warn!("DMA{} not implemented", ch_id);
                self.channels[ch_id].deactivate();
                false
            }
        }
    }

    /// Execute GPU DMA transfer (channel 2)
    ///
    /// Supports linked-list mode for command buffer transfers.
    fn transfer_gpu(&mut self, ram: &mut [u8], gpu: &mut GPU) -> bool {
        // Extract channel data first to avoid borrow issues
        let sync_mode = self.channels[Self::CH_GPU].sync_mode();
        let direction = self.channels[Self::CH_GPU].direction();
        let base_address = self.channels[Self::CH_GPU].base_address;
        let block_control = self.channels[Self::CH_GPU].block_control;

        match sync_mode {
            2 => {
                // Linked-list mode (GPU command lists)
                let mut addr = base_address & 0x001F_FFFC;

                loop {
                    // Read linked-list header
                    let header = self.read_ram_u32(ram, addr);
                    let count = (header >> 24) as usize;

                    // Send all words in this node to GPU
                    for i in 0..count {
                        let word = self.read_ram_u32(ram, addr + 4 + (i * 4) as u32);
                        gpu.write_gp0(word);
                    }

                    // Check for end of list marker (bit 23)
                    if (header & 0x0080_0000) != 0 {
                        break;
                    }

                    // Follow link to next node
                    addr = header & 0x001F_FFFC;
                }

                self.channels[Self::CH_GPU].deactivate();
                log::debug!("GPU DMA linked-list transfer complete");
                true
            }
            0 | 1 => {
                // Block mode for VRAM transfers
                let block_size = (block_control & 0xFFFF) as usize;
                let block_count = ((block_control >> 16) & 0xFFFF) as usize;
                let mut addr = base_address & 0x001F_FFFC;

                let total_words = if sync_mode == 0 {
                    block_size
                } else {
                    block_size * block_count
                };

                if direction == DMAChannel::TRANSFER_FROM_RAM {
                    // RAM → GPU
                    for _ in 0..total_words {
                        let word = self.read_ram_u32(ram, addr);
                        gpu.write_gp0(word);
                        addr = (addr + 4) & 0x001F_FFFC;
                    }
                } else if direction == DMAChannel::TRANSFER_TO_RAM {
                    // GPU → RAM (VRAM reads)
                    for _ in 0..total_words {
                        let word = gpu.read_gpuread();
                        self.write_ram_u32(ram, addr, word);
                        addr = (addr + 4) & 0x001F_FFFC;
                    }
                }

                self.channels[Self::CH_GPU].deactivate();
                log::debug!("GPU DMA block transfer complete ({} words)", total_words);
                true
            }
            _ => {
                log::warn!("GPU DMA sync mode {} not supported", sync_mode);
                self.channels[Self::CH_GPU].deactivate();
                false
            }
        }
    }

    /// Execute CD-ROM DMA transfer (channel 3)
    ///
    /// Transfers sector data from CD-ROM to RAM.
    fn transfer_cdrom(&mut self, ram: &mut [u8], cdrom: &mut CDROM) -> bool {
        // Extract channel data first to avoid borrow issues
        let block_control = self.channels[Self::CH_CDROM].block_control;
        let base_address = self.channels[Self::CH_CDROM].base_address;

        // CD-ROM only supports device→RAM transfers
        let block_size = (block_control & 0xFFFF) as usize;
        let block_count = ((block_control >> 16) & 0xFFFF) as usize;

        let mut addr = base_address & 0x001F_FFFC;
        let total_words = block_size * block_count;

        // Transfer data from CD-ROM buffer to RAM (word by word)
        for _ in 0..total_words {
            // Read 4 bytes (1 word) from CD-ROM
            let byte0 = cdrom.get_data_byte();
            let byte1 = cdrom.get_data_byte();
            let byte2 = cdrom.get_data_byte();
            let byte3 = cdrom.get_data_byte();

            let word = u32::from_le_bytes([byte0, byte1, byte2, byte3]);
            self.write_ram_u32(ram, addr, word);

            addr = (addr + 4) & 0x001F_FFFC;
        }

        self.channels[Self::CH_CDROM].deactivate();
        log::debug!(
            "CD-ROM DMA transfer complete ({} words = {} bytes)",
            total_words,
            total_words * 4
        );
        true
    }

    /// Execute OTC (Ordering Table Clear) transfer (channel 6)
    ///
    /// Creates a reverse-linked list in RAM for GPU command ordering.
    /// Used to set up GPU command lists for rendering.
    fn transfer_otc(&mut self, ram: &mut [u8]) -> bool {
        // Extract channel data first to avoid borrow issues
        let block_control = self.channels[Self::CH_OTC].block_control;
        let base_address = self.channels[Self::CH_OTC].base_address;

        let count = block_control & 0xFFFF;
        let mut addr = base_address & 0x001F_FFFC;

        // Write reverse-linked list
        for i in 0..count {
            if i == count - 1 {
                // Last entry: end marker
                self.write_ram_u32(ram, addr, 0x00FF_FFFF);
            } else {
                // Link to previous address (reverse order)
                self.write_ram_u32(ram, addr, (addr.wrapping_sub(4)) & 0x001F_FFFC);
            }

            addr = addr.wrapping_sub(4) & 0x001F_FFFC;
        }

        self.channels[Self::CH_OTC].deactivate();
        log::debug!("OTC DMA transfer complete ({} entries)", count);
        true
    }

    /// Read 32-bit word from RAM
    #[inline(always)]
    fn read_ram_u32(&self, ram: &[u8], addr: u32) -> u32 {
        let addr = (addr & 0x001F_FFFC) as usize;
        if addr + 4 > ram.len() {
            log::error!("DMA read out of bounds: 0x{:08X}", addr);
            return 0;
        }
        u32::from_le_bytes([ram[addr], ram[addr + 1], ram[addr + 2], ram[addr + 3]])
    }

    /// Write 32-bit word to RAM
    #[inline(always)]
    fn write_ram_u32(&self, ram: &mut [u8], addr: u32, value: u32) {
        let addr = (addr & 0x001F_FFFC) as usize;
        if addr + 4 > ram.len() {
            log::error!("DMA write out of bounds: 0x{:08X}", addr);
            return;
        }
        let bytes = value.to_le_bytes();
        ram[addr..addr + 4].copy_from_slice(&bytes);
    }

    // Register access methods

    /// Read channel MADR register
    pub fn read_madr(&self, channel: usize) -> u32 {
        self.channels[channel].base_address
    }

    /// Write channel MADR register
    pub fn write_madr(&mut self, channel: usize, value: u32) {
        self.channels[channel].base_address = value & 0x00FF_FFFF;
        log::trace!("DMA{} MADR = 0x{:08X}", channel, value);
    }

    /// Read channel BCR register
    pub fn read_bcr(&self, channel: usize) -> u32 {
        self.channels[channel].block_control
    }

    /// Write channel BCR register
    pub fn write_bcr(&mut self, channel: usize, value: u32) {
        self.channels[channel].block_control = value;
        log::trace!("DMA{} BCR = 0x{:08X}", channel, value);
    }

    /// Read channel CHCR register
    pub fn read_chcr(&self, channel: usize) -> u32 {
        self.channels[channel].channel_control
    }

    /// Write channel CHCR register
    pub fn write_chcr(&mut self, channel: usize, value: u32) {
        self.channels[channel].channel_control = value;
        log::trace!("DMA{} CHCR = 0x{:08X}", channel, value);

        // Log transfer initiation
        if (value & 0x0100_0000) != 0 {
            log::debug!(
                "DMA{} started: addr=0x{:08X} bcr=0x{:08X} mode={}",
                channel,
                self.channels[channel].base_address,
                self.channels[channel].block_control,
                self.channels[channel].sync_mode()
            );
        }
    }

    /// Read DMA Control Register (DPCR)
    pub fn read_control(&self) -> u32 {
        self.control
    }

    /// Write DMA Control Register (DPCR)
    pub fn write_control(&mut self, value: u32) {
        self.control = value;
        log::trace!("DPCR = 0x{:08X}", value);
    }

    /// Read DMA Interrupt Register (DICR)
    pub fn read_interrupt(&self) -> u32 {
        self.interrupt
    }

    /// Write DMA Interrupt Register (DICR)
    pub fn write_interrupt(&mut self, value: u32) {
        // Update writable bits (6-23) if any are set in the write value
        // Preserve reserved bits 0-5 as per PSX hardware specification
        // This allows clearing interrupt flags without changing configuration
        if (value & 0x00FF_FFC0) != 0 {
            self.interrupt = (self.interrupt & 0xFF00_003F) | (value & 0x00FF_FFC0);
        }

        // Handle write-1-to-clear for bits 24-30 (interrupt flags)
        let clear_mask = (value >> 24) & 0x7F;
        self.interrupt &= !(clear_mask << 24);

        log::trace!("DICR = 0x{:08X}", self.interrupt);
    }
}

impl Default for DMA {
    fn default() -> Self {
        Self::new()
    }
}
