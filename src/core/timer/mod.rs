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

use super::timing::EventHandle;

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
    /// - Timer 0: bit 8: 0=system clock, 1=pixel clock (values 0,2=sys, 1,3=pixel)
    /// - Timer 1: bit 8: 0=system clock, 1=hblank (values 0,2=sys, 1,3=hblank)
    /// - Timer 2: bit 9: 0=system clock, 1=system/8 (values 0,1=sys, 2,3=sys/8)
    pub clock_source: u8,
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

    /// Last sync signal state (for edge detection)
    last_sync: bool,

    /// Sync mode 3 latch (set on first sync edge, cleared when sync disabled)
    sync_latched: bool,

    /// Overflow timing event handle
    overflow_event: Option<EventHandle>,

    /// Interrupt pending flag (for event-driven timing)
    interrupt_pending: bool,

    /// Flag indicating that the timer needs rescheduling
    needs_reschedule: bool,
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
            last_sync: false,
            sync_latched: false,
            overflow_event: None,
            interrupt_pending: false,
            needs_reschedule: false,
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
    /// resets the IRQ flag, reached_target, and reached_max flags.
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
        self.irq_flag = false;

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
        self.last_sync = false;
        self.sync_latched = false;

        // Mark for rescheduling (event-driven timing)
        self.needs_reschedule = true;

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

        // Mark for rescheduling (event-driven timing)
        self.needs_reschedule = true;

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

        // Detect rising edge of sync signal (transition from false to true)
        let rising_edge = !self.last_sync && sync_signal;

        // Handle sync mode effects on rising edge
        if self.mode.sync_enable && rising_edge {
            match self.mode.sync_mode {
                1 | 2 => {
                    // Mode 1: Reset counter on sync (free-run, reset on blank)
                    // Mode 2: Reset counter on sync (count during blank)
                    self.counter = 0;
                }
                3 => {
                    // Mode 3: Latch on first sync edge, then free-run
                    self.sync_latched = true;
                }
                _ => {}
            }
        }

        // Update last_sync for next edge detection
        self.last_sync = sync_signal;

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
    ///
    /// # Sync Mode Behavior (per PSX-SPX)
    ///
    /// - Mode 0: Pause during sync (count when sync_signal is false)
    /// - Mode 1: Free-run (count always), reset on sync edge
    /// - Mode 2: Count during sync window (count when sync_signal is true)
    /// - Mode 3: Pause until first sync, then free-run (use sync_latched)
    ///
    /// Timer 2 special case: modes 0 and 3 halt counting entirely
    fn should_count(&self, sync_signal: bool) -> bool {
        if !self.mode.sync_enable {
            return true; // Free-run mode
        }

        // Timer 2 has special behavior
        if self.channel_id == 2 {
            // Timer 2: only modes 1 and 2 allow counting
            return matches!(self.mode.sync_mode, 1 | 2);
        }

        // Timer 0 and 1 sync mode behavior
        match self.mode.sync_mode {
            0 => !sync_signal,      // Pause during sync
            1 => true,              // Free-run (reset on edge handled in tick)
            2 => sync_signal,       // Count during sync window
            3 => self.sync_latched, // Pause until first sync edge
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
    /// * `vblank` - Vertical blank signal state
    ///
    /// # Returns
    ///
    /// Array of IRQ flags for each timer channel
    pub fn tick(&mut self, cycles: u32, hblank: bool, vblank: bool) -> [bool; 3] {
        let mut irqs = [false; 3];

        // Timer 0: System clock or pixel clock (simplified as system clock)
        irqs[0] = self.channels[0].tick(cycles, false);

        // Timer 1: System clock or hblank
        // Clock source determines pulse/count rate (HBlank vs system clock)
        // Sync signal is ALWAYS VBlank regardless of clock source
        // Check low bit (bit 8): values 1 and 3 both select HBlank mode
        let (timer1_cycles, timer1_sync) = if self.channels[1].mode.clock_source & 0x01 != 0 {
            (if hblank { 1 } else { 0 }, vblank)
        } else {
            (cycles, vblank)
        };
        irqs[1] = self.channels[1].tick(timer1_cycles, timer1_sync);

        // Timer 2: System clock or system/8
        // Use accumulator to avoid losing fractional cycles
        // Check high bit (bit 9): values 2 and 3 both select system/8 mode
        let timer2_cycles = if self.channels[2].mode.clock_source & 0x02 != 0 {
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

    /// Register timing events for timer overflow
    ///
    /// This should be called during system initialization to register timer
    /// timing events with the timing manager.
    ///
    /// # Arguments
    ///
    /// * `timing` - Timing event manager
    pub fn register_events(&mut self, timing: &mut super::timing::TimingEventManager) {
        const EVENT_NAMES: [&str; 3] = ["Timer0 Overflow", "Timer1 Overflow", "Timer2 Overflow"];

        for i in 0..3 {
            self.channels[i].overflow_event = Some(timing.register_event(EVENT_NAMES[i]));
            log::debug!("Timer {}: Registered overflow event", i);
        }

        log::info!("Timers: Timing events registered for all 3 channels");
    }

    /// Process timer timing events
    ///
    /// This should be called by System when timing events fire.
    /// Also handles rescheduling when mode/target changes occur.
    ///
    /// # Arguments
    ///
    /// * `timing` - Timing event manager
    /// * `triggered_events` - List of event handles that have fired
    pub fn process_events(
        &mut self,
        timing: &mut super::timing::TimingEventManager,
        triggered_events: &[EventHandle],
    ) {
        // Process fired overflow events
        for i in 0..3 {
            if let Some(handle) = self.channels[i].overflow_event {
                if triggered_events.contains(&handle) {
                    self.timer_overflow_callback(i, timing);
                }
            }
        }

        // Handle pending rescheduling (from mode/target writes)
        for i in 0..3 {
            if self.channels[i].needs_reschedule {
                self.channels[i].needs_reschedule = false;
                self.reschedule_timer(i, timing);
            }
        }
    }

    /// Timer overflow callback (called when overflow_event fires)
    ///
    /// Handles timer overflow and reschedules the next overflow event.
    ///
    /// # Arguments
    ///
    /// * `channel` - Timer channel index (0-2)
    /// * `timing` - Timing event manager
    fn timer_overflow_callback(
        &mut self,
        channel: usize,
        timing: &mut super::timing::TimingEventManager,
    ) {
        let ch = &mut self.channels[channel];

        // Reset counter to 0 if reset_on_target is enabled
        if ch.mode.reset_on_target {
            ch.counter = 0;
            ch.reached_target = true;
        } else {
            // Otherwise wrap around
            ch.counter = ch.counter.wrapping_add(1);
        }

        // Set interrupt flags
        if ch.mode.irq_on_target || ch.mode.irq_on_max {
            ch.interrupt_pending = true;
            ch.irq_flag = true;
        }

        log::trace!("Timer {}: Overflow event fired", channel);

        // Reschedule for next overflow
        self.reschedule_timer(channel, timing);
    }

    /// Reschedule timer overflow event
    ///
    /// Calculates when the next overflow will occur and schedules the event.
    ///
    /// # Arguments
    ///
    /// * `channel` - Timer channel index (0-2)
    /// * `timing` - Timing event manager
    fn reschedule_timer(&mut self, channel: usize, timing: &mut super::timing::TimingEventManager) {
        let ch = &self.channels[channel];

        // Get the event handle
        let Some(handle) = ch.overflow_event else {
            return;
        };

        // Determine the target value
        let target = if ch.mode.irq_on_target && ch.target > 0 {
            ch.target
        } else if ch.mode.irq_on_max {
            0xFFFF
        } else {
            // No interrupt conditions enabled, don't schedule
            timing.deactivate(handle);
            return;
        };

        // Calculate cycles until overflow
        let remaining = target.saturating_sub(ch.counter) as i32;
        if remaining <= 0 {
            // Already at or past target, schedule immediately
            timing.schedule(handle, 1);
            return;
        }

        // Get clock divider based on clock source
        let divider = self.get_clock_divider(channel);
        let cycles_until_overflow = remaining * divider;

        timing.schedule(handle, cycles_until_overflow);
        log::trace!(
            "Timer {}: Scheduled overflow in {} cycles (counter={}, target={}, divider={})",
            channel,
            cycles_until_overflow,
            ch.counter,
            target,
            divider
        );
    }

    /// Get clock divider for a timer channel
    ///
    /// Returns the number of CPU cycles per timer tick based on the clock source.
    ///
    /// # Arguments
    ///
    /// * `channel` - Timer channel index (0-2)
    ///
    /// # Returns
    ///
    /// Number of CPU cycles per timer tick
    fn get_clock_divider(&self, channel: usize) -> i32 {
        let ch = &self.channels[channel];

        match channel {
            0 => {
                // Timer 0: system clock or pixel clock
                if ch.mode.clock_source & 1 != 0 {
                    8 // Pixel clock (simplified, approximately 1/8 of CPU clock)
                } else {
                    1 // System clock
                }
            }
            1 => {
                // Timer 1: system clock or hblank
                if ch.mode.clock_source & 1 != 0 {
                    2146 // HBlank (cycles per scanline)
                } else {
                    1 // System clock
                }
            }
            2 => {
                // Timer 2: system clock or system clock / 8
                if ch.mode.clock_source & 2 != 0 {
                    8 // System clock / 8
                } else {
                    1 // System clock
                }
            }
            _ => 1,
        }
    }

    /// Poll timer interrupt flags
    ///
    /// Returns interrupt flags and clears them.
    /// Replaces the return value of tick() for event-driven timing.
    ///
    /// # Returns
    ///
    /// Array of 3 booleans indicating interrupt status for each timer
    pub fn poll_interrupts(&mut self) -> [bool; 3] {
        let mut irqs = [false; 3];

        for i in 0..3 {
            irqs[i] = self.channels[i].interrupt_pending;
            self.channels[i].interrupt_pending = false;
        }

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

        // Timer 2 with clock source = 2 (system/8 mode, bit 9 set)
        timers.channel_mut(2).write_mode(0x0200); // Clock source bit 9

        timers.tick(80, false, false);

        // With /8 divider, 80 cycles should advance counter by 10
        assert_eq!(timers.channel(2).read_counter(), 10);
    }
}
