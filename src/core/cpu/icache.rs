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

//! Instruction cache for MIPS R3000A CPU
//!
//! This module implements a direct-mapped instruction cache that mimics
//! the behavior of the PSX's real hardware I-cache.
//!
//! # Hardware Specifications
//!
//! The PSX CPU (MIPS R3000A) has a 4KB instruction cache with the following characteristics:
//! - **Size**: 4KB (1024 cache lines × 4 bytes per line)
//! - **Organization**: Direct-mapped
//! - **Line size**: 4 bytes (1 instruction per line)
//! - **Indexing**: Lower 12 bits (bits [11:2]) select the cache line
//! - **Tag**: Upper 20 bits (bits [31:12]) identify the cached address
//!
//! # Design Rationale
//!
//! A direct-mapped cache was chosen for:
//! 1. **Hardware accuracy**: Matches the real PSX I-cache behavior
//! 2. **Performance**: O(1) lookup with no search required
//! 3. **Spatial locality**: Leverages sequential instruction execution patterns
//! 4. **Predictability**: Deterministic eviction policy (no LRU overhead)
//!
//! # Example
//!
//! ```
//! use psrx::core::cpu::icache::InstructionCache;
//!
//! let mut cache = InstructionCache::new();
//!
//! // Store instruction
//! cache.store(0x80010000, 0x3C080000); // lui r8, 0x0000
//!
//! // Fetch instruction (cache hit)
//! assert_eq!(cache.fetch(0x80010000), Some(0x3C080000));
//!
//! // Invalidate entry
//! cache.invalidate(0x80010000);
//! assert_eq!(cache.fetch(0x80010000), None);
//! ```

/// A single cache line in the instruction cache
///
/// Each cache line stores:
/// - **tag**: Upper 20 bits of the address (bits [31:12])
/// - **data**: The 32-bit instruction word
/// - **valid**: Whether this cache line contains valid data
#[derive(Debug, Clone, Copy)]
struct CacheLine {
    /// Address tag (upper 20 bits)
    tag: u32,
    /// Cached instruction word
    data: u32,
    /// Valid bit
    valid: bool,
}

impl CacheLine {
    /// Create a new invalid cache line
    #[inline(always)]
    const fn new() -> Self {
        Self {
            tag: 0,
            data: 0,
            valid: false,
        }
    }
}

/// Direct-mapped instruction cache for MIPS R3000A
///
/// Implements a 4KB instruction cache with 1024 cache lines,
/// matching the PSX hardware specifications.
///
/// # Cache Organization
///
/// ```text
/// Address format (32 bits):
/// [31:12] Tag (20 bits) - Identifies which address is cached
/// [11:2]  Index (10 bits) - Selects cache line (0-1023)
/// [1:0]   Byte offset (always 00 for word-aligned instructions)
/// ```
///
/// # Performance Characteristics
///
/// - **Lookup**: O(1) - Direct indexing, no search required
/// - **Store**: O(1) - Direct replacement
/// - **Invalidate**: O(1) - Single entry
/// - **Invalidate range**: O(n) - Linear scan of affected lines
/// - **Clear**: O(1) - Bulk reset
///
/// # Memory Usage
///
/// - 1024 cache lines × 12 bytes per line = 12KB total
/// - Each line contains: tag (4 bytes) + data (4 bytes) + valid (1 byte) + padding (3 bytes)
pub struct InstructionCache {
    /// Cache lines (1024 entries for 4KB cache)
    lines: Vec<CacheLine>,
}

impl InstructionCache {
    /// Number of cache lines (4KB / 4 bytes per instruction)
    const LINE_COUNT: usize = 1024;

    /// Bit mask for extracting the cache line index (bits [11:2])
    const INDEX_MASK: u32 = 0x3FF; // 10 bits for 1024 lines

    /// Bit shift for extracting the cache line index from address
    const INDEX_SHIFT: u32 = 2;

    /// Bit shift for extracting the tag from address
    const TAG_SHIFT: u32 = 12;

    /// Create a new instruction cache
    ///
    /// Allocates 1024 cache lines, all initially invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let cache = InstructionCache::new();
    /// assert_eq!(cache.len(), 0); // No valid entries
    /// ```
    pub fn new() -> Self {
        Self {
            lines: vec![CacheLine::new(); Self::LINE_COUNT],
        }
    }

    /// Extract cache line index from address
    ///
    /// Takes bits [11:2] of the address to select one of 1024 cache lines.
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address (should be word-aligned)
    ///
    /// # Returns
    ///
    /// Cache line index (0-1023)
    #[inline(always)]
    fn index(&self, addr: u32) -> usize {
        ((addr >> Self::INDEX_SHIFT) & Self::INDEX_MASK) as usize
    }

    /// Extract tag from address
    ///
    /// Takes bits [31:12] of the address for tag comparison.
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address
    ///
    /// # Returns
    ///
    /// Tag value (upper 20 bits)
    #[inline(always)]
    fn tag(&self, addr: u32) -> u32 {
        addr >> Self::TAG_SHIFT
    }

    /// Fetch instruction from cache
    ///
    /// Performs a cache lookup using direct-mapped addressing.
    /// Returns the cached instruction if:
    /// - The cache line is valid
    /// - The tag matches
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address to fetch
    ///
    /// # Returns
    ///
    /// - `Some(instruction)` if cache hit
    /// - `None` if cache miss
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    ///
    /// // Cache miss
    /// assert_eq!(cache.fetch(0x80000000), None);
    ///
    /// // Store and fetch
    /// cache.store(0x80000000, 0x00000000); // nop
    /// assert_eq!(cache.fetch(0x80000000), Some(0x00000000));
    /// ```
    #[inline(always)]
    pub fn fetch(&self, addr: u32) -> Option<u32> {
        let index = self.index(addr);
        let tag = self.tag(addr);

        let line = &self.lines[index];
        if line.valid && line.tag == tag {
            Some(line.data)
        } else {
            None
        }
    }

    /// Store instruction in cache
    ///
    /// Stores an instruction in the cache using direct-mapped addressing.
    /// If another instruction already occupies this cache line, it is evicted.
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address
    /// * `instruction` - 32-bit instruction word
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// cache.store(0x80000000, 0x3C080000); // lui r8, 0x0000
    ///
    /// // Storing to same index with different tag evicts previous entry
    /// cache.store(0x80001000, 0x24080001); // addiu r8, r0, 1
    /// assert_eq!(cache.fetch(0x80000000), None); // Evicted
    /// assert_eq!(cache.fetch(0x80001000), Some(0x24080001)); // New entry
    /// ```
    #[inline(always)]
    pub fn store(&mut self, addr: u32, instruction: u32) {
        let index = self.index(addr);
        let tag = self.tag(addr);

        self.lines[index] = CacheLine {
            tag,
            data: instruction,
            valid: true,
        };
    }

    /// Invalidate cached instruction at given address
    ///
    /// Marks the cache line as invalid. This is essential for cache coherency
    /// when memory is modified after caching (self-modifying code, DMA, etc.).
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address to invalidate
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// cache.store(0x80000000, 0x00000000);
    ///
    /// cache.invalidate(0x80000000);
    /// assert_eq!(cache.fetch(0x80000000), None);
    /// ```
    #[inline(always)]
    pub fn invalidate(&mut self, addr: u32) {
        let index = self.index(addr);
        let tag = self.tag(addr);

        let line = &mut self.lines[index];
        if line.valid && line.tag == tag {
            line.valid = false;
        }
    }

    /// Invalidate cached instructions in given address range
    ///
    /// More efficient than individual invalidations when a large memory
    /// region is modified (e.g., DMA transfer, memset operations).
    ///
    /// # Arguments
    ///
    /// * `start` - Start address (inclusive)
    /// * `end` - End address (inclusive)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// cache.store(0x80000000, 0x00000000);
    /// cache.store(0x80000004, 0x00000000);
    /// cache.store(0x80000008, 0x00000000);
    ///
    /// // Invalidate first two instructions
    /// cache.invalidate_range(0x80000000, 0x80000004);
    ///
    /// assert_eq!(cache.fetch(0x80000000), None);
    /// assert_eq!(cache.fetch(0x80000004), None);
    /// assert_eq!(cache.fetch(0x80000008), Some(0x00000000)); // Still valid
    /// ```
    pub fn invalidate_range(&mut self, start: u32, end: u32) {
        if start > end {
            return;
        }

        // Align both bounds to 4-byte word addresses
        let mut addr = start & !0x3;
        let end_aligned = end & !0x3;

        loop {
            if addr > end_aligned {
                break;
            }

            let index = self.index(addr);
            let tag = self.tag(addr);

            let line = &mut self.lines[index];
            if line.valid && line.tag == tag {
                line.valid = false;
            }

            if addr == end_aligned {
                break;
            }
            addr = addr.wrapping_add(4);
        }
    }

    /// Prefill cache with instruction at given address
    ///
    /// This is an alias for `store()`, used when memory writes occur to known
    /// code regions, allowing us to cache instructions before execution
    /// (mimicking how real hardware caches instructions during BIOS copy operations).
    ///
    /// # Arguments
    ///
    /// * `addr` - Instruction address
    /// * `instruction` - 32-bit instruction word
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// // Prefill cache when BIOS copies code to RAM
    /// cache.prefill(0x80000500, 0x3C080000); // lui r8, 0x0000
    /// assert_eq!(cache.fetch(0x80000500), Some(0x3C080000));
    /// ```
    #[inline(always)]
    pub fn prefill(&mut self, addr: u32, instruction: u32) {
        self.store(addr, instruction);
    }

    /// Clear all cached instructions
    ///
    /// Invalidates all cache lines. This is faster than individual invalidation
    /// when resetting the entire cache.
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// cache.store(0x80000000, 0x00000000);
    /// cache.store(0x80000004, 0x00000000);
    ///
    /// cache.clear();
    ///
    /// assert_eq!(cache.fetch(0x80000000), None);
    /// assert_eq!(cache.fetch(0x80000004), None);
    /// assert_eq!(cache.len(), 0);
    /// ```
    pub fn clear(&mut self) {
        for line in &mut self.lines {
            line.valid = false;
        }
    }

    /// Check if cache is empty
    ///
    /// Returns true if no valid cache entries exist.
    ///
    /// # Returns
    ///
    /// `true` if cache is empty, `false` otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// assert!(cache.is_empty());
    ///
    /// cache.store(0x80000000, 0x00000000);
    /// assert!(!cache.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|line| !line.valid)
    }

    /// Get number of valid cached entries
    ///
    /// Counts how many cache lines contain valid data.
    ///
    /// # Returns
    ///
    /// Number of valid cache entries (0-1024)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// assert_eq!(cache.len(), 0);
    ///
    /// cache.store(0x80000000, 0x00000000);
    /// cache.store(0x80000004, 0x00000000);
    /// assert_eq!(cache.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.lines.iter().filter(|line| line.valid).count()
    }

    /// Get cache hit rate statistics
    ///
    /// Returns the percentage of cache lines that are valid.
    /// This can be used to monitor cache effectiveness.
    ///
    /// # Returns
    ///
    /// Cache occupancy as a percentage (0.0-100.0)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::cpu::icache::InstructionCache;
    ///
    /// let mut cache = InstructionCache::new();
    /// cache.store(0x80000000, 0x00000000);
    ///
    /// let occupancy = cache.occupancy();
    /// assert!(occupancy > 0.0 && occupancy <= 100.0);
    /// ```
    pub fn occupancy(&self) -> f64 {
        (self.len() as f64 / Self::LINE_COUNT as f64) * 100.0
    }
}

impl Default for InstructionCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = InstructionCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_store_fetch() {
        let mut cache = InstructionCache::new();

        // Store instruction
        cache.store(0x80000000, 0x3C080000);

        // Fetch should return the stored instruction
        assert_eq!(cache.fetch(0x80000000), Some(0x3C080000));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = InstructionCache::new();

        // Fetch from empty cache should return None
        assert_eq!(cache.fetch(0x80000000), None);
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = InstructionCache::new();

        cache.store(0x80000000, 0x3C080000);
        assert_eq!(cache.fetch(0x80000000), Some(0x3C080000));

        cache.invalidate(0x80000000);
        assert_eq!(cache.fetch(0x80000000), None);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_invalidate_range() {
        let mut cache = InstructionCache::new();

        // Store multiple instructions
        cache.store(0x80000000, 0x00000000);
        cache.store(0x80000004, 0x00000000);
        cache.store(0x80000008, 0x00000000);

        // Invalidate first two
        cache.invalidate_range(0x80000000, 0x80000004);

        assert_eq!(cache.fetch(0x80000000), None);
        assert_eq!(cache.fetch(0x80000004), None);
        assert_eq!(cache.fetch(0x80000008), Some(0x00000000));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = InstructionCache::new();

        cache.store(0x80000000, 0x00000000);
        cache.store(0x80000004, 0x00000000);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_direct_mapped_eviction() {
        let mut cache = InstructionCache::new();

        // Store instruction at 0x80000000
        cache.store(0x80000000, 0x11111111);
        assert_eq!(cache.fetch(0x80000000), Some(0x11111111));

        // Store at address that maps to same cache line (different tag)
        // Address 0x80001000 has same index [11:2] but different tag [31:12]
        cache.store(0x80001000, 0x22222222);

        // First instruction should be evicted
        assert_eq!(cache.fetch(0x80000000), None);
        // Second instruction should be present
        assert_eq!(cache.fetch(0x80001000), Some(0x22222222));
    }

    #[test]
    fn test_sequential_access() {
        let mut cache = InstructionCache::new();

        // Store sequential instructions
        for i in 0..10 {
            let addr = 0x80000000 + i * 4;
            cache.store(addr, i);
        }

        // All should be fetchable
        for i in 0..10 {
            let addr = 0x80000000 + i * 4;
            assert_eq!(cache.fetch(addr), Some(i));
        }

        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn test_prefill() {
        let mut cache = InstructionCache::new();

        cache.prefill(0x80000500, 0x3C080000);
        assert_eq!(cache.fetch(0x80000500), Some(0x3C080000));
    }

    #[test]
    fn test_occupancy() {
        let mut cache = InstructionCache::new();

        assert_eq!(cache.occupancy(), 0.0);

        // Fill half the cache
        for i in 0..512 {
            cache.store(0x80000000 + i * 4, 0x00000000);
        }

        // Should be approximately 50%
        let occ = cache.occupancy();
        assert!((49.0..=51.0).contains(&occ));
    }

    #[test]
    fn test_index_extraction() {
        let cache = InstructionCache::new();

        // Test index extraction
        assert_eq!(cache.index(0x80000000), 0);
        assert_eq!(cache.index(0x80000004), 1);
        assert_eq!(cache.index(0x80000008), 2);

        // Test wrapping (address with same lower 12 bits)
        assert_eq!(cache.index(0x80000000), cache.index(0x80001000));
    }

    #[test]
    fn test_tag_extraction() {
        let cache = InstructionCache::new();

        // Test tag extraction
        assert_eq!(cache.tag(0x80000000), 0x80000);
        assert_eq!(cache.tag(0x80001000), 0x80001);
        assert_eq!(cache.tag(0xBFC00000), 0xBFC00);
    }

    #[test]
    fn test_cache_aliasing() {
        let mut cache = InstructionCache::new();

        // Two addresses with same index but different tags
        let addr1 = 0x80000100; // tag=0x80000, index=64
        let addr2 = 0x80001100; // tag=0x80001, index=64

        cache.store(addr1, 0xAAAAAAAA);
        assert_eq!(cache.fetch(addr1), Some(0xAAAAAAAA));

        // Store to same index with different tag
        cache.store(addr2, 0xBBBBBBBB);

        // First should be evicted
        assert_eq!(cache.fetch(addr1), None);
        assert_eq!(cache.fetch(addr2), Some(0xBBBBBBBB));
    }

    #[test]
    fn test_large_range_invalidation() {
        let mut cache = InstructionCache::new();

        // Fill cache with instructions
        for i in 0..100 {
            cache.store(0x80000000 + i * 4, i);
        }

        assert_eq!(cache.len(), 100);

        // Invalidate large range
        cache.invalidate_range(0x80000000, 0x80000100);

        // Count remaining valid entries
        let remaining = cache.len();
        assert!(remaining < 100);
    }
}
