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

//! GTE transformation operation tests
//!
//! Tests for RTPS, RTPT, and other transformation operations.

use super::super::*;

#[test]
fn test_rtps_identity_matrix() {
    let mut gte = GTE::new();

    // Setup identity rotation matrix (fixed-point 1.0 = 0x1000)
    gte.write_control(GTE::RT11_RT12, 0x00001000); // R11=1.0, R12=0
    gte.write_control(GTE::RT13_RT21, 0x10000000); // R13=0, R21=0
    gte.write_control(GTE::RT22_RT23, 0x00001000); // R22=1.0, R23=0
    gte.write_control(GTE::RT31_RT32, 0x00000000); // R31=0, R32=0
    gte.write_control(GTE::RT33, 0x00001000); // R33=1.0

    // Set translation to zero
    gte.write_control(GTE::TRX, 0);
    gte.write_control(GTE::TRY, 0);
    gte.write_control(GTE::TRZ, 0);

    // Set projection parameters
    gte.write_control(GTE::H, 1000);
    gte.write_control(GTE::OFX, 0);
    gte.write_control(GTE::OFY, 0);

    // Input vector (10, 20, 30)
    gte.write_data(GTE::VXY0, (20 << 16) | (10 & 0xFFFF));
    gte.write_data(GTE::VZ0, 30);

    // Execute RTPS
    gte.rtps(false);

    // Verify MAC values are set (should be transformed coordinates)
    // With identity matrix and zero translation:
    // MAC1 should be proportional to input
    assert_ne!(gte.read_data(GTE::MAC1), 0);
    assert_ne!(gte.read_data(GTE::MAC2), 0);
    assert_ne!(gte.read_data(GTE::MAC3), 0);

    // Screen coordinates should be calculated
    assert_ne!(gte.read_data(GTE::SXY2), 0);
}

#[test]
fn test_nclip_clockwise() {
    let mut gte = GTE::new();

    // Set up clockwise triangle
    // (0,0), (10,0), (5,10)
    gte.write_data(GTE::SXY0, 0); // (0, 0)
    gte.write_data(GTE::SXY1, 10); // (10, 0)
    gte.write_data(GTE::SXY2, (10 << 16) | (5 & 0xFFFF)); // (5, 10)

    gte.nclip();

    let mac0 = gte.read_data(GTE::MAC0);
    // For clockwise winding, MAC0 should be positive
    assert!(mac0 > 0, "MAC0 should be positive for clockwise triangle");
}

#[test]
fn test_nclip_counter_clockwise() {
    let mut gte = GTE::new();

    // Set up counter-clockwise triangle (reversed winding)
    // (0,0), (5,10), (10,0)
    gte.write_data(GTE::SXY0, 0); // (0, 0)
    gte.write_data(GTE::SXY1, (10 << 16) | (5 & 0xFFFF)); // (5, 10)
    gte.write_data(GTE::SXY2, 10); // (10, 0)

    gte.nclip();

    let mac0 = gte.read_data(GTE::MAC0);
    // For counter-clockwise winding, MAC0 should be negative
    assert!(
        mac0 < 0,
        "MAC0 should be negative for counter-clockwise triangle"
    );
}
