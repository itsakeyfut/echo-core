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

//! Basic GTE functionality tests
//!
//! Tests for initialization, register access, and basic operations.

use super::super::*;

#[test]
fn test_gte_initialization() {
    let gte = GTE::new();
    assert_eq!(gte.read_data(0), 0);
    assert_eq!(gte.read_control(0), 0);
    assert_eq!(gte.flags, 0);
}

#[test]
fn test_data_register_read_write() {
    let mut gte = GTE::new();

    // Write and read data register
    gte.write_data(GTE::MAC1, 0x12345678);
    assert_eq!(gte.read_data(GTE::MAC1), 0x12345678);

    // Test register 31 (LZCR/FLAGS) can be written and read
    gte.write_data(GTE::LZCR, 0xABCDEF00u32 as i32);
    assert_eq!(gte.read_data(GTE::LZCR) as u32, 0xABCDEF00);
}

#[test]
fn test_control_register_read_write() {
    let mut gte = GTE::new();

    // Write and read control register
    gte.write_control(GTE::TRX, 1000);
    assert_eq!(gte.read_control(GTE::TRX), 1000);

    gte.write_control(GTE::H, 2000);
    assert_eq!(gte.read_control(GTE::H), 2000);
}

#[test]
fn test_leading_zero_count() {
    let mut gte = GTE::new();

    // Test positive value
    gte.write_data(GTE::LZCS, 0x00000001);
    assert_eq!(gte.read_data(GTE::LZCR), 31);

    // Test negative value
    gte.write_data(GTE::LZCS, -1);
    assert_eq!(gte.read_data(GTE::LZCR), 0);

    // Test zero
    gte.write_data(GTE::LZCS, 0);
    assert_eq!(gte.read_data(GTE::LZCR), 32);
}

#[test]
fn test_sxy_fifo() {
    let mut gte = GTE::new();

    // Write to SXYP should push to FIFO
    gte.write_data(GTE::SXYP, 100);
    assert_eq!(gte.read_data(GTE::SXY2), 100);
    assert_eq!(gte.read_data(GTE::SXY1), 0);
    assert_eq!(gte.read_data(GTE::SXY0), 0);

    gte.write_data(GTE::SXYP, 200);
    assert_eq!(gte.read_data(GTE::SXY2), 200);
    assert_eq!(gte.read_data(GTE::SXY1), 100);
    assert_eq!(gte.read_data(GTE::SXY0), 0);

    gte.write_data(GTE::SXYP, 300);
    assert_eq!(gte.read_data(GTE::SXY2), 300);
    assert_eq!(gte.read_data(GTE::SXY1), 200);
    assert_eq!(gte.read_data(GTE::SXY0), 100);
}
