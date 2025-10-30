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

//! CPU execution tracer for debugging
//!
//! Logs CPU execution state to a file for analysis and debugging.

use super::{Disassembler, CPU};
use crate::core::error::Result;
use crate::core::memory::Bus;
use std::fs::File;
use std::io::Write;

/// CPU execution tracer
///
/// Records CPU state and instruction execution to a file for debugging purposes.
/// Each line in the trace file shows:
/// - Program counter
/// - Raw instruction encoding
/// - Disassembled instruction
/// - Values of first few registers
///
/// # Example
/// ```no_run
/// use echo_core::core::cpu::{CPU, CpuTracer};
/// use echo_core::core::memory::Bus;
///
/// let mut cpu = CPU::new();
/// let mut bus = Bus::new();
/// let mut tracer = CpuTracer::new("trace.log").unwrap();
///
/// // Execute and trace
/// tracer.trace(&cpu, &bus).unwrap();
/// cpu.step(&mut bus).unwrap();
/// ```
pub struct CpuTracer {
    /// Enable/disable tracing
    enabled: bool,
    /// Output file handle
    output: File,
}

impl CpuTracer {
    /// Create a new CPU tracer
    ///
    /// Opens a file for writing trace output. If the file exists, it will be overwritten.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the output trace file
    ///
    /// # Returns
    ///
    /// - `Ok(CpuTracer)` if the file was opened successfully
    /// - `Err(EmulatorError)` if file creation fails
    ///
    /// # Example
    /// ```no_run
    /// use echo_core::core::cpu::CpuTracer;
    ///
    /// let tracer = CpuTracer::new("trace.log").unwrap();
    /// ```
    pub fn new(path: &str) -> Result<Self> {
        let output = File::create(path)?;
        Ok(Self {
            enabled: true,
            output,
        })
    }

    /// Enable or disable tracing
    ///
    /// When disabled, trace() calls will return immediately without writing.
    ///
    /// # Arguments
    ///
    /// * `enabled` - true to enable tracing, false to disable
    ///
    /// # Example
    /// ```no_run
    /// use echo_core::core::cpu::CpuTracer;
    ///
    /// let mut tracer = CpuTracer::new("trace.log").unwrap();
    /// tracer.set_enabled(false); // Disable tracing
    /// ```
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if tracing is enabled
    ///
    /// # Returns
    ///
    /// true if tracing is enabled, false otherwise
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Trace current CPU state
    ///
    /// Writes a single line to the trace file containing:
    /// - PC (program counter)
    /// - Raw instruction encoding
    /// - Disassembled instruction
    /// - Selected register values (r1, r2, r3)
    ///
    /// If tracing is disabled, this function returns immediately.
    ///
    /// # Arguments
    ///
    /// * `cpu` - CPU instance to trace
    /// * `bus` - Memory bus for fetching instructions
    ///
    /// # Returns
    ///
    /// - `Ok(())` if trace was written successfully
    /// - `Err(EmulatorError)` if writing fails or memory access fails
    ///
    /// # Example
    /// ```no_run
    /// use echo_core::core::cpu::{CPU, CpuTracer};
    /// use echo_core::core::memory::Bus;
    ///
    /// let cpu = CPU::new();
    /// let bus = Bus::new();
    /// let mut tracer = CpuTracer::new("trace.log").unwrap();
    ///
    /// tracer.trace(&cpu, &bus).unwrap();
    /// ```
    pub fn trace(&mut self, cpu: &CPU, bus: &Bus) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let pc = cpu.pc();
        let instruction = bus.read32(pc)?;
        let disasm = Disassembler::disassemble(instruction, pc);

        writeln!(
            self.output,
            "PC=0x{:08X} [0x{:08X}] {:30} | r1={:08X} r2={:08X} r3={:08X}",
            pc,
            instruction,
            disasm,
            cpu.reg(1),
            cpu.reg(2),
            cpu.reg(3)
        )?;

        Ok(())
    }

    /// Trace with custom register selection
    ///
    /// Like `trace()`, but allows specifying which registers to display.
    ///
    /// # Arguments
    ///
    /// * `cpu` - CPU instance to trace
    /// * `bus` - Memory bus for fetching instructions
    /// * `regs` - Slice of register numbers to display (up to 8 registers)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if trace was written successfully
    /// - `Err(EmulatorError)` if writing fails or memory access fails
    ///
    /// # Example
    /// ```no_run
    /// use echo_core::core::cpu::{CPU, CpuTracer};
    /// use echo_core::core::memory::Bus;
    ///
    /// let cpu = CPU::new();
    /// let bus = Bus::new();
    /// let mut tracer = CpuTracer::new("trace.log").unwrap();
    ///
    /// // Trace with registers 4, 5, 6
    /// tracer.trace_with_regs(&cpu, &bus, &[4, 5, 6]).unwrap();
    /// ```
    pub fn trace_with_regs(&mut self, cpu: &CPU, bus: &Bus, regs: &[u8]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let pc = cpu.pc();
        let instruction = bus.read32(pc)?;
        let disasm = Disassembler::disassemble(instruction, pc);

        write!(
            self.output,
            "PC=0x{:08X} [0x{:08X}] {:30} |",
            pc, instruction, disasm
        )?;

        for &reg in regs.iter().take(8) {
            write!(self.output, " r{}={:08X}", reg, cpu.reg(reg))?;
        }

        writeln!(self.output)?;

        Ok(())
    }

    /// Flush the output buffer
    ///
    /// Forces any buffered trace data to be written to disk.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if flush succeeded
    /// - `Err(EmulatorError)` if flushing fails
    pub fn flush(&mut self) -> Result<()> {
        self.output.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_tracer_creation() {
        let tracer = CpuTracer::new("/tmp/test_trace.log");
        assert!(tracer.is_ok());
    }

    #[test]
    fn test_tracer_enable_disable() {
        let mut tracer = CpuTracer::new("/tmp/test_trace_enable.log").unwrap();
        assert!(tracer.is_enabled());

        tracer.set_enabled(false);
        assert!(!tracer.is_enabled());

        tracer.set_enabled(true);
        assert!(tracer.is_enabled());
    }

    #[test]
    fn test_tracer_basic_trace() {
        let cpu = CPU::new();
        let mut bus = Bus::new();

        // Write a NOP instruction
        bus.write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        let mut tracer = CpuTracer::new("/tmp/test_trace_basic.log").unwrap();
        let result = tracer.trace(&cpu, &bus);
        assert!(result.is_ok());

        tracer.flush().unwrap();

        // Verify trace file contains expected content
        let mut file = File::open("/tmp/test_trace_basic.log").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("PC=0xBFC00000"));
        assert!(contents.contains("nop"));
    }

    #[test]
    fn test_tracer_disabled() {
        let cpu = CPU::new();
        let bus = Bus::new();

        let mut tracer = CpuTracer::new("/tmp/test_trace_disabled.log").unwrap();
        tracer.set_enabled(false);

        // This should succeed but not write anything
        let result = tracer.trace(&cpu, &bus);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tracer_with_custom_regs() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Set some register values
        cpu.set_reg(4, 0x12345678);
        cpu.set_reg(5, 0xABCDEF00);

        // Write a NOP instruction
        bus.write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        let mut tracer = CpuTracer::new("/tmp/test_trace_custom.log").unwrap();
        let result = tracer.trace_with_regs(&cpu, &bus, &[4, 5]);
        assert!(result.is_ok());

        tracer.flush().unwrap();

        // Verify trace file contains custom register values
        let mut file = File::open("/tmp/test_trace_custom.log").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("r4=12345678"));
        assert!(contents.contains("r5=ABCDEF00"));
    }
}
