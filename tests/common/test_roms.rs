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

//! Test ROM utilities and small test programs

/// Simple test program: NOP loop
#[allow(dead_code)]
pub fn test_program_nop_loop() -> Vec<u32> {
    vec![
        0x00000000, // NOP
        0x00000000, // NOP
        0x00000000, // NOP
        0x00000000, // NOP
    ]
}

/// Test program: Register arithmetic
#[allow(dead_code)]
pub fn test_program_basic_arithmetic() -> Vec<u32> {
    vec![
        0x24010001, // ADDIU $1, $0, 1      ; $1 = 1
        0x24020002, // ADDIU $2, $0, 2      ; $2 = 2
        0x00221820, // ADD   $3, $1, $2     ; $3 = $1 + $2 = 3
        0x00000000, // NOP
    ]
}

/// Test program: Load/Store operations
#[allow(dead_code)]
pub fn test_program_load_store() -> Vec<u32> {
    vec![
        0x3C011F80, // LUI   $1, 0x1F80     ; $1 = 0x1F800000
        0x240200AA, // ADDIU $2, $0, 0xAA   ; $2 = 0xAA
        0xAC220000, // SW    $2, 0($1)      ; Store to 0x1F800000
        0x8C230000, // LW    $3, 0($1)      ; Load from 0x1F800000
        0x00000000, // NOP
    ]
}

/// Test program: Branch instructions
#[allow(dead_code)]
pub fn test_program_branch() -> Vec<u32> {
    vec![
        0x24010001, // ADDIU $1, $0, 1      ; $1 = 1
        0x24020001, // ADDIU $2, $0, 1      ; $2 = 1
        0x10220001, // BEQ   $1, $2, +1     ; Branch if equal
        0x00000000, // NOP (delay slot)
        0x24030042, // ADDIU $3, $0, 0x42   ; Should be skipped
        0x24040099, // ADDIU $4, $0, 0x99   ; Branch target
    ]
}

/// Get BIOS path from environment or default location
#[allow(dead_code)]
pub fn get_bios_path() -> Option<String> {
    std::env::var("PSX_BIOS_PATH").ok().or_else(|| {
        let default_path = "SCPH1001.BIN";
        if std::path::Path::new(default_path).exists() {
            Some(default_path.to_string())
        } else {
            None
        }
    })
}

/// Check if BIOS is available for testing
#[allow(dead_code)]
pub fn is_bios_available() -> bool {
    get_bios_path().is_some()
}
