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

//! PlayStation 1 emulator core library
//!
//! This library provides the core emulation components for a PlayStation 1 emulator,
//! including CPU (MIPS R3000A), memory bus, and other hardware components.
//!
//! # Example
//!
//! ```
//! use echo_core::core::cpu::CPU;
//! use echo_core::core::memory::Bus;
//!
//! let mut cpu = CPU::new();
//! let mut bus = Bus::new();
//!
//! // Execute one instruction
//! // let cycles = cpu.step(&mut bus).unwrap();
//! ```

pub mod core;
