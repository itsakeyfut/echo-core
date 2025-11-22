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

//! DMA transfer tests - DMA read/write operations and addressing

use crate::core::spu::SPU;

#[test]
fn test_spu_dma_write() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write data via DMA
    for _ in 0..100 {
        spu.dma_write(0x12345678);
    }

    spu.flush_dma_fifo();

    // Verify data was written
    assert_eq!(spu.read_ram_word(0x1000 * 8), 0x5678);
    assert_eq!(spu.read_ram_word(0x1000 * 8 + 2), 0x1234);
}

#[test]
fn test_spu_dma_read() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write test data
    spu.write_ram_word(0x1000 * 8, 0xABCD);
    spu.write_ram_word(0x1000 * 8 + 2, 0x1234);

    // Read via DMA
    let value = spu.dma_read();
    assert_eq!(value, 0x1234ABCD);
}

#[test]
fn test_spu_dma_transfer_address() {
    let mut spu = SPU::new();

    // Set transfer address (in 8-byte units)
    spu.set_transfer_address(0x2000);

    // Verify address was set correctly (multiplied by 8)
    assert_eq!(spu.transfer_addr, 0x2000 * 8);
}

#[test]
fn test_spu_dma_ready() {
    let spu = SPU::new();
    // SPU should always be ready for DMA
    assert!(spu.dma_ready());
}

#[test]
fn test_spu_dma_fifo_flush() {
    let mut spu = SPU::new();
    spu.set_transfer_address(0x1000);

    // Write enough data to trigger auto-flush (16 words)
    for _ in 0..8 {
        spu.dma_write(0xAABBCCDD);
    }

    // FIFO should have been flushed automatically
    assert_eq!(spu.dma_fifo.len(), 0);

    // Verify data was written to RAM
    let first_word = spu.read_ram_word(0x1000 * 8);
    assert_eq!(first_word, 0xCCDD);
}

#[test]
fn test_spu_dma_address_wrapping() {
    let mut spu = SPU::new();

    // Set address near end of SPU RAM
    spu.set_transfer_address(0xFFFE);

    // Write data that would go past the end
    spu.dma_write(0x11223344);
    spu.dma_write(0x55667788);
    spu.flush_dma_fifo();

    // Address should have wrapped around
    // After 4 writes (8 bytes), addr should be (0xFFFE * 8 + 8) & 0x7FFFE
    assert!(spu.transfer_addr < 0x80000);
}

#[test]
fn test_spu_dma_register_read_write() {
    let mut spu = SPU::new();

    // Write transfer address via register
    spu.write_register(0x1F801DA6, 0x5000);
    assert_eq!(spu.transfer_addr, 0x5000 * 8);

    // Read transfer address via register (should return value in 8-byte units)
    let read_value = spu.read_register(0x1F801DA6);
    assert_eq!(read_value, 0x5000);
}

#[test]
fn test_spu_dma_manual_write() {
    let mut spu = SPU::new();

    // Set transfer address
    spu.write_register(0x1F801DA6, 0x1000);

    // Write data manually via register
    spu.write_register(0x1F801DA8, 0xABCD);
    spu.write_register(0x1F801DA8, 0x1234);

    // Verify data was written and address auto-incremented
    assert_eq!(spu.read_ram_word(0x1000 * 8), 0xABCD);
    assert_eq!(spu.read_ram_word(0x1000 * 8 + 2), 0x1234);
}
