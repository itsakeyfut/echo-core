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

//! Reverb effect tests - reverb processing and configuration

use crate::core::spu::reverb::ReverbConfig;
use crate::core::spu::SPU;

#[test]
fn test_reverb_creation() {
    let reverb = ReverbConfig::new();
    assert!(!reverb.enabled);
    assert_eq!(reverb.reverb_current_addr, 0);
}

#[test]
fn test_reverb_disabled_passthrough() {
    let mut reverb = ReverbConfig::new();
    reverb.enabled = false;

    let mut spu_ram = vec![0u8; 512 * 1024];
    let (left, right) = reverb.process(1000, 1000, &mut spu_ram);

    // When disabled, reverb should pass through unchanged
    assert_eq!(left, 1000);
    assert_eq!(right, 1000);
}

#[test]
fn test_reverb_enabled_processing() {
    let mut reverb = ReverbConfig::new();
    reverb.enabled = true;
    reverb.input_volume_left = 0x4000;
    reverb.input_volume_right = 0x4000;

    let mut spu_ram = vec![0u8; 512 * 1024];

    let (_left, _right) = reverb.process(1000, 1000, &mut spu_ram);

    // Reverb should process without crashing
    // Output values are i16, so they're always in valid range
}

#[test]
fn test_reverb_register_writes() {
    let mut spu = SPU::new();

    // Write reverb configuration
    spu.write_register(0x1F801DC0, 0x1234); // APF offset 1
    spu.write_register(0x1F801DC2, 0x5678); // APF offset 2
    spu.write_register(0x1F801DD2, 0x4000); // Input volume left
    spu.write_register(0x1F801DD4, 0x3000); // Input volume right

    assert_eq!(spu.reverb.apf_offset1, 0x1234);
    assert_eq!(spu.reverb.apf_offset2, 0x5678);
    assert_eq!(spu.reverb.input_volume_left, 0x4000);
    assert_eq!(spu.reverb.input_volume_right, 0x3000);
}

#[test]
fn test_spu_with_reverb_enabled() {
    let mut spu = SPU::new();

    // Enable SPU and reverb
    spu.control.enabled = true;
    spu.control.unmute = true;
    spu.write_register(0x1F801DAA, 0xC080); // Enable + unmute + reverb

    assert!(spu.control.reverb_enabled);
    assert!(spu.reverb.enabled);

    // Generate a sample
    let _sample = spu.generate_sample();

    // Should generate without crashing
    // Output values are i16, so they're always in valid range
}
