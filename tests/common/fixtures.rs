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

//! Test fixtures for common test scenarios

use psrx::core::cpu::CPU;
use psrx::core::memory::Bus;
use psrx::core::system::System;

/// Create a CPU with default memory bus for testing
#[allow(dead_code)]
pub fn create_cpu_with_bus() -> (CPU, Bus) {
    let cpu = CPU::new();
    let bus = Bus::new();
    (cpu, bus)
}

/// Create a System with initialized components
#[allow(dead_code)]
pub fn create_test_system() -> System {
    System::new()
}

/// Create a System and reset it
#[allow(dead_code)]
pub fn create_and_reset_system() -> System {
    let mut system = System::new();
    system.reset();
    system
}

/// Load a test program into memory at specified address
#[allow(dead_code)]
pub fn load_test_program(bus: &mut Bus, start_addr: u32, program: &[u32]) {
    for (i, &instruction) in program.iter().enumerate() {
        let addr = start_addr + (i as u32 * 4);
        bus.write32(addr, instruction)
            .expect("Failed to write to memory");
    }
}

/// Execute N CPU instructions and return final state
#[allow(dead_code)]
pub fn execute_n_instructions(cpu: &mut CPU, bus: &mut Bus, n: usize) {
    for _ in 0..n {
        let _ = cpu.step(bus);
    }
}
