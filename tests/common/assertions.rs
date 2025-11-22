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

//! Custom assertions for PSX emulator testing

use psrx::core::cpu::CPU;

/// Assert CPU register has expected value
#[allow(dead_code)]
pub fn assert_cpu_reg(cpu: &CPU, reg: u8, expected: u32) {
    let actual = cpu.reg(reg);
    assert_eq!(
        actual, expected,
        "Register ${} mismatch: expected 0x{:08X}, got 0x{:08X}",
        reg, expected, actual
    );
}

/// Assert CPU PC is at expected address
#[allow(dead_code)]
pub fn assert_cpu_pc(cpu: &CPU, expected: u32) {
    let actual = cpu.pc();
    assert_eq!(
        actual, expected,
        "PC mismatch: expected 0x{:08X}, got 0x{:08X}",
        expected, actual
    );
}

/// Assert memory contains expected value at address
#[allow(dead_code)]
pub fn assert_memory_word(bus: &psrx::core::memory::Bus, addr: u32, expected: u32) {
    let actual = bus.read32(addr).expect("Failed to read memory");
    assert_eq!(
        actual, expected,
        "Memory at 0x{:08X} mismatch: expected 0x{:08X}, got 0x{:08X}",
        addr, expected, actual
    );
}

/// Assert VRAM pixel has expected color
#[allow(dead_code)]
pub fn assert_vram_pixel(gpu: &psrx::core::gpu::GPU, x: u16, y: u16, expected: u16) {
    let actual = gpu.read_vram(x, y);
    assert_eq!(
        actual, expected,
        "VRAM at ({}, {}) mismatch: expected 0x{:04X}, got 0x{:04X}",
        x, y, expected, actual
    );
}
