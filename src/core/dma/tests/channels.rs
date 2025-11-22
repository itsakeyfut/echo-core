// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! DMA channel-specific tests

use super::super::*;

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
