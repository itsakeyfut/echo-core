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
use log::info;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("echo-core v{}", env!("CARGO_PKG_VERSION"));
    info!("PlayStation emulator");

    // TODO: Add CLI argument parsing and emulation loop
    // TODO: Load BIOS
    // TODO: Start emulation

    Ok(())
}
