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

//! ADPCM decoding tests - ADPCM format decoding and filtering

use crate::core::spu::adpcm::ADPCMState;

#[test]
fn test_adpcm_decode_filter_0() {
    let mut state = ADPCMState::default();

    // Create a test block with filter 0 (no filtering)
    let mut block = vec![0u8; 16];
    block[0] = 0x00; // Shift=0, Filter=0
    block[1] = 0x00; // No flags

    // Set some test nibbles
    block[2] = 0x12; // Nibbles: 2, 1
    block[3] = 0x34; // Nibbles: 4, 3

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // With shift=0 and filter=0, samples should be nibbles << 12
    assert_eq!(samples[0], 0x2000); // nibble 2 << 12
    assert_eq!(samples[1], 0x1000); // nibble 1 << 12
}

#[test]
fn test_adpcm_decode_with_shift() {
    let mut state = ADPCMState::default();

    // Create a test block with shift
    let mut block = vec![0u8; 16];
    block[0] = 0x04; // Shift=4, Filter=0
    block[1] = 0x00;
    block[2] = 0xFF; // Nibbles: F (-1), F (-1)

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // Nibble F = -1 (sign extended)
    // (-1 << 12) >> 4 = -4096 >> 4 = -256
    assert_eq!(samples[0], -256);
    assert_eq!(samples[1], -256);
}

#[test]
fn test_adpcm_decode_filter_1() {
    let mut state = ADPCMState::default();

    // Set up previous samples
    state.prev_samples[0] = 100;
    state.prev_samples[1] = 50;

    // Create a test block with filter 1
    let mut block = vec![0u8; 16];
    block[0] = 0x10; // Shift=0, Filter=1
    block[1] = 0x00;
    block[2] = 0x00; // Nibbles: 0, 0

    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 28);

    // Filter 1: sample + old[0] + (-old[0] >> 1)
    // 0 + 100 + (-100 >> 1) = 0 + 100 - 50 = 50
    assert_eq!(samples[0], 50);
}

#[test]
fn test_adpcm_empty_block() {
    let mut state = ADPCMState::default();

    // Try to decode empty block
    let block: Vec<u8> = Vec::new();
    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 0);

    // Try with too-short block
    let block = vec![0u8; 10];
    let samples = state.decode_block(&block);
    assert_eq!(samples.len(), 0);
}
