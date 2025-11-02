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
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::time::{Duration, Instant};

slint::include_modules!();

/// Frontend for the PlayStation emulator
///
/// Provides a Slint-based UI that displays the GPU framebuffer and status information.
/// Runs the emulation loop at approximately 60 FPS.
pub struct Frontend {
    /// Slint window instance
    window: MainWindow,
    /// Core emulator system
    system: System,
    /// Last frame time for FPS calculation
    last_frame_time: Instant,
    /// Frame counter for FPS calculation
    frame_count: u32,
    /// Current FPS value
    fps: f32,
    /// Performance tracking: frame times for averaging
    frame_times: Vec<Duration>,
    /// Last performance log time
    last_perf_log: Instant,
    /// Debug mode enabled
    debug_mode: bool,
    /// Pause state
    paused: bool,
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
            log::warn!("Running in WSL ({}). Make sure X11 or Wayland is configured.", wsl_distro);
            log::warn!("For X11: Set DISPLAY environment variable (e.g., export DISPLAY=:0)");
            log::warn!("For WSLg: Ensure you're on Windows 11 with WSLg support");
        }

        let window = MainWindow::new().expect("Failed to create Slint MainWindow");

        log::info!("Slint window created successfully");

        // Enable debug mode by default to see GPU/CPU info
        window.set_debug_mode(true);

        Self {
            window,
            system,
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps: 0.0,
            frame_times: Vec::new(),
            last_perf_log: Instant::now(),
            debug_mode: true, // Enable debug mode
            paused: false,
        }
    }

    /// Run the emulator with UI
    ///
    /// This method runs the main emulation loop:
    /// 1. Execute one frame of emulation (~564,480 CPU cycles)
    /// 2. Get framebuffer from GPU
    /// 3. Convert and display framebuffer
    /// 4. Update FPS counter
    /// 5. Process UI events
    /// 6. Sleep to maintain ~60 FPS
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
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set running state
        self.window.set_running(true);

        // Show the window explicitly
        self.window.show().map_err(|e| {
            log::error!("Failed to show window: {}", e);
            format!("Failed to show window: {}", e)
        })?;

        log::info!("Window displayed, starting emulation loop");

        // Note: Keyboard input will be handled through Slint UI callbacks in future versions
        // For now, users can close the window using the window controls

        // Main loop
        loop {
            let frame_start = Instant::now();

            // Run one frame of emulation (unless paused)
            if !self.paused {
                if let Err(e) = self.system.run_frame() {
                    log::error!("Emulation error: {}", e);
                    self.window.set_running(false);
                    return Err(Box::new(e));
                }
            }

            // Get framebuffer from GPU
            let gpu = self.system.gpu();
            let framebuffer = gpu.borrow().get_framebuffer();
            let display_area = gpu.borrow().display_area();
            let gpu_status = gpu.borrow().status();
            drop(gpu); // Release borrow

            let width = display_area.width as usize;
            let height = display_area.height as usize;

            // Convert to Slint image
            let image = self.framebuffer_to_image(&framebuffer, width, height);

            // Update display
            self.window.set_framebuffer(image);

            // Update FPS counter
            self.update_fps();

            // Update debug info
            if self.debug_mode {
                let pc = self.system.pc();
                self.window.set_cpu_pc(format!("PC: 0x{:08X}", pc).into());
                self.window
                    .set_gpu_status(format!("GPU: 0x{:08X}", gpu_status).into());
            }

            // Track frame time
            let frame_time = frame_start.elapsed();
            self.frame_times.push(frame_time);

            // Update performance text
            self.window.set_performance_text(
                format!("Frame: {:.2}ms", frame_time.as_secs_f64() * 1000.0).into(),
            );

            // Log performance every 5 seconds
            self.log_performance();

            // Process UI events
            slint::platform::update_timers_and_animations();

            // Limit to ~60fps (16.67ms per frame)
            std::thread::sleep(Duration::from_millis(16));

            // Check if window was closed
            if !self.window.window().is_visible() {
                log::info!("Window closed by user");
                break;
            }
        }

        log::info!("Exiting emulation loop");
        self.window.set_running(false);
        Ok(())
    }

    /// Log performance metrics
    ///
    /// Logs average frame time and FPS every 5 seconds
    fn log_performance(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_perf_log);

        // Log performance every 5 seconds
        if elapsed >= Duration::from_secs(5) && !self.frame_times.is_empty() {
            let avg_frame_time =
                self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
            let avg_fps = 1.0 / avg_frame_time.as_secs_f64();

            log::info!(
                "Performance: avg {:.2}ms/frame ({:.1} fps)",
                avg_frame_time.as_secs_f64() * 1000.0,
                avg_fps
            );

            // Reset counters
            self.frame_times.clear();
            self.last_perf_log = now;
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
    fn framebuffer_to_image(&self, framebuffer: &[u8], width: usize, height: usize) -> Image {
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

    /// Update FPS counter
    ///
    /// Calculates FPS based on frames rendered in the last second.
    /// Updates the UI text once per second.
    fn update_fps(&mut self) {
        self.frame_count += 1;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);

        // Update FPS display once per second
        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.window
                .set_fps_text(format!("FPS: {:.1}", self.fps).into());

            // Reset counters
            self.frame_count = 0;
            self.last_frame_time = now;
        }
    }
}
