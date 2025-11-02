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

//! PSX Timer/Counter Implementation
//!
//! The PlayStation has 3 timer channels that can count based on different clock sources
//! and generate interrupts when reaching target values or overflow.
//!
//! ## Timer Channels
//!
//! - **Timer 0**: System clock or pixel clock (GPU dot clock)
//! - **Timer 1**: System clock or horizontal blank
//! - **Timer 2**: System clock or system clock / 8
//!
//! ## Register Layout
//!
//! Each timer has 3 registers at 16-byte intervals:
//! - `0x1F801100 + (n * 0x10)`: Counter value (R/W)
//! - `0x1F801104 + (n * 0x10)`: Mode register (R/W)
//! - `0x1F801108 + (n * 0x10)`: Target value (R/W)
//!
//! ## Mode Register Format (16 bits)
//!
//! ```text
//! 15-13: Not used (always 0)
//! 12:    Reached max value (0xFFFF) - Read-only, reset on read
//! 11:    Reached target value - Read-only, reset on read
//! 10:    IRQ flag - Read-only, reset on read or mode write
//! 9:     Clock source bit 1 (Timer 2 only, other timers: 0)
//! 8:     Clock source bit 0
//! 7:     IRQ pulse mode (0=pulse, 1=toggle)
//! 6:     IRQ repeat mode (0=one-shot, 1=repeat)
//! 5:     IRQ on max value (0xFFFF)
//! 4:     IRQ on target
//! 3:     Reset counter to 0 when target reached
//! 2-1:   Sync mode (meaning depends on timer)
//! 0:     Sync enable
//! ```
//!
//! ## References
//!
//! - [PSX-SPX: Timers](http://problemkaputt.de/psx-spx.htm#timers)

/// Timer mode control register
#[derive(Debug, Clone, Default)]
pub struct TimerMode {
    /// Sync enable (bit 0)
    pub sync_enable: bool,

    /// Sync mode (bits 1-2, meaning depends on timer)
    pub sync_mode: u8,

    /// Reset counter to 0 when target reached (bit 3)
    pub reset_on_target: bool,

    /// IRQ when target reached (bit 4)
    pub irq_on_target: bool,

    /// IRQ when max value (0xFFFF) reached (bit 5)
    pub irq_on_max: bool,

    /// IRQ repeat mode (bit 6)
    pub irq_repeat: bool,

    /// IRQ pulse mode (bit 7) - 0=pulse, 1=toggle
    pub irq_pulse_mode: bool,

    /// Clock source (bits 8-9)
    /// - Timer 0: 0=system clock, 1=pixel clock
    /// - Timer 1: 0=system clock, 1=hblank
    /// - Timer 2: 0=system clock, 1=system/8
    pub clock_source: u8,

    /// IRQ flag (bit 10) - Read-only, writable to reset
    pub irq_flag: bool,

    /// Reached target (bit 11) - Read-only
    pub reached_target: bool,

    /// Reached max (bit 12) - Read-only
    pub reached_max: bool,
}

/// A single timer channel
pub struct TimerChannel {
    /// Current counter value
    counter: u16,

    /// Counter mode/control
    mode: TimerMode,

    /// Target value (for compare interrupt)
    target: u16,

    /// Channel number (0-2)
    channel_id: u8,

    /// IRQ flag (set when target reached or overflow)
    irq_flag: bool,

    /// Reached target flag
    reached_target: bool,

    /// Reached max value (0xFFFF)
    reached_max: bool,
}

impl TimerChannel {
    /// Create a new timer channel
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The timer channel number (0-2)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timer::TimerChannel;
    ///
    /// let timer = TimerChannel::new(0);
    /// assert_eq!(timer.read_counter(), 0);
    /// ```
    pub fn new(channel_id: u8) -> Self {
        Self {
            counter: 0,
            mode: TimerMode::default(),
            target: 0,
            channel_id,
            irq_flag: false,
            reached_target: false,
            reached_max: false,
        }
    }

    /// Read counter value
    ///
    /// Returns the current 16-bit counter value.
    #[inline(always)]
    pub fn read_counter(&self) -> u16 {
        self.counter
    }

    /// Write counter value
    ///
    /// Sets the counter to the specified value.
    ///
    /// # Arguments
    ///
    /// * `value` - New counter value
    pub fn write_counter(&mut self, value: u16) {
        self.counter = value;
        log::trace!("Timer {} counter = 0x{:04X}", self.channel_id, value);
    }

    /// Read mode register
    ///
    /// Returns the mode register value. Reading the mode register
    /// resets the reached_target and reached_max flags.
    pub fn read_mode(&mut self) -> u16 {
        let mut value = 0u16;

        value |= self.mode.sync_enable as u16;
        value |= (self.mode.sync_mode as u16) << 1;
        value |= (self.mode.reset_on_target as u16) << 3;
        value |= (self.mode.irq_on_target as u16) << 4;
        value |= (self.mode.irq_on_max as u16) << 5;
        value |= (self.mode.irq_repeat as u16) << 6;
        value |= (self.mode.irq_pulse_mode as u16) << 7;
        value |= (self.mode.clock_source as u16) << 8;
        value |= (self.irq_flag as u16) << 10;
        value |= (self.reached_target as u16) << 11;
        value |= (self.reached_max as u16) << 12;

        // Reading mode resets flags
        self.reached_target = false;
        self.reached_max = false;

        value
    }

    /// Write mode register
    ///
    /// Sets the timer mode and resets counter and flags.
    ///
    /// # Arguments
    ///
    /// * `value` - Mode register value to write
    pub fn write_mode(&mut self, value: u16) {
        self.mode.sync_enable = (value & 0x0001) != 0;
        self.mode.sync_mode = ((value >> 1) & 0x03) as u8;
        self.mode.reset_on_target = (value & 0x0008) != 0;
        self.mode.irq_on_target = (value & 0x0010) != 0;
        self.mode.irq_on_max = (value & 0x0020) != 0;
        self.mode.irq_repeat = (value & 0x0040) != 0;
        self.mode.irq_pulse_mode = (value & 0x0080) != 0;
        self.mode.clock_source = ((value >> 8) & 0x03) as u8;

        // Writing mode resets counter and flags
        self.counter = 0;
        self.irq_flag = false;
        self.reached_target = false;
        self.reached_max = false;

        log::debug!(
            "Timer {} mode: sync={} source={} target_irq={} max_irq={}",
            self.channel_id,
            self.mode.sync_enable,
            self.mode.clock_source,
            self.mode.irq_on_target,
            self.mode.irq_on_max
        );
    }

    /// Read target value
    ///
    /// Returns the current target value for comparison.
    #[inline(always)]
    pub fn read_target(&self) -> u16 {
        self.target
    }

    /// Write target value
    ///
    /// Sets the target value that triggers interrupts when reached.
    ///
    /// # Arguments
    ///
    /// * `value` - New target value
    pub fn write_target(&mut self, value: u16) {
        self.target = value;
        log::trace!("Timer {} target = 0x{:04X}", self.channel_id, value);
    }

    /// Tick the timer by one or more cycles
    ///
    /// Updates the timer counter and checks for target/overflow conditions.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of cycles to advance
    /// * `sync_signal` - Sync signal state (e.g., hblank, vblank)
    ///
    /// # Returns
    ///
    /// `true` if an IRQ was triggered, `false` otherwise
    pub fn tick(&mut self, cycles: u32, sync_signal: bool) -> bool {
        let mut irq_triggered = false;

        for _ in 0..cycles {
            // Check if we should count based on sync mode
            let should_count = self.should_count(sync_signal);

            if should_count {
                self.counter = self.counter.wrapping_add(1);

                // Check target
                if self.counter == self.target {
                    self.reached_target = true;

                    if self.mode.irq_on_target {
                        self.trigger_irq();
                        irq_triggered = true;
                    }

                    if self.mode.reset_on_target {
                        self.counter = 0;
                    }
                }

                // Check max (0xFFFF)
                if self.counter == 0xFFFF {
                    self.reached_max = true;

                    if self.mode.irq_on_max {
                        self.trigger_irq();
                        irq_triggered = true;
                    }
                }
            }
        }

        irq_triggered
    }

    /// Determine if the timer should count based on sync mode
    ///
    /// # Arguments
    ///
    /// * `sync_signal` - The sync signal state
    ///
    /// # Returns
    ///
    /// `true` if the timer should increment, `false` otherwise
    fn should_count(&self, sync_signal: bool) -> bool {
        if !self.mode.sync_enable {
            return true; // Free-run mode
        }

        // Sync mode behavior (simplified for now)
        // TODO: Implement full sync mode behavior per PSX-SPX specs
        match self.mode.sync_mode {
            0 => !sync_signal, // Pause during sync
            1 => sync_signal,  // Reset on sync
            2 => !sync_signal, // Pause and reset on sync
            3 => sync_signal,  // Pause until sync, then free-run
            _ => true,
        }
    }

    /// Trigger an IRQ
    ///
    /// Sets the IRQ flag if conditions are met (one-shot or repeat mode).
    fn trigger_irq(&mut self) {
        if !self.irq_flag || self.mode.irq_repeat {
            self.irq_flag = true;
            log::trace!("Timer {} IRQ triggered", self.channel_id);
        }
    }

    /// Check if IRQ is pending
    ///
    /// # Returns
    ///
    /// `true` if an interrupt is pending, `false` otherwise
    #[inline(always)]
    pub fn irq_pending(&self) -> bool {
        self.irq_flag
    }

    /// Acknowledge IRQ
    ///
    /// Clears the IRQ flag.
    pub fn ack_irq(&mut self) {
        self.irq_flag = false;
    }
}

/// Timer system managing all 3 timer channels
pub struct Timers {
    /// The 3 timer channels
    channels: [TimerChannel; 3],

    /// Accumulator for Timer 2 divide-by-8 mode
    timer2_div_accum: u32,
}

impl Timers {
    /// Create a new timer system
    ///
    /// Initializes all 3 timer channels.
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timer::Timers;
    ///
    /// let timers = Timers::new();
    /// ```
    pub fn new() -> Self {
        Self {
            channels: [
                TimerChannel::new(0),
                TimerChannel::new(1),
                TimerChannel::new(2),
            ],
            timer2_div_accum: 0,
        }
    }

    /// Get a reference to a timer channel
    ///
    /// # Arguments
    ///
    /// * `index` - Timer channel index (0-2)
    ///
    /// # Returns
    ///
    /// Reference to the requested timer channel
    #[inline(always)]
    pub fn channel(&self, index: usize) -> &TimerChannel {
        &self.channels[index]
    }

    /// Get a mutable reference to a timer channel
    ///
    /// # Arguments
    ///
    /// * `index` - Timer channel index (0-2)
    ///
    /// # Returns
    ///
    /// Mutable reference to the requested timer channel
    #[inline(always)]
    pub fn channel_mut(&mut self, index: usize) -> &mut TimerChannel {
        &mut self.channels[index]
    }

    /// Tick all timers
    ///
    /// Advances all timer channels based on their clock sources.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of CPU cycles elapsed
    /// * `hblank` - Horizontal blank signal state
    /// * `_vblank` - Vertical blank signal state (reserved for future use)
    ///
    /// # Returns
    ///
    /// Array of IRQ flags for each timer channel
    pub fn tick(&mut self, cycles: u32, hblank: bool, _vblank: bool) -> [bool; 3] {
        let mut irqs = [false; 3];

        // Timer 0: System clock or pixel clock (simplified as system clock)
        irqs[0] = self.channels[0].tick(cycles, false);

        // Timer 1: System clock or hblank
        // When in HBlank mode, only advance on actual HBlank edges, not CPU cycles
        let (timer1_cycles, timer1_sync) = if self.channels[1].mode.clock_source == 1 {
            (if hblank { 1 } else { 0 }, hblank)
        } else {
            (cycles, false)
        };
        irqs[1] = self.channels[1].tick(timer1_cycles, timer1_sync);

        // Timer 2: System clock or system/8
        // Use accumulator to avoid losing fractional cycles
        let timer2_cycles = if self.channels[2].mode.clock_source == 1 {
            self.timer2_div_accum += cycles;
            let whole = self.timer2_div_accum / 8;
            self.timer2_div_accum %= 8;
            whole
        } else {
            self.timer2_div_accum = 0;
            cycles
        };
        irqs[2] = self.channels[2].tick(timer2_cycles, false);

        irqs
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_basic_counting() {
        let mut timer = TimerChannel::new(0);

        timer.tick(100, false);
        assert_eq!(timer.read_counter(), 100);

        timer.tick(50, false);
        assert_eq!(timer.read_counter(), 150);
    }

    #[test]
    fn test_timer_target_interrupt() {
        let mut timer = TimerChannel::new(0);

        // Set target to 100
        timer.write_target(100);

        // Enable target IRQ
        timer.write_mode(0x0010); // IRQ on target

        // Count to target
        let irq = timer.tick(100, false);

        assert!(irq);
        assert!(timer.reached_target);
        assert!(timer.irq_pending());
    }

    #[test]
    fn test_timer_reset_on_target() {
        let mut timer = TimerChannel::new(0);

        timer.write_target(50);
        timer.write_mode(0x0008); // Reset on target

        timer.tick(50, false);
        assert_eq!(timer.read_counter(), 0); // Should reset

        timer.tick(25, false);
        assert_eq!(timer.read_counter(), 25);
    }

    #[test]
    fn test_timer_overflow() {
        let mut timer = TimerChannel::new(0);

        timer.write_mode(0x0020); // IRQ on max
        timer.write_counter(0xFFFE);

        let irq = timer.tick(1, false);
        assert!(irq);
        assert!(timer.reached_max);
    }

    #[test]
    fn test_timer_mode_write_resets_counter() {
        let mut timer = TimerChannel::new(0);

        // Count to 100
        timer.tick(100, false);
        assert_eq!(timer.read_counter(), 100);

        // Writing mode should reset counter
        timer.write_mode(0x0000);
        assert_eq!(timer.read_counter(), 0);
    }

    #[test]
    fn test_timer_mode_read_resets_flags() {
        let mut timer = TimerChannel::new(0);

        timer.write_target(50);
        timer.write_mode(0x0010); // IRQ on target

        timer.tick(50, false);
        assert!(timer.reached_target);

        // Reading mode should reset flags
        let _mode = timer.read_mode();
        assert!(!timer.reached_target);
    }

    #[test]
    fn test_timer_irq_one_shot() {
        let mut timer = TimerChannel::new(0);

        timer.write_target(10);
        // IRQ on target, one-shot mode (IRQ repeat bit = 0)
        timer.write_mode(0x0010);

        // First target hit should trigger IRQ
        let irq1 = timer.tick(10, false);
        assert!(irq1);

        // Continue counting past target with reset disabled
        timer.write_mode(0x0010); // Reset counter
        let irq2 = timer.tick(20, false);
        assert!(irq2); // Should trigger again at target

        // But IRQ flag should not be set again in one-shot mode
        // unless it was cleared
    }

    #[test]
    fn test_timer_irq_repeat() {
        let mut timer = TimerChannel::new(0);

        timer.write_target(10);
        // IRQ on target, repeat mode
        timer.write_mode(0x0050); // bit 4 (target IRQ) + bit 6 (repeat)

        let irq1 = timer.tick(10, false);
        assert!(irq1);

        // Even if IRQ is still pending, repeat mode should allow re-trigger
        timer.write_mode(0x0058); // Reset counter + repeat + target IRQ
        let irq2 = timer.tick(10, false);
        assert!(irq2);
    }

    #[test]
    fn test_timers_tick_all_channels() {
        let mut timers = Timers::new();

        // Configure timer 0
        timers.channel_mut(0).write_target(100);
        timers.channel_mut(0).write_mode(0x0010); // IRQ on target

        // Configure timer 1
        timers.channel_mut(1).write_target(50);
        timers.channel_mut(1).write_mode(0x0010);

        // Configure timer 2
        timers.channel_mut(2).write_target(200);
        timers.channel_mut(2).write_mode(0x0010);

        // Tick all timers
        let irqs = timers.tick(100, false, false);

        // Timer 0 should have hit target
        assert!(irqs[0]);
        // Timer 1 should have hit target (50 < 100)
        assert!(irqs[1]);
        // Timer 2 should not have hit target yet
        assert!(!irqs[2]);
    }

    #[test]
    fn test_timer_clock_source_division() {
        let mut timers = Timers::new();

        // Timer 2 with clock source = 1 (system/8)
        timers.channel_mut(2).write_mode(0x0100); // Clock source bit 8

        timers.tick(80, false, false);

        // With /8 divider, 80 cycles should advance counter by 10
        assert_eq!(timers.channel(2).read_counter(), 10);
    }
}
