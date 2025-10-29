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

/// Coprocessor 0 (System Control)
///
/// COP0 is the system control unit responsible for exception handling,
/// status management, cache control, and other system functions.
pub(super) struct COP0 {
    /// COP0 registers (32 registers)
    pub(super) regs: [u32; 32],
}

impl COP0 {
    /// Breakpoint PC
    pub const BPC: usize = 3;
    /// Breakpoint Data Address
    pub const BDA: usize = 5;
    /// Target Address
    pub const TAR: usize = 6;
    /// Cache control
    pub const DCIC: usize = 7;
    /// Bad Virtual Address
    pub const BADA: usize = 8;
    /// Data Address Mask
    pub const BDAM: usize = 9;
    /// PC Mask
    pub const BPCM: usize = 11;
    /// Status Register
    pub const SR: usize = 12;
    /// Cause Register
    pub const CAUSE: usize = 13;
    /// Exception PC
    pub const EPC: usize = 14;
    /// Processor ID
    pub const PRID: usize = 15;

    /// Create a new COP0 instance
    ///
    /// # Returns
    /// Initialized COP0 instance with reset values
    pub(super) fn new() -> Self {
        let mut regs = [0u32; 32];
        // Status Register initial value
        regs[Self::SR] = 0x10900000;
        // Processor ID (R3000A identifier)
        regs[Self::PRID] = 0x00000002;

        Self { regs }
    }

    /// Reset COP0 registers to initial state
    pub(super) fn reset(&mut self) {
        self.regs = [0u32; 32];
        self.regs[Self::SR] = 0x10900000;
        self.regs[Self::PRID] = 0x00000002;
    }
}

/// Exception cause codes for MIPS R3000A
///
/// These correspond to the exception codes stored in the CAUSE register
/// when a CPU exception occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExceptionCause {
    /// Interrupt (external or internal)
    Interrupt = 0,
    /// Address error on load
    AddressErrorLoad = 4,
    /// Address error on store
    AddressErrorStore = 5,
    /// Bus error on instruction fetch
    BusErrorInstruction = 6,
    /// Bus error on data access
    BusErrorData = 7,
    /// Syscall instruction executed
    Syscall = 8,
    /// Breakpoint instruction executed
    Breakpoint = 9,
    /// Reserved or illegal instruction
    ReservedInstruction = 10,
    /// Coprocessor unusable
    CoprocessorUnusable = 11,
    /// Arithmetic overflow
    Overflow = 12,
}
