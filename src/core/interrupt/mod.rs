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

//! PSX Interrupt Controller Implementation
//!
//! The interrupt controller manages interrupt requests from all PSX hardware components
//! and signals the CPU when interrupts should be handled.
//!
//! ## Registers
//!
//! - **I_STAT** (0x1F801070): Interrupt status register (R/W)
//!   - Reading returns current interrupt flags
//!   - Writing 0 to a bit acknowledges that interrupt (clears the bit)
//!   - Writing 1 to a bit has no effect
//!
//! - **I_MASK** (0x1F801074): Interrupt mask register (R/W)
//!   - Controls which interrupts can reach the CPU
//!   - 1 = interrupt enabled, 0 = interrupt masked
//!
//! ## Interrupt Sources (Bit Positions)
//!
//! ```text
//! Bit  | Source        | Description
//! -----|---------------|----------------------------------
//! 0    | VBLANK        | Vertical blank interrupt
//! 1    | GPU           | GPU command/transfer complete
//! 2    | CDROM         | CD-ROM controller
//! 3    | DMA           | DMA transfer complete
//! 4    | TIMER0        | Timer 0 interrupt
//! 5    | TIMER1        | Timer 1 interrupt
//! 6    | TIMER2        | Timer 2 interrupt
//! 7    | CONTROLLER    | Controller/memory card
//! 8    | SIO           | Serial I/O
//! 9    | SPU           | Sound processing unit
//! 10   | LIGHTPEN      | Lightpen/IRQ10 (PIO)
//! 11-15| -             | Not used
//! ```
//!
//! ## References
//!
//! - [PSX-SPX: Interrupt Control](http://problemkaputt.de/psx-spx.htm#interruptcontrol)

/// Interrupt source bit flags
///
/// These constants represent the bit positions in I_STAT and I_MASK registers
/// for each interrupt source.
pub mod interrupts {
    /// Vertical blank interrupt (bit 0)
    pub const VBLANK: u16 = 1 << 0;

    /// GPU command/transfer complete interrupt (bit 1)
    pub const GPU: u16 = 1 << 1;

    /// CD-ROM controller interrupt (bit 2)
    pub const CDROM: u16 = 1 << 2;

    /// DMA transfer complete interrupt (bit 3)
    pub const DMA: u16 = 1 << 3;

    /// Timer 0 interrupt (bit 4)
    pub const TIMER0: u16 = 1 << 4;

    /// Timer 1 interrupt (bit 5)
    pub const TIMER1: u16 = 1 << 5;

    /// Timer 2 interrupt (bit 6)
    pub const TIMER2: u16 = 1 << 6;

    /// Controller/memory card interrupt (bit 7)
    pub const CONTROLLER: u16 = 1 << 7;

    /// Serial I/O interrupt (bit 8)
    pub const SIO: u16 = 1 << 8;

    /// Sound processing unit interrupt (bit 9)
    pub const SPU: u16 = 1 << 9;

    /// Lightpen/IRQ10 (PIO) interrupt (bit 10)
    pub const LIGHTPEN: u16 = 1 << 10;
}

/// PlayStation Interrupt Controller
///
/// Manages interrupt requests from all hardware components and determines
/// which interrupts reach the CPU based on the mask register.
///
/// # Example
///
/// ```
/// use psrx::core::interrupt::{InterruptController, interrupts};
///
/// let mut ic = InterruptController::new();
///
/// // Request VBLANK interrupt
/// ic.request(interrupts::VBLANK);
///
/// // Enable VBLANK interrupts
/// ic.write_mask(interrupts::VBLANK as u32);
///
/// // Check if any interrupt is pending
/// assert!(ic.is_pending());
///
/// // Acknowledge the interrupt (write 0 to clear)
/// ic.write_status(!interrupts::VBLANK as u32);
/// assert!(!ic.is_pending());
/// ```
pub struct InterruptController {
    /// I_STAT (0x1F801070) - Interrupt status register
    ///
    /// Each bit represents a pending interrupt from a specific source.
    /// Writing 0 to a bit acknowledges (clears) that interrupt.
    /// Writing 1 to a bit leaves it unchanged.
    status: u16,

    /// I_MASK (0x1F801074) - Interrupt mask register
    ///
    /// Each bit controls whether the corresponding interrupt can reach the CPU.
    /// 1 = interrupt enabled, 0 = interrupt masked (blocked).
    mask: u16,
}

impl InterruptController {
    /// Create a new interrupt controller
    ///
    /// Initializes with all interrupts cleared and masked.
    ///
    /// # Returns
    ///
    /// A new InterruptController instance
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::InterruptController;
    ///
    /// let ic = InterruptController::new();
    /// assert_eq!(ic.read_status(), 0);
    /// assert_eq!(ic.read_mask(), 0);
    /// ```
    pub fn new() -> Self {
        Self { status: 0, mask: 0 }
    }

    /// Request an interrupt
    ///
    /// Sets the specified interrupt bit(s) in the status register.
    /// This is called by hardware components when they need to signal the CPU.
    ///
    /// # Arguments
    ///
    /// * `interrupt` - Interrupt bit(s) to set (can be multiple ORed together)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.request(interrupts::VBLANK);
    /// assert_eq!(ic.read_status(), interrupts::VBLANK as u32);
    /// ```
    pub fn request(&mut self, interrupt: u16) {
        self.status |= interrupt;
        log::trace!(
            "IRQ requested: 0x{:04X}, status=0x{:04X}",
            interrupt,
            self.status
        );
    }

    /// Acknowledge interrupt (write 0 to clear bits)
    ///
    /// Clears interrupt bits where the corresponding bit in `value` is 0.
    /// This implements the PSX acknowledge mechanism where you write 0
    /// to the bits you want to clear (bits set to 1 are unchanged).
    ///
    /// # Arguments
    ///
    /// * `value` - Acknowledge mask (0 bits will clear corresponding interrupts)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.request(interrupts::VBLANK | interrupts::TIMER0);
    ///
    /// // Acknowledge VBLANK (write 0 to that bit, 1 to others)
    /// ic.acknowledge(!interrupts::VBLANK);
    /// assert_eq!(ic.read_status(), interrupts::TIMER0 as u32);
    /// ```
    pub fn acknowledge(&mut self, value: u16) {
        self.status &= value;
        log::trace!("IRQ acknowledged, status=0x{:04X}", self.status);
    }

    /// Check if any interrupt is pending for CPU
    ///
    /// Returns true if any unmasked interrupt is currently active.
    /// This is used by the CPU to determine if it should handle an interrupt.
    ///
    /// # Returns
    ///
    /// true if (status & mask) != 0, false otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    ///
    /// // Request interrupt but it's masked
    /// ic.request(interrupts::VBLANK);
    /// assert!(!ic.is_pending());
    ///
    /// // Unmask the interrupt
    /// ic.write_mask(interrupts::VBLANK as u32);
    /// assert!(ic.is_pending());
    /// ```
    pub fn is_pending(&self) -> bool {
        (self.status & self.mask) != 0
    }

    /// Read I_STAT register
    ///
    /// Returns the current interrupt status register value.
    ///
    /// # Returns
    ///
    /// Current status register value (extended to u32)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.request(interrupts::TIMER0);
    /// assert_eq!(ic.read_status(), interrupts::TIMER0 as u32);
    /// ```
    pub fn read_status(&self) -> u32 {
        self.status as u32
    }

    /// Write I_STAT register (acknowledge)
    ///
    /// Acknowledges interrupts by writing 0 to clear the corresponding bits.
    /// Only the lower 16 bits are used.
    ///
    /// # Arguments
    ///
    /// * `value` - Value to write (lower 16 bits used, 0 bits clear interrupts)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.request(interrupts::VBLANK);
    /// ic.write_status(!interrupts::VBLANK as u32);
    /// assert_eq!(ic.read_status(), 0);
    /// ```
    pub fn write_status(&mut self, value: u32) {
        self.acknowledge(value as u16);
    }

    /// Read I_MASK register
    ///
    /// Returns the current interrupt mask register value.
    ///
    /// # Returns
    ///
    /// Current mask register value (extended to u32)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.write_mask(interrupts::VBLANK as u32);
    /// assert_eq!(ic.read_mask(), interrupts::VBLANK as u32);
    /// ```
    pub fn read_mask(&self) -> u32 {
        self.mask as u32
    }

    /// Write I_MASK register
    ///
    /// Sets which interrupts are enabled to reach the CPU.
    /// Only the lower 16 bits are used.
    ///
    /// # Arguments
    ///
    /// * `value` - Mask value to set (lower 16 bits used)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::interrupt::{InterruptController, interrupts};
    ///
    /// let mut ic = InterruptController::new();
    /// ic.write_mask((interrupts::VBLANK | interrupts::TIMER0) as u32);
    /// assert_eq!(ic.read_mask(), (interrupts::VBLANK | interrupts::TIMER0) as u32);
    /// ```
    pub fn write_mask(&mut self, value: u32) {
        self.mask = value as u16;
        log::debug!("IRQ mask set: 0x{:04X}", self.mask);
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
