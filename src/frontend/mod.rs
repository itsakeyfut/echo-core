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

//! Frontend UI module
//!
//! This module implements the Slint-based user interface for the emulator.
//! It handles:
//! - Window creation and management
//! - Framebuffer rendering (GPU → Screen)
//! - FPS counter and status display
//! - Main emulation loop timing
//!
//! # Architecture
//!
//! The frontend wraps the core `System` and provides a visual interface:
//!
//! ```text
//! Frontend
//!   ├─ MainWindow (Slint UI)
//!   │   ├─ Framebuffer display
//!   │   └─ Status bar (FPS, running state)
//!   └─ System (emulator core)
//!       └─ GPU (framebuffer source)
//! ```
//!
//! # Example
//!
//! ```no_run
//! use psrx::core::system::System;
//! use psrx::frontend::Frontend;
//!
//! let mut system = System::new();
//! system.load_bios("bios.bin").unwrap();
//! system.reset();
//!
//! let frontend = Frontend::new(system);
//! frontend.run().unwrap();
//! ```

use crate::core::system::System;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::time::{Duration, Instant};

slint::include_modules!();

/// Frontend state for the emulator
///
/// Shared state accessed by the timer callback
struct FrontendState {
    system: System,
    last_frame_time: Instant,
    frame_count: u32,
    frame_times: Vec<Duration>,
    last_perf_log: Instant,
}

impl FrontendState {
    fn new(system: System) -> Self {
        Self {
            system,
            last_frame_time: Instant::now(),
            frame_count: 0,
            frame_times: Vec::new(),
            last_perf_log: Instant::now(),
        }
    }
}

/// Frontend for the PlayStation emulator
///
/// Provides a Slint-based UI that displays the GPU framebuffer and status information.
/// Runs the emulation loop at approximately 60 FPS.
pub struct Frontend {
    /// Slint window instance
    window: MainWindow,
    /// Frontend state (shared with timer callback)
    state: Rc<RefCell<FrontendState>>,
}

impl Frontend {
    /// Create a new frontend instance
    ///
    /// # Arguments
    /// * `system` - The emulator system to run
    ///
    /// # Returns
    /// A new Frontend instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    /// use psrx::frontend::Frontend;
    ///
    /// let system = System::new();
    /// let frontend = Frontend::new(system);
    /// ```
    pub fn new(system: System) -> Self {
        // Check for WSL and provide helpful message
        if let Ok(wsl_distro) = std::env::var("WSL_DISTRO_NAME") {
            log::warn!(
                "Running in WSL ({}). Make sure X11 or Wayland is configured.",
                wsl_distro
            );
            log::warn!("For X11: Set DISPLAY environment variable (e.g., export DISPLAY=:0)");
            log::warn!("For WSLg: Ensure you're on Windows 11 with WSLg support");
        }

        let window = MainWindow::new().expect("Failed to create Slint MainWindow");

        log::info!("Slint window created successfully");

        // Enable debug mode by default to see GPU/CPU info
        window.set_debug_mode(true);
        window.set_running(true);

        let state = Rc::new(RefCell::new(FrontendState::new(system)));

        Self { window, state }
    }

    /// Run the emulator with UI
    ///
    /// This method starts the Slint event loop with a timer that:
    /// 1. Executes one frame of emulation (~564,480 CPU cycles)
    /// 2. Gets framebuffer from GPU
    /// 3. Converts and displays framebuffer
    /// 4. Updates FPS counter
    ///
    /// The loop continues until the window is closed.
    ///
    /// # Returns
    /// Ok(()) if the window was closed normally, or an error if emulation failed
    ///
    /// # Errors
    /// Returns an error if:
    /// - Emulation encounters a fatal error
    /// - GPU framebuffer cannot be retrieved
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::system::System;
    /// use psrx::frontend::Frontend;
    ///
    /// let mut system = System::new();
    /// system.load_bios("bios.bin").unwrap();
    /// system.reset();
    ///
    /// let frontend = Frontend::new(system);
    /// frontend.run().unwrap();
    /// ```
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting emulation with Slint event loop");

        // Draw test pattern to verify display is working
        {
            let mut state = self.state.borrow_mut();
            let gpu = state.system.gpu();
            let mut gpu_mut = gpu.borrow_mut();

            // Draw white rectangle in top-left (10x10 pixels at position 10,10)
            for y in 10..20 {
                for x in 10..20 {
                    gpu_mut.write_vram(x, y, 0x7FFF); // White
                }
            }

            // Draw red rectangle (20x20 pixels at position 100,50)
            for y in 50..70 {
                for x in 100..120 {
                    let r = 31; // Max red
                    let g = 0;
                    let b = 0;
                    let color = (b << 10) | (g << 5) | r;
                    gpu_mut.write_vram(x, y, color);
                }
            }

            // Draw green rectangle (20x20 pixels at position 150,50)
            for y in 50..70 {
                for x in 150..170 {
                    let r = 0;
                    let g = 31; // Max green
                    let b = 0;
                    let color = (b << 10) | (g << 5) | r;
                    gpu_mut.write_vram(x, y, color);
                }
            }

            // Draw blue rectangle (20x20 pixels at position 200,50)
            for y in 50..70 {
                for x in 200..220 {
                    let r = 0;
                    let g = 0;
                    let b = 31; // Max blue
                    let color = (b << 10) | (g << 5) | r;
                    gpu_mut.write_vram(x, y, color);
                }
            }

            log::info!(
                "Test pattern drawn to VRAM. You should see colored rectangles if display is working."
            );

            // Enable CPU tracing if configured via environment variables
            let trace_enabled = env::var("PSRX_TRACE_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true";

            if trace_enabled {
                let trace_file = env::var("PSRX_TRACE_FILE")
                    .unwrap_or_else(|_| "bios_trace.log".to_string());

                let trace_limit: usize = env::var("PSRX_TRACE_LIMIT")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()
                    .unwrap_or(10000);

                log::info!(
                    "CPU tracing enabled via config: file={}, limit={}",
                    trace_file,
                    trace_limit
                );

                if let Err(e) = state.system.enable_tracing(&trace_file, trace_limit) {
                    log::warn!("Failed to enable CPU tracing: {}", e);
                }
            } else {
                log::debug!("CPU tracing disabled (set PSRX_TRACE_ENABLED=true to enable)");
            }
        }

        // Create timer for emulation loop (60 FPS = ~16.67ms per frame)
        let timer = Timer::default();
        let window_weak = self.window.as_weak();
        let state_rc = self.state.clone();

        timer.start(
            TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let frame_start = Instant::now();

                // Run one frame of emulation
                let mut state = state_rc.borrow_mut();
                if let Err(e) = state.system.run_frame() {
                    log::error!("Emulation error: {}", e);
                    if let Some(window) = window_weak.upgrade() {
                        window.set_running(false);
                    }
                    return;
                }

                // Get framebuffer from GPU
                let gpu = state.system.gpu();
                let framebuffer = gpu.borrow().get_framebuffer();
                let display_area = gpu.borrow().display_area();
                let gpu_status = gpu.borrow().status();
                drop(gpu);

                let width = display_area.width as usize;
                let height = display_area.height as usize;

                // Convert to Slint image
                let image = Self::framebuffer_to_image(&framebuffer, width, height);

                // Update display
                if let Some(window) = window_weak.upgrade() {
                    window.set_framebuffer(image);

                    // Update FPS counter
                    state.frame_count += 1;
                    let now = Instant::now();
                    let elapsed = now.duration_since(state.last_frame_time);

                    if elapsed >= Duration::from_secs(1) {
                        let fps = state.frame_count as f32 / elapsed.as_secs_f32();
                        window.set_fps_text(format!("FPS: {:.1}", fps).into());
                        state.frame_count = 0;
                        state.last_frame_time = now;
                    }

                    // Update debug info
                    let pc = state.system.pc();
                    window.set_cpu_pc(format!("PC: 0x{:08X}", pc).into());
                    window.set_gpu_status(format!("GPU: 0x{:08X}", gpu_status).into());

                    // Track frame time
                    let frame_time = frame_start.elapsed();
                    state.frame_times.push(frame_time);

                    window.set_performance_text(
                        format!("Frame: {:.2}ms", frame_time.as_secs_f64() * 1000.0).into(),
                    );

                    // Log performance every 5 seconds
                    Self::log_performance(&mut state);

                    // Log PC and GPU status periodically
                    use std::sync::OnceLock;
                    static LAST_DEBUG_LOG: OnceLock<std::sync::Mutex<std::time::Instant>> =
                        OnceLock::new();
                    let last_log = LAST_DEBUG_LOG
                        .get_or_init(|| std::sync::Mutex::new(std::time::Instant::now()));
                    if let Ok(mut last) = last_log.lock() {
                        if now.duration_since(*last).as_secs() >= 2 {
                            log::debug!(
                                "PC: 0x{:08X}, GPU Status: 0x{:08X}, Display disabled: {}",
                                pc,
                                gpu_status,
                                (gpu_status >> 23) & 1
                            );
                            *last = now;
                        }
                    }
                }
            },
        );

        // Run Slint event loop (blocks until window is closed)
        log::info!("Entering Slint event loop");
        self.window.run()?;

        log::info!("Exiting emulation");
        Ok(())
    }

    /// Log performance metrics
    ///
    /// Logs average frame time and FPS every 5 seconds
    fn log_performance(state: &mut FrontendState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_perf_log);

        // Log performance every 5 seconds
        if elapsed >= Duration::from_secs(5) && !state.frame_times.is_empty() {
            let avg_frame_time =
                state.frame_times.iter().sum::<Duration>() / state.frame_times.len() as u32;
            let avg_fps = 1.0 / avg_frame_time.as_secs_f64();

            log::info!(
                "Performance: avg {:.2}ms/frame ({:.1} fps)",
                avg_frame_time.as_secs_f64() * 1000.0,
                avg_fps
            );

            // Reset counters
            state.frame_times.clear();
            state.last_perf_log = now;
        }
    }

    /// Convert RGB24 framebuffer to Slint RGBA8 image
    ///
    /// Takes a framebuffer in RGB24 format (3 bytes per pixel) and converts it
    /// to RGBA8 format (4 bytes per pixel) for display in Slint.
    ///
    /// # Arguments
    /// * `framebuffer` - RGB24 data from GPU (width * height * 3 bytes)
    /// * `width` - Framebuffer width in pixels
    /// * `height` - Framebuffer height in pixels
    ///
    /// # Returns
    /// Slint Image in RGBA8 format
    ///
    /// # Panics
    /// Panics if framebuffer size doesn't match width * height * 3
    fn framebuffer_to_image(framebuffer: &[u8], width: usize, height: usize) -> Image {
        // Convert RGB24 to RGBA8
        let mut rgba_buffer = vec![0u8; width * height * 4];

        for y in 0..height {
            for x in 0..width {
                let src_idx = (y * width + x) * 3;
                let dst_idx = (y * width + x) * 4;

                // Copy RGB and add alpha channel
                rgba_buffer[dst_idx] = framebuffer[src_idx]; // R
                rgba_buffer[dst_idx + 1] = framebuffer[src_idx + 1]; // G
                rgba_buffer[dst_idx + 2] = framebuffer[src_idx + 2]; // B
                rgba_buffer[dst_idx + 3] = 255; // A (fully opaque)
            }
        }

        // Create Slint image
        let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &rgba_buffer,
            width as u32,
            height as u32,
        );

        Image::from_rgba8(pixel_buffer)
    }
}
