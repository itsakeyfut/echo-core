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

//! System integration module
//!
//! This module ties together all emulator components (CPU, Memory, GPU, SPU, Controller)
//! and provides the main emulation loop.

use super::controller::Controller;
use super::cpu::{CpuTracer, CPU};
use super::error::Result;
use super::gpu::GPU;
use super::interrupt::InterruptController;
use super::memory::Bus;
use super::spu::SPU;
use super::timer::Timers;
use std::cell::RefCell;
use std::rc::Rc;

/// PlayStation Controller Port Registers
///
/// Manages the memory-mapped I/O registers for controller communication.
///
/// # Register Map
/// - 0x1F801040: JOY_TX_DATA / JOY_RX_DATA (read/write)
/// - 0x1F801044: JOY_STAT (Status register)
/// - 0x1F801048: JOY_MODE (Mode register)
/// - 0x1F80104A: JOY_CTRL (Control register)
/// - 0x1F80104E: JOY_BAUD (Baud rate)
///
/// # Protocol
/// The controller uses a synchronous serial protocol:
/// 1. Write to JOY_CTRL to select controller
/// 2. Write bytes to JOY_TX_DATA
/// 3. Read responses from JOY_RX_DATA
/// 4. Write to JOY_CTRL to deselect controller
pub struct ControllerPorts {
    /// JOY_TX_DATA (0x1F801040) - Transmit data
    tx_data: u8,

    /// JOY_RX_DATA (0x1F801040) - Receive data (same register)
    rx_data: u8,

    /// JOY_STAT (0x1F801044) - Status register
    stat: u32,

    /// JOY_MODE (0x1F801048) - Mode register
    mode: u16,

    /// JOY_CTRL (0x1F80104A) - Control register
    ctrl: u16,

    /// JOY_BAUD (0x1F80104E) - Baud rate
    baud: u16,

    /// Connected controllers (port 1 and 2)
    controllers: [Option<Controller>; 2],

    /// Currently selected port (0 or 1)
    selected_port: Option<usize>,
}

impl ControllerPorts {
    /// Create new controller ports with default state
    ///
    /// Initializes with one controller connected to port 1.
    pub fn new() -> Self {
        Self {
            tx_data: 0xFF,
            rx_data: 0xFF,
            stat: 0x05, // TX ready (bit 0), RX ready (bit 2)
            mode: 0x000D,
            ctrl: 0,
            baud: 0,
            controllers: [Some(Controller::new()), None], // Port 1 has controller
            selected_port: None,
        }
    }

    /// Write to TX_DATA register (0x1F801040)
    ///
    /// Transmits a byte to the selected controller and receives a response byte.
    ///
    /// # Arguments
    ///
    /// * `value` - Byte to transmit
    pub fn write_tx_data(&mut self, value: u8) {
        self.tx_data = value;

        // If controller is selected, perform transfer
        if let Some(port) = self.selected_port {
            if let Some(controller) = &mut self.controllers[port] {
                self.rx_data = controller.transfer(value);
            } else {
                self.rx_data = 0xFF; // No controller
            }
        } else {
            self.rx_data = 0xFF;
        }

        // Set RX ready flag (bit 1)
        self.stat |= 0x02;
    }

    /// Read from RX_DATA register (0x1F801040)
    ///
    /// Returns the last received byte from the controller.
    ///
    /// # Returns
    ///
    /// Received byte
    pub fn read_rx_data(&mut self) -> u8 {
        // Clear RX ready flag
        self.stat &= !0x02;
        self.rx_data
    }

    /// Write to CTRL register (0x1F80104A)
    ///
    /// Controls controller selection and interrupt acknowledgment.
    ///
    /// # Arguments
    ///
    /// * `value` - Control register value
    pub fn write_ctrl(&mut self, value: u16) {
        self.ctrl = value;

        // Check for controller select (bit 1)
        if (value & 0x0002) != 0 {
            // Determine which port based on DTR bits
            let port = if (value & 0x2000) != 0 { 1 } else { 0 };
            self.selected_port = Some(port);

            if let Some(controller) = &mut self.controllers[port] {
                controller.select();
            }

            log::trace!("Controller port {} selected", port + 1);
        } else {
            // Deselect
            if let Some(port) = self.selected_port {
                if let Some(controller) = &mut self.controllers[port] {
                    controller.deselect();
                }
                log::trace!("Controller port {} deselected", port + 1);
            }
            self.selected_port = None;
        }

        // Acknowledge interrupt (bit 4)
        if (value & 0x0010) != 0 {
            self.stat &= !0x0200; // Clear IRQ flag
        }
    }

    /// Read STAT register (0x1F801044)
    ///
    /// Returns the controller port status.
    ///
    /// # Returns
    ///
    /// Status register value
    #[inline]
    pub fn read_stat(&self) -> u32 {
        self.stat
    }

    /// Read MODE register (0x1F801048)
    ///
    /// # Returns
    ///
    /// Mode register value
    #[inline]
    pub fn read_mode(&self) -> u16 {
        self.mode
    }

    /// Write MODE register (0x1F801048)
    ///
    /// # Arguments
    ///
    /// * `value` - Mode register value
    #[inline]
    pub fn write_mode(&mut self, value: u16) {
        self.mode = value;
    }

    /// Read CTRL register (0x1F80104A)
    ///
    /// # Returns
    ///
    /// Control register value
    #[inline]
    pub fn read_ctrl(&self) -> u16 {
        self.ctrl
    }

    /// Read BAUD register (0x1F80104E)
    ///
    /// # Returns
    ///
    /// Baud rate register value
    #[inline]
    pub fn read_baud(&self) -> u16 {
        self.baud
    }

    /// Write BAUD register (0x1F80104E)
    ///
    /// # Arguments
    ///
    /// * `value` - Baud rate value
    #[inline]
    pub fn write_baud(&mut self, value: u16) {
        self.baud = value;
    }

    /// Get mutable reference to controller at port (0 or 1)
    ///
    /// # Arguments
    ///
    /// * `port` - Port number (0 = port 1, 1 = port 2)
    ///
    /// # Returns
    ///
    /// Optional mutable reference to controller
    pub fn get_controller_mut(&mut self, port: usize) -> Option<&mut Controller> {
        self.controllers.get_mut(port).and_then(|c| c.as_mut())
    }
}

impl Default for ControllerPorts {
    fn default() -> Self {
        Self::new()
    }
}

/// PlayStation System
///
/// Integrates all hardware components and manages the emulation loop.
///
/// # Components
/// - CPU: MIPS R3000A processor
/// - Bus: Memory bus for RAM, BIOS, and I/O
/// - GPU: Graphics processing unit
/// - SPU: Sound processing unit
/// - Controller Ports: Input device interface
/// - Timers: 3 timer/counter channels
///
/// # Example
/// ```no_run
/// use psrx::core::system::System;
///
/// let mut system = System::new();
/// // system.load_bios("path/to/bios.bin")?;
/// // system.run();
/// ```
pub struct System {
    /// CPU instance
    cpu: CPU,
    /// Memory bus
    bus: Bus,
    /// GPU instance (shared via Rc<RefCell> for memory-mapped access)
    gpu: Rc<RefCell<GPU>>,
    /// SPU instance
    spu: SPU,
    /// Controller ports (shared via Rc<RefCell> for memory-mapped access)
    controller_ports: Rc<RefCell<ControllerPorts>>,
    /// Timers (shared via Rc<RefCell> for memory-mapped access)
    timers: Rc<RefCell<Timers>>,
    /// Interrupt controller (shared via Rc<RefCell> for memory-mapped access)
    interrupt_controller: Rc<RefCell<InterruptController>>,
    /// Total cycles executed
    cycles: u64,
    /// Running state
    running: bool,
    /// CPU tracer for debugging (optional)
    tracer: Option<CpuTracer>,
    /// Maximum instructions to trace (0 = unlimited)
    trace_limit: usize,
    /// Number of instructions traced so far
    trace_count: usize,
    /// Cycles at last VBLANK
    last_vblank_cycles: u64,
}

impl System {
    /// Create a new System instance
    ///
    /// Initializes all hardware components to their reset state.
    /// Sets up memory-mapped I/O connections between components.
    ///
    /// # Returns
    /// Initialized System instance
    pub fn new() -> Self {
        // Create GPU wrapped in Rc<RefCell> for shared access
        let gpu = Rc::new(RefCell::new(GPU::new()));

        // Create ControllerPorts wrapped in Rc<RefCell> for shared access
        let controller_ports = Rc::new(RefCell::new(ControllerPorts::new()));

        // Create Timers wrapped in Rc<RefCell> for shared access
        let timers = Rc::new(RefCell::new(Timers::new()));

        // Create Interrupt Controller wrapped in Rc<RefCell> for shared access
        let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));

        // Create bus and connect GPU, ControllerPorts, Timers, and InterruptController for memory-mapped I/O
        let mut bus = Bus::new();
        bus.set_gpu(gpu.clone());
        bus.set_controller_ports(controller_ports.clone());
        bus.set_timers(timers.clone());
        bus.set_interrupt_controller(interrupt_controller.clone());

        Self {
            cpu: CPU::new(),
            bus,
            gpu,
            spu: SPU::new(),
            controller_ports,
            timers,
            interrupt_controller,
            cycles: 0,
            running: false,
            tracer: None,
            trace_limit: 0,
            trace_count: 0,
            last_vblank_cycles: 0,
        }
    }

    /// Load BIOS from file
    ///
    /// Loads a BIOS ROM file into the system. The BIOS must be 512KB in size.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the BIOS file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if BIOS was loaded successfully
    /// - `Err(EmulatorError)` if loading fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.load_bios("SCPH1001.BIN").unwrap();
    /// ```
    pub fn load_bios(&mut self, path: &str) -> Result<()> {
        self.bus.load_bios(path)
    }

    /// Reset the system to initial state
    ///
    /// Resets all components as if the console was power-cycled.
    /// This clears RAM/scratchpad but preserves loaded BIOS.
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.bus.reset();
        self.gpu.borrow_mut().reset();
        self.spu = SPU::new();
        self.cycles = 0;
        self.running = true;
        self.trace_count = 0;
        self.last_vblank_cycles = 0;
    }

    /// Execute one CPU instruction
    ///
    /// Executes a single CPU instruction and ticks the GPU accordingly.
    /// The GPU is synchronized with CPU cycles for accurate emulation.
    ///
    /// # Returns
    /// Number of cycles consumed
    ///
    /// # Errors
    /// Returns error if instruction execution fails
    pub fn step(&mut self) -> Result<u32> {
        // Trace instruction if tracer is enabled
        if let Some(ref mut tracer) = self.tracer {
            // Check if we should still trace
            if self.trace_limit == 0 || self.trace_count < self.trace_limit {
                if let Err(e) = tracer.trace(&self.cpu, &self.bus) {
                    log::warn!("Failed to write trace: {}", e);
                }
                self.trace_count += 1;

                // Flush every 100 instructions to ensure data is written
                if self.trace_count.is_multiple_of(100) {
                    log::debug!("Flushed trace at {} instructions", self.trace_count);
                    let _ = tracer.flush();
                }
            } else if self.trace_count == self.trace_limit {
                log::info!(
                    "Trace limit reached ({} instructions), disabling tracer",
                    self.trace_limit
                );
                // Flush and disable tracer
                let _ = tracer.flush();
                self.trace_count += 1; // Increment to prevent repeated logging
            }
        } else if self.trace_count == 0 {
            // Log once if tracer is not enabled
            static LOGGED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                log::warn!("Tracer is None in step() - tracing not active");
            }
        }

        let cpu_cycles = self.cpu.step(&mut self.bus)?;

        // Tick GPU (synchronized with CPU cycles)
        self.gpu.borrow_mut().tick(cpu_cycles);

        // Tick timers (synchronized with CPU cycles)
        // TODO: Implement proper hblank and vblank signals from GPU
        let timer_irqs = self.timers.borrow_mut().tick(cpu_cycles, false, false);

        // Request timer interrupts
        use super::interrupt::interrupts;
        if timer_irqs[0] {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::TIMER0);
        }
        if timer_irqs[1] {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::TIMER1);
        }
        if timer_irqs[2] {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::TIMER2);
        }

        // TODO: Step SPU in future phases
        // self.spu.step()?;

        self.cycles += cpu_cycles as u64;

        // TODO: VBLANK interrupts disabled temporarily
        // The BIOS needs to set up interrupt handlers before we can safely generate interrupts
        // For now, we'll skip VBLANK generation to let the BIOS complete initialization
        /*
        // Check for VBLANK interrupt (approximately 60 Hz)
        // VBLANK occurs every ~564,480 cycles (33.8688 MHz / 60 Hz)
        const CYCLES_PER_VBLANK: u64 = 564_480;
        if self.cycles - self.last_vblank_cycles >= CYCLES_PER_VBLANK {
            self.last_vblank_cycles = self.cycles;
            // Trigger VBLANK interrupt (interrupt 0, bit 0)
            log::debug!("VBLANK interrupt triggered at cycle {}", self.cycles);
            self.cpu.check_interrupts(0x01);
        }
        */

        Ok(cpu_cycles)
    }

    /// Execute multiple instructions
    ///
    /// Executes exactly `n` instructions unless an error occurs.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of instructions to execute
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all instructions executed successfully
    /// - `Err(EmulatorError)` if any instruction fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.step_n(100).unwrap(); // Execute 100 instructions
    /// ```
    pub fn step_n(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    /// Execute one frame worth of instructions
    ///
    /// The PlayStation CPU runs at approximately 33.8688 MHz.
    /// At 60 fps, one frame requires approximately 564,480 cycles.
    ///
    /// During frame execution, the GPU is ticked alongside the CPU to keep
    /// components synchronized for accurate emulation.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if frame executed successfully
    /// - `Err(EmulatorError)` if execution fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.reset();
    /// system.run_frame().unwrap(); // Execute one frame
    /// ```
    pub fn run_frame(&mut self) -> Result<()> {
        // PSX CPU runs at ~33.8688 MHz
        // At 60 fps, one frame = 33868800 / 60 ≈ 564,480 cycles
        const CYCLES_PER_FRAME: u64 = 564_480;
        let target_cycles = self.cycles + CYCLES_PER_FRAME;

        while self.cycles < target_cycles && self.running {
            // Execute CPU instruction (via step() to enable tracing)
            self.step()?;
        }

        Ok(())
    }

    /// Get current PC value
    ///
    /// # Returns
    /// Current program counter value
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::system::System;
    ///
    /// let system = System::new();
    /// assert_eq!(system.pc(), 0xBFC00000);
    /// ```
    pub fn pc(&self) -> u32 {
        self.cpu.pc()
    }

    /// Get total cycles executed
    ///
    /// # Returns
    /// Total number of cycles since reset
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::system::System;
    ///
    /// let system = System::new();
    /// assert_eq!(system.cycles(), 0);
    /// ```
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Get reference to CPU
    ///
    /// # Returns
    /// Reference to CPU instance
    pub fn cpu(&self) -> &CPU {
        &self.cpu
    }

    /// Get mutable reference to CPU
    ///
    /// # Returns
    /// Mutable reference to CPU instance
    pub fn cpu_mut(&mut self) -> &mut CPU {
        &mut self.cpu
    }

    /// Get reference to memory bus
    ///
    /// # Returns
    /// Reference to Bus instance
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Get mutable reference to memory bus
    ///
    /// # Returns
    /// Mutable reference to Bus instance
    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    /// Get reference to GPU
    ///
    /// # Returns
    /// Reference to GPU instance (wrapped in Rc<RefCell>)
    pub fn gpu(&self) -> Rc<RefCell<GPU>> {
        Rc::clone(&self.gpu)
    }

    /// Get reference to Controller Ports
    ///
    /// # Returns
    /// Reference to ControllerPorts instance (wrapped in Rc<RefCell>)
    pub fn controller_ports(&self) -> Rc<RefCell<ControllerPorts>> {
        Rc::clone(&self.controller_ports)
    }

    /// Enable CPU execution tracing to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the trace file to write
    /// * `limit` - Maximum number of instructions to trace (0 = unlimited)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if tracing was enabled successfully
    /// - `Err(EmulatorError)` if file creation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.enable_tracing("trace.log", 5000).unwrap(); // Trace first 5000 instructions
    /// ```
    pub fn enable_tracing(&mut self, path: &str, limit: usize) -> Result<()> {
        self.tracer = Some(CpuTracer::new(path)?);
        self.trace_limit = limit;
        self.trace_count = 0;
        log::info!(
            "CPU tracing enabled: {} (limit: {})",
            path,
            if limit == 0 {
                "unlimited".to_string()
            } else {
                limit.to_string()
            }
        );
        Ok(())
    }

    /// Disable CPU execution tracing
    ///
    /// Closes the trace file and disables tracing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.enable_tracing("trace.log", 1000).unwrap();
    /// // ... run emulation ...
    /// system.disable_tracing();
    /// ```
    pub fn disable_tracing(&mut self) {
        if self.tracer.is_some() {
            log::info!(
                "CPU tracing disabled (traced {} instructions)",
                self.trace_count
            );
            self.tracer = None;
            self.trace_limit = 0;
            self.trace_count = 0;
        }
    }

    /// Check if tracing is currently enabled
    ///
    /// # Returns
    /// true if tracing is active
    pub fn is_tracing(&self) -> bool {
        self.tracer.is_some()
    }

    /// Get the number of instructions traced so far
    ///
    /// # Returns
    /// Number of instructions traced
    pub fn trace_count(&self) -> usize {
        self.trace_count
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_initialization() {
        let system = System::new();
        assert_eq!(system.cycles(), 0);
        assert_eq!(system.pc(), 0xBFC00000);
    }

    #[test]
    fn test_system_step() {
        let mut system = System::new();

        // Write NOP instruction directly to BIOS memory for testing
        // NOP = 0x00000000
        system
            .bus_mut()
            .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        let initial_pc = system.pc();
        system.step().unwrap();

        assert_eq!(system.pc(), initial_pc + 4);
        assert_eq!(system.cycles(), 1);
    }

    #[test]
    fn test_system_step_n() {
        let mut system = System::new();

        // Fill BIOS with NOPs for testing
        for i in 0..10 {
            let offset = (i * 4) as usize;
            system
                .bus_mut()
                .write_bios_for_test(offset, &[0x00, 0x00, 0x00, 0x00]);
        }

        system.step_n(10).unwrap();

        assert_eq!(system.cycles(), 10);
    }

    #[test]
    fn test_system_reset() {
        let mut system = System::new();

        // Setup BIOS with NOP for testing
        system
            .bus_mut()
            .write_bios_for_test(0, &[0x00, 0x00, 0x00, 0x00]);

        // Execute some instructions to change state
        system.step().unwrap();
        system.step().unwrap();

        assert!(system.cycles() > 0);

        system.reset();
        assert_eq!(system.cycles(), 0);
        assert_eq!(system.pc(), 0xBFC00000);
        assert!(system.running);
    }

    #[test]
    fn test_system_run_frame() {
        let mut system = System::new();

        // Create an infinite loop in BIOS for testing:
        // 0xBFC00000: j 0xBFC00000  (jump to self)
        // Encoding: opcode=2 (J), target=0x0F000000 (0xBFC00000 >> 2)
        // Full instruction: 0x0BF00000
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);

        // 0xBFC00004: nop (delay slot)
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();
        let initial_cycles = system.cycles();

        system.run_frame().unwrap();

        // Should execute approximately one frame worth of cycles (564,480)
        let cycles_executed = system.cycles() - initial_cycles;
        assert!(cycles_executed >= 564_480);
    }

    #[test]
    fn test_system_pc_accessor() {
        let system = System::new();
        assert_eq!(system.pc(), 0xBFC00000);
    }

    #[test]
    fn test_system_cycles_accessor() {
        let system = System::new();
        assert_eq!(system.cycles(), 0);
    }

    // GPU-Bus Integration Tests

    #[test]
    fn test_gpu_register_mapping() {
        let mut system = System::new();

        // Write to GP0 (0x1F801810)
        system.bus.write32(0x1F801810, 0xA0000000).unwrap();

        // Write to GP1 (0x1F801814)
        system.bus.write32(0x1F801814, 0x03000000).unwrap();

        // Read GPUSTAT (0x1F801814)
        let status = system.bus.read32(0x1F801814).unwrap();
        // Display should be enabled (bit 23 should be 0)
        assert_eq!(status & (1 << 23), 0);
    }

    #[test]
    fn test_gpustat_read() {
        let system = System::new();

        // Read GPU status register
        let status = system.bus.read32(0x1F801814).unwrap();

        // Status register should have valid format
        // Initially display should be disabled (bit 23 = 1)
        assert_ne!(status & (1 << 23), 0);

        // Ready flags should be set (bits 26, 27, 28)
        assert_ne!(status & (1 << 26), 0); // Ready to receive command
        assert_ne!(status & (1 << 27), 0); // Ready to send VRAM
        assert_ne!(status & (1 << 28), 0); // Ready to receive DMA
    }

    #[test]
    fn test_gpuread() {
        let mut system = System::new();

        // Setup VRAM with test data via direct GPU access
        system.gpu.borrow_mut().write_vram(100, 100, 0x1234);
        system.gpu.borrow_mut().write_vram(101, 100, 0x5678);

        // Setup VRAM→CPU transfer via GP0
        system.bus.write32(0x1F801810, 0xC0000000).unwrap(); // Command
        system.bus.write32(0x1F801810, 0x00640064).unwrap(); // Position (100, 100)
        system.bus.write32(0x1F801810, 0x00010002).unwrap(); // Size 2×1

        // Read data via GPUREAD
        let data = system.bus.read32(0x1F801810).unwrap();
        assert_eq!(data & 0xFFFF, 0x1234);
        assert_eq!((data >> 16) & 0xFFFF, 0x5678);
    }

    #[test]
    fn test_system_gpu_integration() {
        let mut system = System::new();

        // Run for a few cycles
        for _ in 0..100 {
            let _ = system.step();
        }

        // System should not crash
        assert!(system.cycles() >= 100);
    }

    #[test]
    fn test_run_frame_ticks_gpu() {
        let mut system = System::new();

        // Create an infinite loop in BIOS for testing:
        // 0xBFC00000: j 0xBFC00000  (jump to self)
        // Encoding: opcode=2 (J), target=0x0F000000 (0xBFC00000 >> 2)
        // Full instruction: 0x0BF00000
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);

        // 0xBFC00004: nop (delay slot)
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();
        let initial_cycles = system.cycles();

        // Run one frame
        system.run_frame().unwrap();

        // Should execute approximately one frame worth of cycles (564,480)
        let cycles_executed = system.cycles() - initial_cycles;
        assert!(cycles_executed >= 564_480);
    }

    #[test]
    fn test_gp0_command_via_bus() {
        let mut system = System::new();

        // Send CPU→VRAM transfer command via bus
        system.bus.write32(0x1F801810, 0xA0000000).unwrap(); // GP0 command
        system.bus.write32(0x1F801810, 0x00000000).unwrap(); // Position (0, 0)
        system.bus.write32(0x1F801810, 0x00010001).unwrap(); // Size 1×1

        // Write pixel data
        system.bus.write32(0x1F801810, 0x7FFF7FFF).unwrap();

        // Verify pixel was written to VRAM
        assert_eq!(system.gpu.borrow().read_vram(0, 0), 0x7FFF);
    }

    #[test]
    fn test_gp1_command_via_bus() {
        let mut system = System::new();

        // Initially display should be disabled
        let status_before = system.bus.read32(0x1F801814).unwrap();
        assert_ne!(status_before & (1 << 23), 0);

        // Enable display via GP1
        system.bus.write32(0x1F801814, 0x03000000).unwrap();

        // Display should now be enabled
        let status_after = system.bus.read32(0x1F801814).unwrap();
        assert_eq!(status_after & (1 << 23), 0);
    }

    #[test]
    fn test_gpu_reset_via_gp1() {
        let mut system = System::new();

        // Enable display
        system.bus.write32(0x1F801814, 0x03000000).unwrap();
        let status_enabled = system.bus.read32(0x1F801814).unwrap();
        assert_eq!(status_enabled & (1 << 23), 0);

        // Reset GPU via GP1(0x00)
        system.bus.write32(0x1F801814, 0x00000000).unwrap();

        // Display should be disabled again after reset
        let status_reset = system.bus.read32(0x1F801814).unwrap();
        assert_ne!(status_reset & (1 << 23), 0);
    }

    #[test]
    fn test_vram_transfer_via_bus() {
        let mut system = System::new();

        // Start CPU→VRAM transfer
        system.bus.write32(0x1F801810, 0xA0000000).unwrap();
        system.bus.write32(0x1F801810, 0x000A000A).unwrap(); // Position (10, 10)
        system.bus.write32(0x1F801810, 0x00020002).unwrap(); // Size 2×2

        // Write 2 u32 words (4 pixels)
        system.bus.write32(0x1F801810, 0xAAAABBBB).unwrap();
        system.bus.write32(0x1F801810, 0xCCCCDDDD).unwrap();

        // Verify pixels written correctly
        assert_eq!(system.gpu.borrow().read_vram(10, 10), 0xBBBB);
        assert_eq!(system.gpu.borrow().read_vram(11, 10), 0xAAAA);
        assert_eq!(system.gpu.borrow().read_vram(10, 11), 0xDDDD);
        assert_eq!(system.gpu.borrow().read_vram(11, 11), 0xCCCC);
    }

    #[test]
    fn test_gpu_memory_mirroring() {
        let mut system = System::new();

        // Test that GPU registers are accessible via different segments

        // Write via KUSEG
        system.bus.write32(0x1F801814, 0x03000000).unwrap();
        let status1 = system.bus.read32(0x1F801814).unwrap();

        // Read via KSEG0
        let status2 = system.bus.read32(0x9F801814).unwrap();

        // Read via KSEG1
        let status3 = system.bus.read32(0xBF801814).unwrap();

        // All should return the same value
        assert_eq!(status1, status2);
        assert_eq!(status2, status3);
    }

    // Controller Port Tests

    #[test]
    fn test_controller_ports_initialization() {
        let system = System::new();

        // Controller port 1 should have a controller
        assert!(system
            .controller_ports()
            .borrow_mut()
            .get_controller_mut(0)
            .is_some());
    }

    #[test]
    fn test_controller_ports_select() {
        let mut ports = ControllerPorts::new();

        // Select port 1
        ports.write_ctrl(0x0002); // SELECT bit

        // Transfer data
        ports.write_tx_data(0x01);
        assert_eq!(ports.read_rx_data(), 0xFF);

        ports.write_tx_data(0x42);
        assert_eq!(ports.read_rx_data(), 0x41); // Digital pad ID
    }

    #[test]
    fn test_controller_ports_button_state() {
        let system = System::new();

        // Press a button on port 1
        let controller_ports = system.controller_ports();
        let mut ports_borrow = controller_ports.borrow_mut();
        if let Some(controller) = ports_borrow.get_controller_mut(0) {
            use crate::core::controller::buttons;
            controller.press_button(buttons::CROSS);
            assert_eq!(controller.get_buttons() & buttons::CROSS, 0);
        }
    }

    #[test]
    #[ignore] // Requires actual BIOS file - run with: cargo test -- --ignored
    fn test_bios_boot() {
        // This test requires an actual PSX BIOS file.
        // Place your BIOS file (e.g., SCPH1001.BIN) in the project root or specify the path.
        //
        // To run this test:
        //   cargo test test_bios_boot -- --ignored --nocapture
        //
        // Note: You must legally own a PlayStation console to use its BIOS.

        let bios_path =
            std::env::var("PSX_BIOS_PATH").unwrap_or_else(|_| "SCPH1001.BIN".to_string());

        let mut system = System::new();

        // Load actual PSX BIOS
        match system.load_bios(&bios_path) {
            Ok(_) => println!("BIOS loaded successfully from: {}", bios_path),
            Err(e) => {
                println!("Failed to load BIOS: {}", e);
                println!("Set PSX_BIOS_PATH environment variable or place BIOS in project root");
                panic!("BIOS file not found");
            }
        }

        system.reset();

        println!("Starting BIOS execution test...");
        println!("Initial PC: 0x{:08X}", system.pc());

        // Execute first 10,000 instructions
        const TEST_INSTRUCTIONS: usize = 10_000;
        for i in 0..TEST_INSTRUCTIONS {
            if i % 1000 == 0 && i > 0 {
                println!(
                    "Progress: {}/{} | PC: 0x{:08X} | Cycles: {}",
                    i,
                    TEST_INSTRUCTIONS,
                    system.pc(),
                    system.cycles()
                );
            }

            match system.step() {
                Ok(_) => {}
                Err(e) => {
                    println!("Error at PC=0x{:08X}: {}", system.pc(), e);
                    println!("Instruction count: {}", i);
                    system.cpu().dump_registers();
                    panic!("BIOS boot failed");
                }
            }
        }

        // If we got here, BIOS is executing successfully
        println!();
        println!("BIOS boot test completed successfully!");
        println!("Executed {} instructions", TEST_INSTRUCTIONS);
        println!("Total cycles: {}", system.cycles());
        println!("Final PC: 0x{:08X}", system.pc());

        // Basic sanity checks
        assert!(system.cycles() >= TEST_INSTRUCTIONS as u64);
        // PC should have moved from initial BIOS entry point
        assert_ne!(system.pc(), 0xBFC00000);
    }

    // Interrupt Controller Integration Tests

    #[test]
    fn test_interrupt_controller_registers() {
        let mut system = System::new();

        // Write to I_MASK register via bus
        system.bus.write32(0x1F801074, 0x00FF).unwrap();

        // Read back I_MASK
        let mask = system.bus.read32(0x1F801074).unwrap();
        assert_eq!(mask, 0x00FF);

        // Read I_STAT (should be 0 initially)
        let status = system.bus.read32(0x1F801070).unwrap();
        assert_eq!(status, 0);
    }

    #[test]
    fn test_timer_interrupt_flow() {
        use crate::core::interrupt::interrupts;

        let mut system = System::new();

        // Setup a simple instruction loop in BIOS
        // j 0xBFC00000 (jump to self)
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        // nop (delay slot)
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Configure timer 0 to trigger quickly
        system.timers.borrow_mut().channel_mut(0).write_target(10); // Target of 10 cycles
        system.timers.borrow_mut().channel_mut(0).write_mode(0x0010); // IRQ on target

        // Enable Timer 0 interrupts in interrupt controller
        system
            .interrupt_controller
            .borrow_mut()
            .write_mask(interrupts::TIMER0 as u32);

        // Run for a few cycles to trigger the timer
        for _ in 0..20 {
            system.step().unwrap();
        }

        // Verify interrupt was requested
        let status = system.interrupt_controller.borrow().read_status();
        assert_ne!(
            status & interrupts::TIMER0 as u32,
            0,
            "Timer 0 interrupt should be pending"
        );

        // Verify interrupt is pending for CPU
        assert!(system.interrupt_controller.borrow().is_pending());
    }

    #[test]
    fn test_interrupt_masking() {
        use crate::core::interrupt::interrupts;

        let system = System::new();

        // Request Timer 0 interrupt
        system
            .interrupt_controller
            .borrow_mut()
            .request(interrupts::TIMER0);

        // Mask all interrupts
        system.interrupt_controller.borrow_mut().write_mask(0);

        // Interrupt should not be pending
        assert!(!system.interrupt_controller.borrow().is_pending());

        // Unmask Timer 0
        system
            .interrupt_controller
            .borrow_mut()
            .write_mask(interrupts::TIMER0 as u32);

        // Now it should be pending
        assert!(system.interrupt_controller.borrow().is_pending());
    }

    #[test]
    fn test_interrupt_acknowledge() {
        use crate::core::interrupt::interrupts;

        let system = System::new();

        // Request Timer 0 interrupt
        system
            .interrupt_controller
            .borrow_mut()
            .request(interrupts::TIMER0);

        // Enable Timer 0 interrupts
        system
            .interrupt_controller
            .borrow_mut()
            .write_mask(interrupts::TIMER0 as u32);

        assert!(system.interrupt_controller.borrow().is_pending());

        // Acknowledge the interrupt (write 0 to clear)
        system
            .interrupt_controller
            .borrow_mut()
            .write_status(!interrupts::TIMER0 as u32);

        // Should no longer be pending
        assert!(!system.interrupt_controller.borrow().is_pending());
    }

    #[test]
    fn test_multiple_timer_interrupts() {
        use crate::core::interrupt::interrupts;

        let mut system = System::new();

        // Setup a simple instruction loop
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Configure multiple timers
        for i in 0..3 {
            system
                .timers
                .borrow_mut()
                .channel_mut(i)
                .write_target(10 + (i as u16) * 5);
            system.timers.borrow_mut().channel_mut(i).write_mode(0x0010); // IRQ on target
        }

        // Enable all timer interrupts
        system
            .interrupt_controller
            .borrow_mut()
            .write_mask((interrupts::TIMER0 | interrupts::TIMER1 | interrupts::TIMER2) as u32);

        // Run for enough cycles to trigger all timers
        for _ in 0..30 {
            system.step().unwrap();
        }

        // All timer interrupts should be pending
        let status = system.interrupt_controller.borrow().read_status();
        assert_ne!(
            status & interrupts::TIMER0 as u32,
            0,
            "Timer 0 should have triggered"
        );
        assert_ne!(
            status & interrupts::TIMER1 as u32,
            0,
            "Timer 1 should have triggered"
        );
        assert_ne!(
            status & interrupts::TIMER2 as u32,
            0,
            "Timer 2 should have triggered"
        );
    }
}
