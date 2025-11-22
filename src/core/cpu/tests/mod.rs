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

//! CPU test modules
//!
//! Tests are organized into the following categories:
//! - `basic`: CPU initialization, reset, register access, PC handling
//! - `load_delay`: Load delay slot behavior
//! - `exceptions`: Exception handling, syscall, break, interrupts
//! - `cop0`: COP0 coprocessor operations (MFC0, MTC0, RFE)
//! - `decode`: Instruction decoding
//! - `instructions`: All instruction execution tests
//! - `timing`: Timing event system integration tests

#[cfg(test)]
mod basic;

#[cfg(test)]
mod load_delay;

#[cfg(test)]
mod exceptions;

#[cfg(test)]
mod cop0;

#[cfg(test)]
mod decode;

#[cfg(test)]
mod instructions;

#[cfg(test)]
mod timing;
