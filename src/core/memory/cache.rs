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

//! Instruction cache management for memory bus
//!
//! This module handles the ICache prefill and invalidation queues used to maintain
//! cache coherency between the memory bus and the CPU's instruction cache.
//!
//! # ICache Prefill
//!
//! When the BIOS copies code from ROM to RAM (e.g., 0xBFC10000 -> 0xA0000500),
//! we track these writes and queue them for prefilling the CPU's instruction cache.
//! This ensures instructions are cached before RAM is zeroed by BIOS initialization.
//!
//! # ICache Invalidation
//!
//! When memory is written that may contain already-cached instructions
//! (e.g., self-modifying code, runtime patching), we queue the addresses
//! for cache invalidation to maintain coherency.

use super::Bus;

impl Bus {
    /// Drain the icache prefill queue
    ///
    /// Returns all queued (address, instruction) pairs and clears the queue.
    /// This should be called periodically by the System to apply prefills to
    /// the CPU's instruction cache.
    pub fn drain_icache_prefill_queue(&mut self) -> Vec<(u32, u32)> {
        self.icache_prefill_queue.drain(..).collect()
    }

    /// Drain the icache invalidation queue
    ///
    /// Returns all queued addresses for invalidation and clears the queue.
    /// This should be called periodically by the System to invalidate stale
    /// cache entries when memory is modified.
    pub fn drain_icache_invalidate_queue(&mut self) -> Vec<u32> {
        self.icache_invalidate_queue.drain(..).collect()
    }

    /// Drain the icache range invalidation queue
    ///
    /// Returns all queued (start, end) address ranges for invalidation and clears the queue.
    /// This should be called periodically by the System to invalidate ranges of stale
    /// cache entries (e.g., when loading executables).
    pub fn drain_icache_invalidate_range_queue(&mut self) -> Vec<(u32, u32)> {
        self.icache_invalidate_range_queue.drain(..).collect()
    }

    /// Queue an instruction for ICache prefill
    ///
    /// Called when BIOS copies code to RAM. Queues both cached and uncached
    /// address aliases for prefilling.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address where instruction was written
    /// * `instruction` - The instruction word that was written
    pub(super) fn queue_icache_prefill(&mut self, paddr: u32, instruction: u32) {
        let offset = paddr as usize;

        // Only prefill for code in the low memory region
        if (Self::ICACHE_PREFILL_START..=Self::ICACHE_PREFILL_END).contains(&offset) {
            // Queue for cached addresses (KSEG0: 0x80000000-0x9FFFFFFF)
            let cached_addr = 0x80000000 | paddr;
            self.icache_prefill_queue.push((cached_addr, instruction));

            // And for uncached addresses (KUSEG: 0x00000000-0x7FFFFFFF)
            self.icache_prefill_queue.push((paddr, instruction));
        }
    }

    /// Queue an address for ICache invalidation
    ///
    /// Called when RAM is written. Queues both cached and uncached
    /// address aliases for invalidation to maintain cache coherency.
    ///
    /// # Arguments
    ///
    /// * `paddr` - Physical address that was written
    pub(super) fn queue_icache_invalidation(&mut self, paddr: u32) {
        // Queue for icache invalidation (all RAM writes)
        // This maintains cache coherency for self-modifying code,
        // runtime patching, and DMA writes to instruction memory
        let cached_addr = 0x80000000 | paddr;
        self.icache_invalidate_queue.push(cached_addr);
        self.icache_invalidate_queue.push(paddr); // Also uncached alias
    }

    /// Queue an address range for ICache invalidation
    ///
    /// Called when bulk data is written to RAM (e.g., executable loading).
    /// Queues both cached and uncached address aliases for the entire range.
    ///
    /// # Arguments
    ///
    /// * `start_paddr` - Physical start address of the written range
    /// * `end_paddr` - Physical end address (exclusive) of the written range
    pub(super) fn queue_icache_range_invalidation(&mut self, start_paddr: u32, end_paddr: u32) {
        // Queue cached addresses (KSEG0: 0x80000000-0x9FFFFFFF)
        let cached_start = 0x80000000 | start_paddr;
        let cached_end = 0x80000000 | end_paddr;
        self.icache_invalidate_range_queue
            .push((cached_start, cached_end));

        // Also queue uncached aliases (KUSEG: 0x00000000-0x7FFFFFFF)
        self.icache_invalidate_range_queue
            .push((start_paddr, end_paddr));
    }
}
