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

mod controller_ports;

pub use controller_ports::ControllerPorts;

#[cfg(feature = "audio")]
use super::audio::AudioBackend;
use super::cdrom::CDROM;
use super::cpu::{CpuTracer, CPU};
use super::dma::DMA;
use super::error::{EmulatorError, Result};
use super::gpu::GPU;
use super::interrupt::{interrupts, InterruptController};
use super::memory::Bus;
use super::spu::SPU;
use super::timer::Timers;
use super::timing::TimingEventManager;
use std::cell::RefCell;
use std::rc::Rc;

/// PlayStation System
///
/// Integrates all hardware components and manages the emulation loop.
///
/// # Components
/// - CPU: MIPS R3000A processor
/// - Bus: Memory bus for RAM, BIOS, and I/O
/// - GPU: Graphics processing unit
/// - SPU: Sound processing unit
/// - Audio: Audio output backend
/// - DMA: Direct Memory Access controller
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
    /// Timing event manager
    timing: TimingEventManager,
    /// GPU instance (shared via Rc<RefCell> for memory-mapped access)
    gpu: Rc<RefCell<GPU>>,
    /// SPU instance (shared via Rc<RefCell> for memory-mapped access)
    spu: Rc<RefCell<SPU>>,
    /// DMA controller (shared via Rc<RefCell> for memory-mapped access)
    dma: Rc<RefCell<DMA>>,
    /// CDROM drive (shared via Rc<RefCell> for memory-mapped access)
    cdrom: Rc<RefCell<CDROM>>,
    /// Controller ports (shared via Rc<RefCell> for memory-mapped access)
    controller_ports: Rc<RefCell<ControllerPorts>>,
    /// Timers (shared via Rc<RefCell> for memory-mapped access)
    timers: Rc<RefCell<Timers>>,
    /// Interrupt controller (shared via Rc<RefCell> for memory-mapped access)
    interrupt_controller: Rc<RefCell<InterruptController>>,
    /// Audio output backend (optional, may not be available on all systems)
    #[cfg(feature = "audio")]
    audio: Option<AudioBackend>,
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
    /// Registers timing events for all components.
    ///
    /// # Returns
    /// Initialized System instance
    pub fn new() -> Self {
        // Create GPU wrapped in Rc<RefCell> for shared access
        let gpu = Rc::new(RefCell::new(GPU::new()));

        // Create DMA controller wrapped in Rc<RefCell> for shared access
        let dma = Rc::new(RefCell::new(DMA::new()));

        // Create CDROM wrapped in Rc<RefCell> for shared access
        let cdrom = Rc::new(RefCell::new(CDROM::new()));

        // Create ControllerPorts wrapped in Rc<RefCell> for shared access
        let controller_ports = Rc::new(RefCell::new(ControllerPorts::new()));

        // Create Timers wrapped in Rc<RefCell> for shared access
        let timers = Rc::new(RefCell::new(Timers::new()));

        // Create Interrupt Controller wrapped in Rc<RefCell> for shared access
        let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));

        // Create SPU wrapped in Rc<RefCell> for shared access
        let spu = Rc::new(RefCell::new(SPU::new()));

        // Create bus and connect all peripherals for memory-mapped I/O
        let mut bus = Bus::new();
        bus.set_gpu(gpu.clone());
        bus.set_dma(dma.clone());
        bus.set_cdrom(cdrom.clone());
        bus.set_controller_ports(controller_ports.clone());
        bus.set_timers(timers.clone());
        bus.set_interrupt_controller(interrupt_controller.clone());
        bus.set_spu(spu.clone());

        // Create timing manager
        let mut timing = TimingEventManager::new();

        // Register timing events for CD-ROM
        cdrom.borrow_mut().register_events(&mut timing);

        // Register timing events for GPU
        gpu.borrow_mut().register_events(&mut timing);

        // Register timing events for Timers
        timers.borrow_mut().register_events(&mut timing);

        log::info!("System: All components initialized and timing events registered");

        // Initialize audio backend (optional, only if feature is enabled)
        #[cfg(feature = "audio")]
        let audio = match AudioBackend::new() {
            Ok(backend) => {
                log::info!("Audio backend initialized successfully");
                Some(backend)
            }
            Err(e) => {
                log::warn!("Failed to initialize audio backend: {}", e);
                log::warn!("Audio output will be disabled");
                None
            }
        };

        Self {
            cpu: CPU::new(),
            bus,
            timing,
            gpu,
            spu,
            dma,
            cdrom,
            controller_ports,
            timers,
            interrupt_controller,
            #[cfg(feature = "audio")]
            audio,
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
        // Reset SPU by creating a new instance and updating bus connection
        self.spu = Rc::new(RefCell::new(SPU::new()));
        self.bus.set_spu(self.spu.clone());
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

        // Tick DMA controller to process active transfers
        // DMA gets access to RAM, GPU, CD-ROM, and SPU for data transfers
        let dma_irq = {
            let ram = self.bus.ram_mut();
            let mut gpu = self.gpu.borrow_mut();
            let mut cdrom = self.cdrom.borrow_mut();
            let mut spu = self.spu.borrow_mut();
            self.dma
                .borrow_mut()
                .tick(ram, &mut gpu, &mut cdrom, &mut spu)
        };

        // Request DMA interrupt if any transfer completed
        if dma_irq {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::DMA);
        }

        // Apply icache invalidation from memory writes (must come before prefill)
        // This maintains cache coherency when memory is modified
        for addr in self.bus.drain_icache_invalidate_queue() {
            self.cpu.invalidate_icache(addr);
        }

        // Apply icache range invalidation from bulk memory writes (e.g., executable loading)
        // This efficiently invalidates large ranges without queueing individual addresses
        for (start, end) in self.bus.drain_icache_invalidate_range_queue() {
            self.cpu.invalidate_icache_range(start, end);
        }

        // Apply icache prefill from memory writes
        // This ensures instructions are cached before execution
        for (addr, instruction) in self.bus.drain_icache_prefill_queue() {
            self.cpu.prefill_icache(addr, instruction);
        }

        // Tick GPU (legacy timing for backward compatibility)
        // Event-driven timing handles VBlank/HBlank via timing events
        let (_vblank_irq_legacy, hblank_irq_legacy) = self.gpu.borrow_mut().tick(cpu_cycles);

        // Tick timers with HBlank signal (legacy timing)
        // For now, in_hblank is simplified (always false)
        let timer_irqs_legacy = self
            .timers
            .borrow_mut()
            .tick(cpu_cycles, false, hblank_irq_legacy);

        // Run pending timing events to get list of triggered events
        // Note: CPU::execute() also calls this, but we may need to run it here
        // for events triggered during this step
        let triggered_events = if self.timing.pending_ticks > 0 {
            self.timing.run_events()
        } else {
            Vec::new()
        };

        // Process CD-ROM timing events
        // This handles both command scheduling and event callbacks
        self.cdrom
            .borrow_mut()
            .process_events(&mut self.timing, &triggered_events);

        // Process GPU timing events (VBlank/HBlank)
        self.gpu
            .borrow_mut()
            .process_events(&mut self.timing, &triggered_events);

        // Poll GPU interrupts from event-driven timing
        let (vblank_irq, hblank_irq) = self.gpu.borrow_mut().poll_interrupts();

        // Request VBlank interrupt
        if vblank_irq {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::VBLANK);
        }

        // Process Timer timing events (overflow detection)
        self.timers
            .borrow_mut()
            .process_events(&mut self.timing, &triggered_events);

        // Poll timer interrupts from event-driven timing
        let timer_irqs_event = self.timers.borrow_mut().poll_interrupts();

        // Re-tick timers if event-driven HBlank occurred
        // This ensures timers see the HBlank signal from timing events
        if hblank_irq {
            let _timer_irqs = self.timers.borrow_mut().tick(0, false, true);
        }

        // Merge timer interrupts from both event-driven and legacy timing
        let timer_irqs = [
            timer_irqs_legacy[0] || timer_irqs_event[0],
            timer_irqs_legacy[1] || timer_irqs_event[1],
            timer_irqs_legacy[2] || timer_irqs_event[2],
        ];

        // Request timer interrupts (merged from both timing methods)
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

        // Tick CD-ROM drive (synchronized with CPU cycles) - for legacy timing
        // TODO: Remove this once all CD-ROM timing is event-driven
        self.cdrom.borrow_mut().tick(cpu_cycles);

        // Request CD-ROM interrupt if flag is set
        let cdrom_irq_flag = self.cdrom.borrow().interrupt_flag();
        if cdrom_irq_flag != 0 {
            self.interrupt_controller
                .borrow_mut()
                .request(interrupts::CDROM);
        }

        // Tick SPU to generate audio samples with CD-DA mixing (only if audio feature is enabled)
        #[cfg(feature = "audio")]
        {
            // Generate audio samples with CD audio mixed in
            // We need to coordinate between CDROM (which owns cd_audio) and SPU
            let audio_samples = {
                let mut cdrom = self.cdrom.borrow_mut();
                let mut spu = self.spu.borrow_mut();
                spu.tick_with_cd(cpu_cycles, &mut cdrom.cd_audio)
            };

            // Queue samples to audio backend if available
            if let Some(ref mut audio) = self.audio {
                if !audio_samples.is_empty() {
                    audio.queue_samples(&audio_samples);

                    // Check buffer level and warn on underruns
                    let buffer_level = audio.buffer_level();
                    if buffer_level < 512 {
                        log::warn!("Audio buffer underrun: {} samples queued", buffer_level);
                    }
                }
            }
        }

        self.cycles += cpu_cycles as u64;

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
    /// This method uses event-driven execution through the timing system.
    /// The CPU executes until the timing system signals the frame is complete.
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

        // Set frame target in timing system
        self.timing.set_frame_target(CYCLES_PER_FRAME);

        // Execute CPU until timing system signals frame complete
        self.cpu.execute(&mut self.bus, &mut self.timing)?;

        // Tick SPU for one frame worth of cycles and queue audio if available
        #[cfg(feature = "audio")]
        {
            // Generate audio samples with CD audio mixed in
            let audio_samples = {
                let mut cdrom = self.cdrom.borrow_mut();
                let mut spu = self.spu.borrow_mut();
                spu.tick_with_cd(CYCLES_PER_FRAME as u32, &mut cdrom.cd_audio)
            };

            if let Some(ref mut audio) = self.audio {
                if !audio_samples.is_empty() {
                    audio.queue_samples(&audio_samples);

                    // Check buffer level and warn on underruns
                    let buffer_level = audio.buffer_level();
                    if buffer_level < 512 {
                        log::warn!("Audio buffer underrun: {} samples queued", buffer_level);
                    }
                }
            }
        }

        // Update total cycles from timing system
        self.cycles = self.timing.global_tick_counter;

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

    /// Get reference to CDROM
    ///
    /// # Returns
    /// Reference to CDROM instance (wrapped in Rc<RefCell>)
    pub fn cdrom(&self) -> Rc<RefCell<CDROM>> {
        Rc::clone(&self.cdrom)
    }

    /// Load a game from CD-ROM and prepare for execution
    ///
    /// **Current Implementation Status (Partial):**
    ///
    /// Currently implemented:
    /// 1. Load disc image from .cue file
    /// 2. Read SYSTEM.CNF from disc (hard-coded filename: "SYSTEM.CNF;1")
    /// 3. Parse SYSTEM.CNF to find boot executable path
    ///
    /// **Not yet implemented (TODO):**
    /// 4. Full ISO9660 filesystem parsing to locate executable by path
    /// 5. Load PSX-EXE file from disc
    /// 6. Copy executable data to RAM
    /// 7. Set CPU registers (PC, GP, SP, FP)
    ///
    /// This method will return an error until ISO9660 support is completed.
    /// The full game boot sequence is planned for a future phase.
    ///
    /// # Arguments
    ///
    /// * `cue_path` - Path to the disc image .cue file
    ///
    /// # Returns
    ///
    /// - `Ok(())` if disc loads and SYSTEM.CNF is parsed successfully
    /// - `Err(EmulatorError)` currently returns error for unimplemented executable loading
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    ///
    /// let mut system = System::new();
    /// system.load_bios("SCPH1001.BIN").unwrap();
    ///
    /// // Currently only loads disc and parses SYSTEM.CNF
    /// // Full executable loading not yet implemented
    /// match system.load_game("game.cue") {
    ///     Ok(_) => println!("Disc loaded, SYSTEM.CNF parsed"),
    ///     Err(_) => println!("Executable loading not yet implemented"),
    /// }
    /// ```
    pub fn load_game(&mut self, cue_path: &str) -> Result<()> {
        use super::loader::SystemConfig;
        // PSXExecutable will be used when full ISO9660 parsing is implemented
        #[allow(unused_imports)]
        use super::loader::PSXExecutable;

        log::info!("Loading game from: {}", cue_path);

        // Step 1: Load disc image
        self.cdrom
            .borrow_mut()
            .load_disc(cue_path)
            .map_err(EmulatorError::CdRom)?;

        log::info!("Disc loaded successfully");

        // Step 2: Read SYSTEM.CNF from disc
        let system_cnf_data = self
            .cdrom
            .borrow_mut()
            .read_file("SYSTEM.CNF;1")
            .map_err(EmulatorError::CdRom)?;

        let system_cnf_text = String::from_utf8_lossy(&system_cnf_data);
        log::debug!("SYSTEM.CNF contents:\n{}", system_cnf_text);

        // Step 3: Parse SYSTEM.CNF
        let config = SystemConfig::parse(&system_cnf_text)?;
        log::info!("Boot file: {}", config.boot_file);
        log::debug!("Stack: 0x{:08X}", config.stack);

        // Step 4: Read executable from disc
        // A full implementation would need ISO9660 parsing to locate the executable
        // TODO: Implement full ISO9660 file system parsing
        //
        // When implemented, this would be:
        // let exe_data = self.cdrom.borrow_mut().read_file(&config.boot_file)?;
        // let exe = PSXExecutable::load(&exe_data)?;
        //
        // // Step 5: Load executable data into RAM
        // self.bus.write_ram_slice(exe.load_address, &exe.data)?;
        //
        // // Step 6: Set CPU registers
        // self.cpu.set_pc(exe.pc);
        // self.cpu.set_reg(28, exe.gp);  // $gp (global pointer)
        //
        // // Setup stack
        // let sp = if config.stack != 0x801FFF00 {
        //     config.stack
        // } else if exe.stack_base != 0 {
        //     exe.stack_base + exe.stack_offset
        // } else {
        //     config.stack
        // };
        // self.cpu.set_reg(29, sp);  // $sp (stack pointer)
        // self.cpu.set_reg(30, sp);  // $fp (frame pointer)
        //
        // log::info!("Game loaded successfully!");
        // log::info!("Entry point: 0x{:08X}", exe.pc);
        // log::info!("Global pointer: 0x{:08X}", exe.gp);
        // log::info!("Stack pointer: 0x{:08X}", sp);

        // For now, return error since executable loading is not implemented
        Err(EmulatorError::LoaderError(format!(
            "ISO9660 filesystem parsing not yet implemented. Cannot load executable: {}. \
             Disc loaded successfully and SYSTEM.CNF parsed, but full boot sequence requires ISO9660 support.",
            config.boot_file
        )))
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
    fn test_system_timing_manager_created() {
        let system = System::new();
        // Verify timing manager is initialized properly
        assert_eq!(system.timing.global_tick_counter, 0);
        assert_eq!(system.timing.pending_ticks, 0);
        // With GPU events activated, downcount should be set to HBlank interval (2146 cycles)
        // which is the smallest periodic event
        assert_eq!(system.timing.downcount, 2146);
    }

    #[test]
    fn test_run_frame_uses_timing_system() {
        let mut system = System::new();

        // Create an infinite loop in BIOS
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Run one frame
        system.run_frame().unwrap();

        // Verify that timing system's global counter was updated
        const CYCLES_PER_FRAME: u64 = 564_480;
        assert!(system.timing.global_tick_counter >= CYCLES_PER_FRAME);
        assert_eq!(system.cycles(), system.timing.global_tick_counter);
    }

    #[test]
    fn test_frame_target_stops_execution() {
        let mut system = System::new();

        // Create an infinite loop in BIOS
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Run frame should set frame target and stop execution
        let initial_cycles = system.cycles();
        system.run_frame().unwrap();

        const CYCLES_PER_FRAME: u64 = 564_480;
        let cycles_executed = system.cycles() - initial_cycles;

        // Verify frame target mechanism works:
        // 1. Should execute at least the target number of cycles
        assert!(
            cycles_executed >= CYCLES_PER_FRAME,
            "Expected at least {} cycles, got {}",
            CYCLES_PER_FRAME,
            cycles_executed
        );

        // 2. Should stop execution (not run indefinitely)
        // The infinite loop test proves the frame target mechanism stopped execution
        // Note: May overshoot target due to instruction and event processing granularity
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

    // DMA Integration Tests

    #[test]
    fn test_dma_integration() {
        let mut system = System::new();

        // Setup a simple instruction loop in BIOS
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
        system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap(); // All channels enabled with priorities

        // Setup GPU DMA transfer (OTC mode for simplicity)
        // Use Channel 6 (OTC) which is simpler to test
        system.bus.write32(0x1F8010E0, 0x00000100).unwrap(); // MADR = 0x100
        system.bus.write32(0x1F8010E4, 0x00000010).unwrap(); // BCR = 16 entries
        system.bus.write32(0x1F8010E8, 0x11000002).unwrap(); // CHCR = start + trigger

        // Enable DMA interrupts in DICR
        system.bus.write32(0x1F8010F4, 0x00FF0000).unwrap(); // Enable all channel interrupts

        // Run a few cycles to trigger DMA
        for _ in 0..5 {
            system.step().unwrap();
        }

        // Check that DMA transfer completed
        let chcr = system.bus.read32(0x1F8010E8).unwrap();
        assert_eq!(
            chcr & 0x01000000,
            0,
            "DMA channel 6 should be inactive after transfer"
        );

        // Check that DMA created the ordering table in RAM
        // OTC writes backwards: first entry points to previous, last entry is 0x00FFFFFF
        // With MADR=0x100 and count=16, entries are at 0x100, 0xFC, 0xF8, ... 0xC4
        // First entry at 0x100 should link to 0xFC
        let first_entry = system.bus.read32(0x00000100).unwrap();
        assert_eq!(
            first_entry, 0x000000FC,
            "OTC first entry should link to previous address"
        );

        // Last entry at 0xC4 (0x100 - 15*4 = 0x100 - 0x3C) should be end marker
        let last_entry = system.bus.read32(0x000000C4).unwrap();
        assert_eq!(
            last_entry, 0x00FFFFFF,
            "OTC last entry should be end marker"
        );
    }

    #[test]
    fn test_dma_gpu_transfer() {
        let mut system = System::new();

        // Setup a simple instruction loop
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
        system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap();

        // Setup test data in RAM for GPU transfer
        system.bus.write32(0x00001000, 0xA0000000).unwrap(); // GP0 fill command
        system.bus.write32(0x00001004, 0x00640064).unwrap(); // Position
        system.bus.write32(0x00001008, 0x00020002).unwrap(); // Size
        system.bus.write32(0x0000100C, 0x12345678).unwrap(); // Color data

        // Setup GPU DMA transfer (Channel 2, block mode)
        system.bus.write32(0x1F8010A0, 0x00001000).unwrap(); // MADR = 0x1000
        system.bus.write32(0x1F8010A4, 0x00010004).unwrap(); // BCR = 4 words, 1 block
        system.bus.write32(0x1F8010A8, 0x11000201).unwrap(); // CHCR = to GPU, sync mode 0, start, trigger

        // Run a few cycles to process DMA
        for _ in 0..10 {
            system.step().unwrap();
        }

        // Verify DMA channel is no longer active
        let chcr = system.bus.read32(0x1F8010A8).unwrap();
        assert_eq!(
            chcr & 0x01000000,
            0,
            "GPU DMA should be complete and inactive"
        );
    }

    #[test]
    fn test_dma_interrupt() {
        use crate::core::interrupt::interrupts;

        let mut system = System::new();

        // Setup a simple instruction loop
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Enable DMA channels in DPCR (bit 3 of each nibble enables the channel)
        system.bus.write32(0x1F8010F0, 0x0FEDCBA8).unwrap();

        // Enable DMA interrupts in DICR (master enable + channel 6 enable)
        system.bus.write32(0x1F8010F4, 0x00C00000).unwrap(); // Bit 23 (master) + bit 22 (ch6)

        // Enable DMA interrupt in interrupt controller
        system
            .interrupt_controller
            .borrow_mut()
            .write_mask(interrupts::DMA as u32);

        // Setup OTC DMA transfer
        system.bus.write32(0x1F8010E0, 0x00001000).unwrap(); // MADR = 0x1000
        system.bus.write32(0x1F8010E4, 0x00000008).unwrap(); // BCR = 8 entries
        system.bus.write32(0x1F8010E8, 0x11000002).unwrap(); // CHCR = start + trigger

        // Run a few cycles to trigger DMA
        for _ in 0..5 {
            system.step().unwrap();
        }

        // Verify DMA interrupt was raised
        let i_stat = system.interrupt_controller.borrow().read_status();
        assert_ne!(
            i_stat & interrupts::DMA as u32,
            0,
            "DMA interrupt should be set in I_STAT"
        );

        // Verify DICR has channel 6 flag set
        let dicr = system.bus.read32(0x1F8010F4).unwrap();
        assert_ne!(
            dicr & (1 << 30),
            0,
            "DICR should have channel 6 interrupt flag set"
        );
        assert_ne!(dicr & (1 << 31), 0, "DICR master flag should be set");
    }

    // Audio Integration Tests

    #[test]
    #[cfg(feature = "audio")]
    fn test_audio_backend_optional() {
        let system = System::new();
        // Audio backend may or may not be initialized depending on system capabilities
        // This test just ensures the system can be created regardless
        assert_eq!(system.cycles(), 0);
    }

    #[test]
    #[cfg(feature = "audio")]
    fn test_spu_audio_integration_via_step() {
        let mut system = System::new();

        // Skip test if no audio backend available
        if system.audio.is_none() {
            return;
        }

        // Create an infinite loop in BIOS
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Enable SPU via control register write
        system.bus.write16(0x1F801DAA, 0x8000).unwrap(); // Enable bit

        // Note: Individual step() calls with 1 cycle each won't generate samples
        // because SPU::tick(1) returns 0 samples (truncates to 0).
        // This test verifies the integration doesn't crash.
        // For actual sample generation, see test_run_frame_generates_audio.
        for _ in 0..100 {
            system.step().unwrap();
        }

        // Verify system continues to work with audio enabled
        // (actual sample generation requires larger cycle batches)
        assert!(system.cycles() >= 100);
    }

    #[test]
    #[cfg(feature = "audio")]
    fn test_run_frame_generates_audio() {
        let mut system = System::new();

        // Skip test if no audio backend available
        if system.audio.is_none() {
            return;
        }

        // Create an infinite loop in BIOS
        let jump_bytes = 0x0BF00000u32.to_le_bytes();
        system.bus_mut().write_bios_for_test(0, &jump_bytes);
        system
            .bus_mut()
            .write_bios_for_test(4, &[0x00, 0x00, 0x00, 0x00]);

        system.reset();

        // Enable SPU
        system.bus.write16(0x1F801DAA, 0x8000).unwrap();

        // Run one frame - should generate ~735 samples at 44.1 kHz
        system.run_frame().unwrap();

        // Verify audio samples were generated and queued
        if let Some(ref audio) = system.audio {
            let buffer_level = audio.buffer_level();
            // One frame should generate approximately 735 samples
            assert!(
                (730..=740).contains(&buffer_level),
                "Expected ~735 samples per frame, got {}",
                buffer_level
            );
        }
    }

    #[test]
    fn test_dma_registers_accessible() {
        let system = System::new();

        // Verify all DMA channel registers are accessible
        for ch in 0..7 {
            let base = 0x1F801080 + (ch * 0x10);

            // Read MADR (should be 0 initially)
            let madr = system.bus.read32(base).unwrap();
            assert_eq!(madr, 0, "Channel {} MADR should be 0", ch);

            // Read BCR (should be 0 initially)
            let bcr = system.bus.read32(base + 4).unwrap();
            assert_eq!(bcr, 0, "Channel {} BCR should be 0", ch);

            // Read CHCR (should be 0 initially)
            let chcr = system.bus.read32(base + 8).unwrap();
            assert_eq!(chcr, 0, "Channel {} CHCR should be 0", ch);
        }

        // Read DPCR (should have default priority)
        let dpcr = system.bus.read32(0x1F8010F0).unwrap();
        assert_eq!(dpcr, 0x07654321, "DPCR should have default priority");

        // Read DICR (should be 0 initially)
        let dicr = system.bus.read32(0x1F8010F4).unwrap();
        assert_eq!(dicr, 0, "DICR should be 0 initially");
    }
}
