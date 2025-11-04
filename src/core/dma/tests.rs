// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Unit tests for DMA controller

use super::*;

#[test]
fn test_dma_initialization() {
    let dma = DMA::new();

    // All channels should be inactive initially
    for i in 0..7 {
        assert!(!dma.channels[i].is_active());
        assert_eq!(dma.channels[i].base_address, 0);
        assert_eq!(dma.channels[i].block_control, 0);
        assert_eq!(dma.channels[i].channel_control, 0);
    }

    // Control register should have default priority
    assert_eq!(dma.read_control(), 0x0765_4321);

    // Interrupt register should be cleared
    assert_eq!(dma.read_interrupt(), 0);
}

#[test]
fn test_channel_register_access() {
    let mut dma = DMA::new();

    // Test MADR register
    dma.write_madr(DMA::CH_GPU, 0x8012_3456);
    assert_eq!(dma.read_madr(DMA::CH_GPU), 0x0012_3456); // Top byte masked

    // Test BCR register
    dma.write_bcr(DMA::CH_GPU, 0x0010_0020);
    assert_eq!(dma.read_bcr(DMA::CH_GPU), 0x0010_0020);

    // Test CHCR register
    dma.write_chcr(DMA::CH_GPU, 0x0100_0201);
    assert_eq!(dma.read_chcr(DMA::CH_GPU), 0x0100_0201);
}

#[test]
fn test_channel_control_bits() {
    let mut channel = DMAChannel::new(2);

    // Initially inactive
    assert!(!channel.is_active());
    assert!(!channel.trigger());

    // Set active bit (bit 24)
    channel.channel_control = 0x0100_0000;
    assert!(channel.is_active());

    // Set trigger bit (bit 28)
    channel.channel_control = 0x1000_0000;
    assert!(channel.trigger());

    // Test direction
    channel.channel_control = 0;
    assert_eq!(channel.direction(), DMAChannel::TRANSFER_TO_RAM);

    channel.channel_control = 1;
    assert_eq!(channel.direction(), DMAChannel::TRANSFER_FROM_RAM);

    // Test sync modes
    channel.channel_control = 0 << 9;
    assert_eq!(channel.sync_mode(), 0);

    channel.channel_control = 1 << 9;
    assert_eq!(channel.sync_mode(), 1);

    channel.channel_control = 2 << 9;
    assert_eq!(channel.sync_mode(), 2);
}

#[test]
fn test_otc_transfer() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Setup OTC transfer
    let base_addr = 0x1000;
    let entry_count = 8;

    dma.write_madr(DMA::CH_OTC, base_addr);
    dma.write_bcr(DMA::CH_OTC, entry_count);
    dma.write_chcr(DMA::CH_OTC, 0x1100_0000); // Active + trigger

    // Execute transfer
    let irq = dma.transfer_otc(&mut ram);

    // Should generate interrupt
    assert!(irq);

    // Channel should be deactivated
    assert!(!dma.channels[DMA::CH_OTC].is_active());

    // Verify ordering table entries
    // Last entry should be end marker
    let last_entry_addr = base_addr - (entry_count - 1) * 4;
    let last_entry = dma.read_ram_u32(&ram, last_entry_addr);
    assert_eq!(last_entry, 0x00FF_FFFF);

    // Other entries should link backwards
    let mut addr = base_addr;
    for i in 0..entry_count - 1 {
        let entry = dma.read_ram_u32(&ram, addr);
        let expected_link = (addr - 4) & 0x001F_FFFC;
        assert_eq!(
            entry, expected_link,
            "Entry {} at 0x{:X} should link to 0x{:X}, got 0x{:X}",
            i, addr, expected_link, entry
        );
        addr -= 4;
    }
}

#[test]
fn test_otc_transfer_single_entry() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Setup OTC transfer with single entry
    dma.write_madr(DMA::CH_OTC, 0x1000);
    dma.write_bcr(DMA::CH_OTC, 1); // Only 1 entry
    dma.write_chcr(DMA::CH_OTC, 0x1100_0000);

    dma.transfer_otc(&mut ram);

    // Single entry should be end marker
    let entry = dma.read_ram_u32(&ram, 0x1000);
    assert_eq!(entry, 0x00FF_FFFF);
}

#[test]
fn test_ram_word_access() {
    let dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Test write
    dma.write_ram_u32(&mut ram, 0x1000, 0x12345678);

    // Test read
    let value = dma.read_ram_u32(&ram, 0x1000);
    assert_eq!(value, 0x12345678);

    // Verify byte order (little-endian)
    assert_eq!(ram[0x1000], 0x78);
    assert_eq!(ram[0x1001], 0x56);
    assert_eq!(ram[0x1002], 0x34);
    assert_eq!(ram[0x1003], 0x12);
}

#[test]
fn test_ram_word_access_with_masking() {
    let dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];

    // Test that address masking works correctly
    dma.write_ram_u32(&mut ram, 0xFFFF_FFFF, 0xDEADBEEF);

    // Should write to 0x001F_FFFC (masked address)
    let value = dma.read_ram_u32(&ram, 0x001F_FFFC);
    assert_eq!(value, 0xDEADBEEF);
}

#[test]
fn test_dpcr_access() {
    let mut dma = DMA::new();

    // Default value
    assert_eq!(dma.read_control(), 0x0765_4321);

    // Write new value
    dma.write_control(0x1234_5678);
    assert_eq!(dma.read_control(), 0x1234_5678);
}

#[test]
fn test_dicr_access() {
    let mut dma = DMA::new();

    // Initial value
    assert_eq!(dma.read_interrupt(), 0);

    // Write configuration bits (bits 0-5 are reserved and should be preserved as 0)
    // Write without setting force flag (bit 15) to avoid triggering master flag (bit 31)
    dma.write_interrupt(0x00FF_7FC0);
    assert_eq!(dma.read_interrupt(), 0x00FF_7FC0); // Bits 6-14 and 16-23 are writable

    // Test that force flag (bit 15) causes master flag (bit 31) to be set
    dma.write_interrupt(0x0000_8000); // Set only force flag
    assert_eq!(dma.read_interrupt(), 0x8000_8000); // Master flag (bit 31) should be set

    // Clear force flag and verify master flag is cleared
    dma.write_interrupt(0x0000_0000); // Clear force flag
    assert_eq!(dma.read_interrupt(), 0x0000_0000);

    // Set up configuration with channel enables and master enable
    dma.write_interrupt(0x00FF_FFC0); // Set all config bits including force flag
    assert_eq!(dma.read_interrupt(), 0x80FF_FFC0); // Master flag (bit 31) set due to force flag

    // Test write-1-to-clear for bits 24-30 (interrupt flags)
    // Note: Since bits 6-23 are always updated, we need to re-write config to preserve it
    dma.write_interrupt(0x7FFF_FFC0); // Clear all interrupt flags and re-write config
    assert_eq!(dma.read_interrupt(), 0x80FF_FFC0); // Flags cleared, config preserved, master flag set
}

#[test]
fn test_channel_deactivation() {
    let mut channel = DMAChannel::new(0);

    // Activate channel
    channel.channel_control = 0x0100_0000;
    assert!(channel.is_active());

    // Deactivate
    channel.deactivate();
    assert!(!channel.is_active());

    // Other bits should remain unchanged
    channel.channel_control = 0x1100_0201;
    assert!(channel.is_active());
    assert!(channel.trigger());

    channel.deactivate();
    assert!(!channel.is_active());
    assert!(channel.trigger()); // Trigger bit should remain set
}

#[test]
fn test_gpu_dma_linked_list_simple() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];
    let mut gpu = GPU::new();

    // Create a simple linked list with one node
    let list_addr = 0x1000;

    // Node 1: header with count=2 and end marker
    dma.write_ram_u32(&mut ram, list_addr, 0x0280_0000); // 2 words + end bit

    // Command data
    dma.write_ram_u32(&mut ram, list_addr + 4, 0xE100_0000); // GP0 command
    dma.write_ram_u32(&mut ram, list_addr + 8, 0x00FF_FF00); // Command data

    // Setup GPU DMA
    dma.write_madr(DMA::CH_GPU, list_addr);
    dma.write_bcr(DMA::CH_GPU, 0); // Not used in linked-list mode
    dma.write_chcr(DMA::CH_GPU, 0x1100_0401); // Active + trigger + linked-list mode

    // Execute transfer
    let irq = dma.transfer_gpu(&mut ram, &mut gpu);

    // Should complete and generate interrupt
    assert!(irq);
    assert!(!dma.channels[DMA::CH_GPU].is_active());
}

#[test]
fn test_gpu_dma_linked_list_chain() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];
    let mut gpu = GPU::new();

    // Create a linked list with 3 nodes
    let node1_addr = 0x1000;
    let node2_addr = 0x2000;
    let node3_addr = 0x3000;

    // Node 1: 1 word, link to node 2
    dma.write_ram_u32(&mut ram, node1_addr, 0x0100_0000 | node2_addr);
    dma.write_ram_u32(&mut ram, node1_addr + 4, 0xA0000000);

    // Node 2: 2 words, link to node 3
    dma.write_ram_u32(&mut ram, node2_addr, 0x0200_0000 | node3_addr);
    dma.write_ram_u32(&mut ram, node2_addr + 4, 0xB0000000);
    dma.write_ram_u32(&mut ram, node2_addr + 8, 0xB1111111);

    // Node 3: 1 word, end of list
    dma.write_ram_u32(&mut ram, node3_addr, 0x0180_0000); // End marker
    dma.write_ram_u32(&mut ram, node3_addr + 4, 0xC0000000);

    // Setup GPU DMA
    dma.write_madr(DMA::CH_GPU, node1_addr);
    dma.write_chcr(DMA::CH_GPU, 0x1100_0401); // Linked-list mode

    // Execute transfer
    let irq = dma.transfer_gpu(&mut ram, &mut gpu);

    assert!(irq);
    assert!(!dma.channels[DMA::CH_GPU].is_active());
}

#[test]
fn test_cdrom_dma_transfer() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];
    let mut cdrom = CDROM::new();

    // Fill CD-ROM data buffer with test pattern
    for i in 0..512 {
        cdrom.push_data_byte((i & 0xFF) as u8);
    }

    // Setup CD-ROM DMA: transfer 128 words (512 bytes)
    let dest_addr = 0x1000;
    dma.write_madr(DMA::CH_CDROM, dest_addr);
    dma.write_bcr(DMA::CH_CDROM, 0x0001_0080); // 1 block of 128 words
    dma.write_chcr(DMA::CH_CDROM, 0x1100_0000); // Active + trigger

    // Execute transfer
    let irq = dma.transfer_cdrom(&mut ram, &mut cdrom);

    assert!(irq);
    assert!(!dma.channels[DMA::CH_CDROM].is_active());

    // Verify transferred data
    for i in 0..512 {
        assert_eq!(ram[dest_addr as usize + i], i as u8, "Byte {} mismatch", i);
    }
}

#[test]
fn test_gpu_dma_block_mode_to_gpu() {
    let mut dma = DMA::new();
    let mut ram = vec![0u8; 2 * 1024 * 1024];
    let mut gpu = GPU::new();

    // Setup test data in RAM
    let src_addr = 0x1000;
    for i in 0..16 {
        dma.write_ram_u32(&mut ram, src_addr + i * 4, 0xA000_0000 + i);
    }

    // Setup GPU DMA block transfer: 16 words
    dma.write_madr(DMA::CH_GPU, src_addr);
    dma.write_bcr(DMA::CH_GPU, 0x0001_0010); // 1 block of 16 words
    dma.write_chcr(DMA::CH_GPU, 0x1100_0201); // Active + trigger + block mode + to device

    // Execute transfer
    let irq = dma.transfer_gpu(&mut ram, &mut gpu);

    assert!(irq);
    assert!(!dma.channels[DMA::CH_GPU].is_active());
}
