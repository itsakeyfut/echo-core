// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! DMA transfer operation tests

use super::super::*;
use crate::core::cdrom::CDROM;
use crate::core::gpu::GPU;

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
