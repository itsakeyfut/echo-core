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

use echo_core::core::error::Result;
use echo_core::core::system::System;

#[test]
fn test_basic_initialization() -> Result<()> {
    // Basic smoke test
    let system = System::new();
    assert_eq!(system.total_cycles(), 0);
    Ok(())
}

#[test]
fn test_system_reset() {
    let mut system = System::new();
    system.reset();
    assert_eq!(system.total_cycles(), 0);
}

#[test]
fn test_cpu_initialization() {
    let system = System::new();
    // PC should start at BIOS entry point
    assert_eq!(system.cpu().pc(), 0xBFC00000);
}
