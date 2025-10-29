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

//! GPU (Graphics Processing Unit) implementation
//!
//! This module will contain the PSX GPU emulation in Phase 2.
//! For now, it's a stub implementation.

/// GPU (Graphics Processing Unit)
///
/// # TODO
/// - Implement in Phase 2
/// - VRAM management
/// - GP0/GP1 command processing
/// - Rasterization
pub struct GPU {
    // TODO: implement in Phase 2
}

impl GPU {
    /// Create a new GPU instance
    ///
    /// # Returns
    /// Initialized GPU instance
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GPU {
    fn default() -> Self {
        Self::new()
    }
}
