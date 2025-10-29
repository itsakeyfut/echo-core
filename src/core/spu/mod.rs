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

//! SPU (Sound Processing Unit) implementation
//!
//! This module will contain the PSX SPU emulation in Phase 4.
//! For now, it's a stub implementation.

/// SPU (Sound Processing Unit)
///
/// # TODO
/// - Implement in Phase 4
/// - Voice management (24 voices)
/// - ADPCM decoding
/// - Reverb
/// - Audio output
pub struct SPU {
    // TODO: implement in Phase 4
}

impl SPU {
    /// Create a new SPU instance
    ///
    /// # Returns
    /// Initialized SPU instance
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SPU {
    fn default() -> Self {
        Self::new()
    }
}
