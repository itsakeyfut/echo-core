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

//! Exception-triggering instructions

use super::super::ExceptionCause;
use super::CPU;
use crate::core::error::Result;

impl CPU {
    /// SYSCALL: System Call
    ///
    /// Triggers a system call exception, transferring control to the
    /// exception handler. This is typically used by user programs to
    /// request operating system services.
    ///
    /// # Arguments
    ///
    /// * `_instruction` - The full 32-bit instruction (unused)
    ///
    /// # Exception
    ///
    /// Always triggers ExceptionCause::Syscall
    ///
    /// # Example
    ///
    /// ```text
    /// SYSCALL  # Trigger system call exception
    /// ```
    pub(in crate::core::cpu) fn op_syscall(&mut self, _instruction: u32) -> Result<()> {
        self.exception(ExceptionCause::Syscall);
        Ok(())
    }

    /// BREAK: Breakpoint
    ///
    /// Triggers a breakpoint exception, transferring control to the
    /// exception handler. This is typically used by debuggers to set
    /// breakpoints in code.
    ///
    /// # Arguments
    ///
    /// * `_instruction` - The full 32-bit instruction (unused)
    ///
    /// # Exception
    ///
    /// Always triggers ExceptionCause::Breakpoint
    ///
    /// # Example
    ///
    /// ```text
    /// BREAK  # Trigger breakpoint exception
    /// ```
    pub(in crate::core::cpu) fn op_break(&mut self, _instruction: u32) -> Result<()> {
        self.exception(ExceptionCause::Breakpoint);
        Ok(())
    }
}
