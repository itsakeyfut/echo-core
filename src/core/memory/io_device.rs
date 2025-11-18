// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! I/O Device Trait
//!
//! This module defines a trait-based abstraction for memory-mapped I/O devices.
//! By implementing the `IODevice` trait, peripherals can be registered with the
//! memory bus without requiring the Bus to have explicit knowledge of each device type.
//!
//! # Design Goals
//!
//! - **Decoupling**: Bus doesn't need to know about specific peripheral types
//! - **Extensibility**: New peripherals can be added without modifying Bus
//! - **Testability**: Devices can be tested in isolation with mock implementations
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │              Memory Bus                     │
//! ├─────────────────────────────────────────────┤
//! │  Devices: Vec<Box<dyn IODevice>>            │
//! │                                             │
//! │  read_io_port(addr) {                       │
//! │    for device in devices {                  │
//! │      if device.contains(addr) {             │
//! │        return device.read_register(offset)  │
//! │      }                                      │
//! │    }                                        │
//! │  }                                          │
//! └─────────────────────────────────────────────┘
//!           ▲                   ▲
//!           │                   │
//!    ┌──────┴──────┐    ┌──────┴──────┐
//!    │   GPU       │    │  Timers     │
//!    │ (IODevice)  │    │ (IODevice)  │
//!    └─────────────┘    └─────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use psrx::core::memory::IODevice;
//! use psrx::core::error::Result;
//!
//! struct MyPeripheral {
//!     base_addr: u32,
//!     registers: [u32; 4],
//! }
//!
//! impl IODevice for MyPeripheral {
//!     fn address_range(&self) -> (u32, u32) {
//!         (self.base_addr, self.base_addr + 0x0F)
//!     }
//!
//!     fn read_register(&self, offset: u32) -> Result<u32> {
//!         let index = (offset / 4) as usize;
//!         Ok(self.registers.get(index).copied().unwrap_or(0))
//!     }
//!
//!     fn write_register(&mut self, offset: u32, value: u32) -> Result<()> {
//!         let index = (offset / 4) as usize;
//!         if index < self.registers.len() {
//!             self.registers[index] = value;
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use crate::core::error::Result;

/// Trait for memory-mapped I/O devices
///
/// This trait provides a uniform interface for all memory-mapped peripherals
/// in the PlayStation hardware. Each device declares its address range and
/// implements read/write operations for its registers.
///
/// # Register Access
///
/// The trait provides methods for 8-bit, 16-bit, and 32-bit register access.
/// Devices must implement the 32-bit methods; default implementations are provided
/// for 8-bit and 16-bit access that delegate to the 32-bit methods.
///
/// # Address Translation
///
/// The Bus translates physical addresses to device-relative offsets before
/// calling trait methods. For example:
///
/// - Device address range: `0x1F801810 - 0x1F801817`
/// - Physical address: `0x1F801814`
/// - Offset passed to device: `0x04`
///
/// # Thread Safety
///
/// IODevice implementations do not need to be `Send` or `Sync` as the Bus
/// is not shared across threads. However, they should handle interior mutability
/// properly when accessed through `&self` methods (e.g., using `RefCell`).
pub trait IODevice {
    /// Get the address range this device responds to
    ///
    /// Returns a tuple of (start_address, end_address) inclusive.
    /// The Bus will route any memory access within this range to this device.
    ///
    /// # Returns
    ///
    /// `(start, end)` - Start and end physical addresses (inclusive)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use psrx::core::memory::IODevice;
    /// # struct GPU;
    /// # impl IODevice for GPU {
    /// #     fn address_range(&self) -> (u32, u32) {
    /// // GPU registers: 0x1F801810 - 0x1F801817
    /// (0x1F801810, 0x1F801817)
    /// #     }
    /// #     fn read_register(&self, offset: u32) -> psrx::core::error::Result<u32> { Ok(0) }
    /// #     fn write_register(&mut self, offset: u32, value: u32) -> psrx::core::error::Result<()> { Ok(()) }
    /// # }
    /// ```
    fn address_range(&self) -> (u32, u32);

    /// Check if this device contains the given address
    ///
    /// This is a helper method that checks if an address falls within
    /// this device's address range.
    ///
    /// # Arguments
    ///
    /// * `addr` - Physical address to check
    ///
    /// # Returns
    ///
    /// `true` if the address is within this device's range
    fn contains(&self, addr: u32) -> bool {
        let (start, end) = self.address_range();
        addr >= start && addr <= end
    }

    /// Read a 32-bit value from a device register
    ///
    /// The offset is relative to the device's base address. For example,
    /// if the device base is `0x1F801810` and the access is to `0x1F801814`,
    /// the offset will be `0x04`.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address (must be 4-byte aligned)
    ///
    /// # Returns
    ///
    /// The 32-bit register value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The offset is out of range for this device
    /// - The offset is not properly aligned
    /// - The register is write-only
    fn read_register(&self, offset: u32) -> Result<u32>;

    /// Write a 32-bit value to a device register
    ///
    /// The offset is relative to the device's base address.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address (must be 4-byte aligned)
    /// * `value` - 32-bit value to write
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The offset is out of range for this device
    /// - The offset is not properly aligned
    /// - The register is read-only
    fn write_register(&mut self, offset: u32, value: u32) -> Result<()>;

    /// Read a 16-bit value from a device register
    ///
    /// Default implementation reads the 32-bit value and masks to 16 bits.
    /// Devices can override this if they need special 16-bit handling.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address (must be 2-byte aligned)
    ///
    /// # Returns
    ///
    /// The 16-bit register value
    fn read_register16(&self, offset: u32) -> Result<u16> {
        // Default: read 32-bit and mask
        let value = self.read_register(offset & !0x03)?;
        let shift = (offset & 0x02) * 8;
        Ok(((value >> shift) & 0xFFFF) as u16)
    }

    /// Write a 16-bit value to a device register
    ///
    /// Default implementation performs read-modify-write on the aligned 32-bit word,
    /// updating only the targeted 16-bit half. Devices can override this if they
    /// need special 16-bit handling.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address (must be 2-byte aligned)
    /// * `value` - 16-bit value to write
    fn write_register16(&mut self, offset: u32, value: u16) -> Result<()> {
        // Read-modify-write to update only the target 16-bit field
        let aligned = offset & !0x03;
        let shift = (offset & 0x02) * 8;
        let mask = !(0xFFFFu32 << shift);
        let current = self.read_register(aligned)?;
        let new_value = (current & mask) | ((value as u32) << shift);
        self.write_register(aligned, new_value)
    }

    /// Read an 8-bit value from a device register
    ///
    /// Default implementation reads the 32-bit value and masks to 8 bits.
    /// Devices can override this if they need special 8-bit handling.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address
    ///
    /// # Returns
    ///
    /// The 8-bit register value
    fn read_register8(&self, offset: u32) -> Result<u8> {
        // Default: read 32-bit and mask
        let value = self.read_register(offset & !0x03)?;
        let shift = (offset & 0x03) * 8;
        Ok(((value >> shift) & 0xFF) as u8)
    }

    /// Write an 8-bit value to a device register
    ///
    /// Default implementation performs read-modify-write on the aligned 32-bit word,
    /// updating only the targeted 8-bit byte. Devices can override this if they
    /// need special 8-bit handling.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from device base address
    /// * `value` - 8-bit value to write
    fn write_register8(&mut self, offset: u32, value: u8) -> Result<()> {
        // Read-modify-write to update only the target 8-bit field
        let aligned = offset & !0x03;
        let shift = (offset & 0x03) * 8;
        let mask = !(0xFFu32 << shift);
        let current = self.read_register(aligned)?;
        let new_value = (current & mask) | ((value as u32) << shift);
        self.write_register(aligned, new_value)
    }

    /// Optional: Device name for debugging
    ///
    /// Returns a human-readable name for this device.
    /// Useful for logging and debugging.
    fn name(&self) -> &str {
        "Unknown Device"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::EmulatorError;

    /// Mock device for testing
    struct MockDevice {
        base: u32,
        size: u32,
        registers: Vec<u32>,
    }

    impl MockDevice {
        fn new(base: u32, register_count: usize) -> Self {
            Self {
                base,
                size: (register_count * 4) as u32,
                registers: vec![0; register_count],
            }
        }
    }

    impl IODevice for MockDevice {
        fn address_range(&self) -> (u32, u32) {
            (self.base, self.base + self.size - 1)
        }

        fn read_register(&self, offset: u32) -> Result<u32> {
            let index = (offset / 4) as usize;
            self.registers
                .get(index)
                .copied()
                .ok_or(EmulatorError::InvalidMemoryAccess {
                    address: self.base + offset,
                })
        }

        fn write_register(&mut self, offset: u32, value: u32) -> Result<()> {
            let index = (offset / 4) as usize;
            if index < self.registers.len() {
                self.registers[index] = value;
                Ok(())
            } else {
                Err(EmulatorError::InvalidMemoryAccess {
                    address: self.base + offset,
                })
            }
        }

        fn name(&self) -> &str {
            "MockDevice"
        }
    }

    #[test]
    fn test_address_range() {
        let device = MockDevice::new(0x1F801000, 4);
        assert_eq!(device.address_range(), (0x1F801000, 0x1F80100F));
    }

    #[test]
    fn test_contains() {
        let device = MockDevice::new(0x1F801000, 4);

        assert!(device.contains(0x1F801000));
        assert!(device.contains(0x1F801008));
        assert!(device.contains(0x1F80100F));

        assert!(!device.contains(0x1F800FFF));
        assert!(!device.contains(0x1F801010));
    }

    #[test]
    fn test_read_write_32bit() {
        let mut device = MockDevice::new(0x1F801000, 4);

        // Write and read back
        device.write_register(0x00, 0x12345678).unwrap();
        assert_eq!(device.read_register(0x00).unwrap(), 0x12345678);

        device.write_register(0x04, 0xABCDEF00).unwrap();
        assert_eq!(device.read_register(0x04).unwrap(), 0xABCDEF00);
    }

    #[test]
    fn test_read_write_16bit() {
        let mut device = MockDevice::new(0x1F801000, 4);

        // Write 16-bit value
        device.write_register16(0x00, 0x1234).unwrap();

        // Read back as 16-bit
        assert_eq!(device.read_register16(0x00).unwrap(), 0x1234);
    }

    #[test]
    fn test_read_write_8bit() {
        let mut device = MockDevice::new(0x1F801000, 4);

        // Write 8-bit value
        device.write_register8(0x00, 0xAB).unwrap();

        // Read back as 8-bit
        assert_eq!(device.read_register8(0x00).unwrap(), 0xAB);
    }

    #[test]
    fn test_out_of_range() {
        let mut device = MockDevice::new(0x1F801000, 4);

        // Out of range access should fail
        assert!(device.read_register(0x10).is_err());
        assert!(device.write_register(0x10, 0).is_err());
    }

    #[test]
    fn test_device_name() {
        let device = MockDevice::new(0x1F801000, 4);
        assert_eq!(device.name(), "MockDevice");
    }
}
