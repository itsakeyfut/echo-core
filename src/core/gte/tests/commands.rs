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

//! GTE command execution tests
//!
//! Tests for the execute() method and command dispatching.

use super::super::*;

#[test]
fn test_execute_rtps_command() {
    let mut gte = GTE::new();

    // Setup basic parameters
    gte.write_control(GTE::RT11_RT12, 0x00001000);
    gte.write_control(GTE::RT22_RT23, 0x00001000);
    gte.write_control(GTE::RT33, 0x00001000);
    gte.write_control(GTE::H, 1000);

    // Input vector
    gte.write_data(GTE::VXY0, (100 << 16) | (50 & 0xFFFF));
    gte.write_data(GTE::VZ0, 200);

    // Execute RTPS via command (opcode 0x01)
    gte.execute(0x01);

    // Verify execution completed (screen coordinates updated)
    assert_ne!(gte.read_data(GTE::SXY2), 0);
}

#[test]
fn test_execute_nclip_command() {
    let mut gte = GTE::new();

    // Setup triangle
    gte.write_data(GTE::SXY0, 0);
    gte.write_data(GTE::SXY1, 10);
    gte.write_data(GTE::SXY2, (10 << 16) | (5 & 0xFFFF));

    // Execute NCLIP via command (opcode 0x06)
    gte.execute(0x06);

    // Verify MAC0 was calculated
    assert_ne!(gte.read_data(GTE::MAC0), 0);
}

#[test]
fn test_execute_unknown_command() {
    let mut gte = GTE::new();

    // Execute unknown command
    gte.execute(0xFF);

    // Should set error flag
    assert_ne!(gte.flags & 0x80000000, 0);
}
