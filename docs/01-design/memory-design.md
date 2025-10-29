# Memory System Design

## Overview

The PSX memory system is a complex integration of physical memory, memory-mapped I/O, cache, and DMA. This document provides a detailed explanation of the memory system design in the emulator.

## PSX Memory Architecture

### Physical Memory Configuration

```
┌─────────────────────────────────────────┐
│     PSX Memory Architecture             │
├─────────────────────────────────────────┤
│  RAM (Main Memory)      : 2MB           │
│  Scratchpad (D-Cache)   : 1KB           │
│  BIOS ROM               : 512KB         │
│  I/O Ports              : ~8KB          │
│  Expansion (Unused)     : 8MB (Reserved)│
└─────────────────────────────────────────┘
```

### Address Space Mapping

PSX uses a 32-bit address space with the following physical layout:

```
┌──────────────┬─────────────────┬──────────┬───────────────┐
│ Address Range│ Description     │ Size     │ Cache         │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x00000000   │ RAM             │ 2MB      │ Yes (KUSEG)   │
│ - 0x001FFFFF │                 │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x1F000000   │ Expansion 1     │ 8MB      │ No            │
│ - 0x1F7FFFFF │ (Unused)        │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x1F800000   │ Scratchpad      │ 1KB      │ N/A (Built-in)│
│ - 0x1F8003FF │                 │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x1F801000   │ I/O Ports       │ 8KB      │ No            │
│ - 0x1F802FFF │                 │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x1F810000   │ Expansion 2     │ TBD      │ No            │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x1FC00000   │ BIOS ROM        │ 512KB    │ Yes           │
│ - 0x1FC7FFFF │                 │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0x80000000   │ RAM (Mirror)    │ 2MB      │ Yes (KSEG0)   │
│ - 0x801FFFFF │ Cache Enabled   │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0xA0000000   │ RAM (Mirror)    │ 2MB      │ No (KSEG1)    │
│ - 0xA01FFFFF │ Cache Disabled  │          │               │
├──────────────┼─────────────────┼──────────┼───────────────┤
│ 0xBFC00000   │ BIOS (Mirror)   │ 512KB    │ No (KSEG1)    │
│ - 0xBFC7FFFF │                 │          │               │
└──────────────┴─────────────────┴──────────┴───────────────┘
```

### Memory Region Descriptions

**KUSEG (0x00000000 - 0x7FFFFFFF):**
- User mode space
- Virtual address translation via TLB (disabled on PSX)
- Cache enabled

**KSEG0 (0x80000000 - 0x9FFFFFFF):**
- Kernel mode space
- Direct mapping to physical address (ignore upper 3 bits)
- Cache enabled
- Most frequently used

**KSEG1 (0xA0000000 - 0xBFFFFFFF):**
- Kernel mode space
- Direct mapping to physical address
- **Cache disabled**
- Used for I/O access

## Emulator Design

### Bus Structure

The central structure that manages all memory access.

```rust
/// Memory Bus
///
/// Mediates all memory access and I/O access
pub struct Bus {
    /// Main RAM (2MB)
    ram: Vec<u8>,

    /// Scratchpad (1KB fast RAM)
    scratchpad: [u8; 1024],

    /// BIOS ROM (512KB)
    bios: Vec<u8>,

    /// References to each hardware component
    /// In actual implementation, these are managed differently
    /// (e.g., Rc<RefCell<T>> or raw pointers)
}

impl Bus {
    /// Create a new Bus instance
    pub fn new() -> Self {
        Self {
            ram: vec![0; 2 * 1024 * 1024],  // 2MB
            scratchpad: [0; 1024],
            bios: vec![0; 512 * 1024],      // 512KB
        }
    }

    /// Load BIOS file
    pub fn load_bios(&mut self, path: &str) -> Result<()> {
        let data = std::fs::read(path)?;

        if data.len() != 512 * 1024 {
            return Err(EmulatorError::InvalidBiosSize {
                expected: 512 * 1024,
                got: data.len(),
            });
        }

        self.bios.copy_from_slice(&data);
        Ok(())
    }
}
```

### Address Translation

PSX's address map has complex mirroring that requires translation to physical addresses.

```rust
impl Bus {
    /// Translate virtual address to physical address
    ///
    /// Resolves KUSEG/KSEG0/KSEG1 mirroring
    #[inline(always)]
    fn translate_address(&self, vaddr: u32) -> u32 {
        // Determine region by upper 3 bits of address
        // 0x00000000-0x1FFFFFFF: KUSEG (use as-is)
        // 0x80000000-0x9FFFFFFF: KSEG0 (strip upper bits)
        // 0xA0000000-0xBFFFFFFF: KSEG1 (strip upper bits)

        // Simplified version: use only lower 29 bits (mask upper 3 bits)
        vaddr & 0x1FFF_FFFF
    }

    /// Identify memory region
    ///
    /// Used for debugging and access control
    fn identify_region(&self, vaddr: u32) -> MemoryRegion {
        let paddr = self.translate_address(vaddr);

        match paddr {
            0x00000000..=0x001FFFFF => MemoryRegion::RAM,
            0x1F800000..=0x1F8003FF => MemoryRegion::Scratchpad,
            0x1F801000..=0x1F802FFF => MemoryRegion::IO,
            0x1FC00000..=0x1FC7FFFF => MemoryRegion::BIOS,
            _ => MemoryRegion::Unmapped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegion {
    RAM,
    Scratchpad,
    IO,
    BIOS,
    Unmapped,
}
```

### Memory Read Operations

```rust
impl Bus {
    /// 8-bit read
    pub fn read8(&self, vaddr: u32) -> Result<u8> {
        let paddr = self.translate_address(vaddr);

        match paddr {
            // RAM
            0x00000000..=0x001FFFFF => {
                Ok(self.ram[paddr as usize])
            }

            // Scratchpad
            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                Ok(self.scratchpad[offset])
            }

            // I/O Ports
            0x1F801000..=0x1F802FFF => {
                self.read_io_port(paddr)
            }

            // BIOS
            0x1FC00000..=0x1FC7FFFF => {
                let offset = (paddr - 0x1FC00000) as usize;
                Ok(self.bios[offset])
            }

            // Unmapped
            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }

    /// 16-bit read
    pub fn read16(&self, vaddr: u32) -> Result<u16> {
        // Alignment check
        if vaddr & 0x1 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 2
            });
        }

        let paddr = self.translate_address(vaddr);

        match paddr {
            0x00000000..=0x001FFFFF => {
                // Little-endian
                let offset = paddr as usize;
                let low = self.ram[offset] as u16;
                let high = self.ram[offset + 1] as u16;
                Ok(low | (high << 8))
            }

            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                let low = self.scratchpad[offset] as u16;
                let high = self.scratchpad[offset + 1] as u16;
                Ok(low | (high << 8))
            }

            0x1F801000..=0x1F802FFF => {
                self.read_io_port16(paddr)
            }

            0x1FC00000..=0x1FC7FFFF => {
                let offset = (paddr - 0x1FC00000) as usize;
                let low = self.bios[offset] as u16;
                let high = self.bios[offset + 1] as u16;
                Ok(low | (high << 8))
            }

            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }

    /// 32-bit read (most frequently used)
    #[inline(always)]
    pub fn read32(&self, vaddr: u32) -> Result<u32> {
        // Alignment check
        if vaddr & 0x3 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 4
            });
        }

        let paddr = self.translate_address(vaddr);

        match paddr {
            0x00000000..=0x001FFFFF => {
                let offset = paddr as usize;
                // Little-endian (combine 4 bytes into one u32)
                Ok(u32::from_le_bytes([
                    self.ram[offset],
                    self.ram[offset + 1],
                    self.ram[offset + 2],
                    self.ram[offset + 3],
                ]))
            }

            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                Ok(u32::from_le_bytes([
                    self.scratchpad[offset],
                    self.scratchpad[offset + 1],
                    self.scratchpad[offset + 2],
                    self.scratchpad[offset + 3],
                ]))
            }

            0x1F801000..=0x1F802FFF => {
                self.read_io_port32(paddr)
            }

            0x1FC00000..=0x1FC7FFFF => {
                let offset = (paddr - 0x1FC00000) as usize;
                Ok(u32::from_le_bytes([
                    self.bios[offset],
                    self.bios[offset + 1],
                    self.bios[offset + 2],
                    self.bios[offset + 3],
                ]))
            }

            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }
}
```

### Memory Write Operations

```rust
impl Bus {
    /// 8-bit write
    pub fn write8(&mut self, vaddr: u32, value: u8) -> Result<()> {
        let paddr = self.translate_address(vaddr);

        match paddr {
            // RAM
            0x00000000..=0x001FFFFF => {
                self.ram[paddr as usize] = value;
                Ok(())
            }

            // Scratchpad
            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                self.scratchpad[offset] = value;
                Ok(())
            }

            // I/O Ports
            0x1F801000..=0x1F802FFF => {
                self.write_io_port(paddr, value)
            }

            // BIOS (Read-Only)
            0x1FC00000..=0x1FC7FFFF => {
                log::warn!("Attempt to write to BIOS at 0x{:08X}", vaddr);
                Ok(())  // Ignore writes
            }

            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }

    /// 16-bit write
    pub fn write16(&mut self, vaddr: u32, value: u16) -> Result<()> {
        if vaddr & 0x1 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 2
            });
        }

        let paddr = self.translate_address(vaddr);
        let bytes = value.to_le_bytes();

        match paddr {
            0x00000000..=0x001FFFFF => {
                let offset = paddr as usize;
                self.ram[offset] = bytes[0];
                self.ram[offset + 1] = bytes[1];
                Ok(())
            }

            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                self.scratchpad[offset] = bytes[0];
                self.scratchpad[offset + 1] = bytes[1];
                Ok(())
            }

            0x1F801000..=0x1F802FFF => {
                self.write_io_port16(paddr, value)
            }

            0x1FC00000..=0x1FC7FFFF => {
                log::warn!("Attempt to write to BIOS at 0x{:08X}", vaddr);
                Ok(())
            }

            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }

    /// 32-bit write (most frequently used)
    #[inline(always)]
    pub fn write32(&mut self, vaddr: u32, value: u32) -> Result<()> {
        if vaddr & 0x3 != 0 {
            return Err(EmulatorError::UnalignedAccess {
                address: vaddr,
                size: 4
            });
        }

        let paddr = self.translate_address(vaddr);
        let bytes = value.to_le_bytes();

        match paddr {
            0x00000000..=0x001FFFFF => {
                let offset = paddr as usize;
                self.ram[offset..offset + 4].copy_from_slice(&bytes);
                Ok(())
            }

            0x1F800000..=0x1F8003FF => {
                let offset = (paddr - 0x1F800000) as usize;
                self.scratchpad[offset..offset + 4].copy_from_slice(&bytes);
                Ok(())
            }

            0x1F801000..=0x1F802FFF => {
                self.write_io_port32(paddr, value)
            }

            0x1FC00000..=0x1FC7FFFF => {
                log::warn!("Attempt to write to BIOS at 0x{:08X}", vaddr);
                Ok(())
            }

            _ => {
                Err(EmulatorError::InvalidAddress { address: vaddr })
            }
        }
    }
}
```

### I/O Port Access

I/O ports are registers for each hardware component.

```rust
impl Bus {
    /// I/O port read (32-bit)
    fn read_io_port32(&self, paddr: u32) -> Result<u32> {
        match paddr {
            // Memory Control
            0x1F801000..=0x1F801023 => {
                log::trace!("Memory control read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // Peripheral I/O Ports
            0x1F801040..=0x1F80104F => {
                // JOY_DATA (Controller)
                log::trace!("Controller port read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // Interrupt Controller
            0x1F801070..=0x1F801077 => {
                // I_STAT, I_MASK
                log::trace!("Interrupt controller read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // DMA Registers
            0x1F801080..=0x1F8010FF => {
                log::trace!("DMA register read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // Timers
            0x1F801100..=0x1F80112F => {
                log::trace!("Timer read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // CD-ROM
            0x1F801800..=0x1F801803 => {
                log::trace!("CD-ROM read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // GPU
            0x1F801810..=0x1F801817 => {
                log::trace!("GPU read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            // SPU
            0x1F801C00..=0x1F801FFF => {
                log::trace!("SPU read at 0x{:08X}", paddr);
                Ok(0)  // TODO: Implement
            }

            _ => {
                log::warn!("Unknown I/O port read at 0x{:08X}", paddr);
                Ok(0)
            }
        }
    }

    /// I/O port write (32-bit)
    fn write_io_port32(&mut self, paddr: u32, value: u32) -> Result<()> {
        match paddr {
            0x1F801000..=0x1F801023 => {
                log::trace!("Memory control write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801040..=0x1F80104F => {
                log::trace!("Controller port write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801070..=0x1F801077 => {
                log::trace!("Interrupt controller write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801080..=0x1F8010FF => {
                log::trace!("DMA register write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801100..=0x1F80112F => {
                log::trace!("Timer write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801800..=0x1F801803 => {
                log::trace!("CD-ROM write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801810..=0x1F801817 => {
                log::trace!("GPU write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            0x1F801C00..=0x1F801FFF => {
                log::trace!("SPU write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())  // TODO: Implement
            }

            _ => {
                log::warn!("Unknown I/O port write at 0x{:08X} = 0x{:08X}", paddr, value);
                Ok(())
            }
        }
    }
}
```

## DMA (Direct Memory Access)

DMA transfers data between memory and devices without CPU intervention.

```rust
/// DMA transfer direction
#[derive(Debug, Clone, Copy)]
pub enum DMADirection {
    ToDevice,      // RAM → Device
    FromDevice,    // Device → RAM
}

/// DMA transfer mode
#[derive(Debug, Clone, Copy)]
pub enum DMAMode {
    Immediate,     // Transfer immediately
    Sync,          // Synchronous transfer
    LinkedList,    // Linked list mode (for GPU commands)
}

impl Bus {
    /// Execute DMA transfer
    pub fn dma_transfer(
        &mut self,
        channel: u8,
        base_addr: u32,
        block_size: u32,
        block_count: u32,
        direction: DMADirection,
        mode: DMAMode,
    ) -> Result<()> {
        match mode {
            DMAMode::Immediate => {
                self.dma_transfer_immediate(
                    channel, base_addr, block_size, direction
                )
            }
            DMAMode::Sync => {
                self.dma_transfer_sync(
                    channel, base_addr, block_size, block_count, direction
                )
            }
            DMAMode::LinkedList => {
                self.dma_transfer_linked_list(
                    channel, base_addr
                )
            }
        }
    }

    /// Immediate transfer mode
    fn dma_transfer_immediate(
        &mut self,
        _channel: u8,
        base_addr: u32,
        size: u32,
        direction: DMADirection,
    ) -> Result<()> {
        let addr = base_addr & 0x1FFFFC;  // Alignment

        for i in 0..size {
            let offset = addr + (i * 4);

            match direction {
                DMADirection::ToDevice => {
                    let data = self.read32(offset)?;
                    // Send to device (TODO: Implement)
                    log::trace!("DMA to device: 0x{:08X}", data);
                }
                DMADirection::FromDevice => {
                    // Receive from device (TODO: Implement)
                    let data = 0;  // Placeholder
                    self.write32(offset, data)?;
                }
            }
        }

        Ok(())
    }
}
```

## Cache Emulation

PSX has I-Cache (instruction cache) and D-Cache (data cache, used as scratchpad).

### Cache Strategy

**Phase 1: Ignore Cache**
- Prioritize accuracy
- All memory accesses directly access RAM/ROM
- Small performance impact

**Phase 2 and later: Simulate Cache (Optional)**
- Only implement if ultra-high accuracy is required
- Unnecessary for most games

```rust
// Future implementation example (Phase 2 and later)
pub struct InstructionCache {
    lines: Vec<CacheLine>,
    enabled: bool,
}

struct CacheLine {
    tag: u32,
    data: [u32; 4],  // 16 bytes/line
    valid: bool,
}
```

## Performance Optimization

### Hot Path Optimization for Memory Access

```rust
impl Bus {
    /// Optimized RAM read (without bounds checking)
    ///
    /// # Safety
    /// Caller must guarantee address validity
    #[inline(always)]
    pub unsafe fn read32_ram_unchecked(&self, offset: usize) -> u32 {
        debug_assert!(offset + 4 <= self.ram.len());

        // Assume aligned
        let ptr = self.ram.as_ptr().add(offset) as *const u32;
        ptr.read_unaligned()
    }
}
```

### Memory Pooling

Consider memory pooling if frequent allocations occur.

```rust
// Future optimization
pub struct MemoryPool {
    pool: Vec<Vec<u8>>,
}

impl MemoryPool {
    pub fn allocate(&mut self, size: usize) -> Vec<u8> {
        self.pool.pop().unwrap_or_else(|| vec![0; size])
    }

    pub fn deallocate(&mut self, buffer: Vec<u8>) {
        self.pool.push(buffer);
    }
}
```

## Testing Strategy

### Unit Tests

```rust
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

        // Write
        bus.write32(0x80000000, 0x12345678).unwrap();

        // Read (different segments)
        assert_eq!(bus.read32(0x00000000).unwrap(), 0x12345678);
        assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
        assert_eq!(bus.read32(0xA0000000).unwrap(), 0x12345678);
    }

    #[test]
    fn test_bios_read_only() {
        let mut bus = Bus::new();

        // Writes to BIOS region are ignored
        bus.write32(0xBFC00000, 0xDEADBEEF).unwrap();

        // Value remains unchanged
        assert_eq!(bus.read32(0xBFC00000).unwrap(), 0x00000000);
    }

    #[test]
    fn test_alignment() {
        let bus = Bus::new();

        // Alignment violations
        assert!(bus.read32(0x80000001).is_err());
        assert!(bus.read16(0x80000001).is_err());
    }
}
```

## Summary

### Implementation Checklist

**Phase 1:**
- [x] Bus structure implementation
- [x] Address translation
- [x] RAM read/write
- [x] BIOS read/write
- [x] Scratchpad read/write
- [x] I/O port skeleton
- [x] Alignment checks
- [x] Unit tests

**Phase 2 and later:**
- [ ] Detailed implementation of each I/O port
- [ ] DMA transfer
- [ ] Cache simulation (optional)
- [ ] Performance optimization

## Related Documents

- [PSX Hardware Overview](../03-hardware-specs/psx-overview.md)
- [Memory Map Details](../03-hardware-specs/memory-map.md)
- [CPU Design](./cpu-design.md)
- [System Architecture](../00-overview/architecture.md)

---

## Update History

- 2025-01-29: Initial version created (translated from Japanese)
