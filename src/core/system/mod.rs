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
mod tests;
