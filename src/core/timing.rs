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

//! Timing Event System
//!
//! This module implements a global timing event system for accurate emulation timing.
//! Based on DuckStation's timing event architecture.
//!
//! # Architecture
//!
//! The timing system uses a global tick counter to synchronize all emulator components.
//! Events are scheduled to run at specific tick counts, allowing for precise timing
//! of operations like:
//! - CD-ROM command delays
//! - GPU VBlank/HBlank timing
//! - Timer overflows
//! - Interrupt delivery
//!
//! # Example
//!
//! ```
//! use psrx::core::timing::TimingEventManager;
//!
//! let mut timing = TimingEventManager::new();
//!
//! // Register an event
//! let event_id = timing.register_event("Test Event");
//!
//! // Schedule it to run after 1000 cycles
//! timing.schedule(event_id, 1000);
//!
//! // Simulate CPU execution
//! timing.pending_ticks = 1000;
//! timing.run_events();
//! ```

/// Tick count type (relative time in CPU cycles)
pub type TickCount = i32;

/// Global tick counter type (absolute time in CPU cycles since reset)
pub type GlobalTicks = u64;

/// Event handle (identifier for registered events)
pub type EventHandle = usize;

/// Timing event
///
/// Represents a single scheduled event that will execute at a specific time.
/// Events can be one-shot or periodic (with automatic rescheduling).
#[derive(Debug)]
pub struct TimingEvent {
    /// Event ID (handle)
    pub id: EventHandle,

    /// Event name (for debugging)
    pub name: &'static str,

    /// Next execution time (global ticks)
    pub next_run_time: GlobalTicks,

    /// Last execution time (global ticks)
    pub last_run_time: GlobalTicks,

    /// Interval for periodic events (0 = one-shot)
    pub interval: TickCount,

    /// Whether this event is currently active
    pub active: bool,
}

impl TimingEvent {
    /// Create a new timing event
    ///
    /// # Arguments
    ///
    /// * `id` - Event ID (handle)
    /// * `name` - Event name for debugging
    /// * `interval` - Interval for periodic events (0 for one-shot)
    pub fn new(id: EventHandle, name: &'static str, interval: TickCount) -> Self {
        Self {
            id,
            name,
            next_run_time: 0,
            last_run_time: 0,
            interval,
            active: false,
        }
    }
}

/// Timing Event Manager
///
/// Manages the global timing system and schedules events for execution.
///
/// # Design
///
/// Based on DuckStation's timing event system:
/// - Global tick counter tracks absolute time
/// - Pending ticks accumulate CPU cycles between event checks
/// - Downcount determines when to run events
/// - Events are stored in a sorted vector (by next_run_time)
///
/// # Example
///
/// ```
/// use psrx::core::timing::TimingEventManager;
///
/// let mut timing = TimingEventManager::new();
/// let event = timing.register_event("MyEvent");
/// timing.schedule(event, 5000);
///
/// // Simulate 5000 CPU cycles
/// timing.pending_ticks = 5000;
/// timing.run_events();
/// ```
#[derive(Debug)]
pub struct TimingEventManager {
    /// Global tick counter (absolute time since reset)
    pub global_tick_counter: GlobalTicks,

    /// Tick counter at last event run
    pub event_run_tick_counter: GlobalTicks,

    /// Pending ticks (accumulated since last event run)
    pub pending_ticks: TickCount,

    /// Downcount (cycles until next event)
    pub downcount: TickCount,

    /// Registered events
    events: Vec<TimingEvent>,

    /// Frame target for execution control
    frame_target: Option<GlobalTicks>,
}

impl TimingEventManager {
    /// Create a new timing event manager
    ///
    /// Initializes the timing system with default values:
    /// - Global tick counter: 0
    /// - Pending ticks: 0
    /// - Downcount: maximum (no events scheduled)
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timing::TimingEventManager;
    ///
    /// let timing = TimingEventManager::new();
    /// assert_eq!(timing.global_tick_counter, 0);
    /// assert_eq!(timing.pending_ticks, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            global_tick_counter: 0,
            event_run_tick_counter: 0,
            pending_ticks: 0,
            downcount: i32::MAX,
            events: Vec::new(),
            frame_target: None,
        }
    }

    /// Register a new timing event
    ///
    /// Creates a new event and returns its handle. The event is initially inactive
    /// and must be scheduled with `schedule()` to run.
    ///
    /// # Arguments
    ///
    /// * `name` - Event name for debugging
    ///
    /// # Returns
    ///
    /// Event handle that can be used with `schedule()`, `activate()`, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timing::TimingEventManager;
    ///
    /// let mut timing = TimingEventManager::new();
    /// let event = timing.register_event("Test Event");
    /// ```
    pub fn register_event(&mut self, name: &'static str) -> EventHandle {
        let handle = self.events.len();
        self.events.push(TimingEvent::new(handle, name, 0));
        handle
    }

    /// Register a periodic event with automatic rescheduling
    ///
    /// Creates a new event that will automatically reschedule itself
    /// after each execution.
    ///
    /// # Arguments
    ///
    /// * `name` - Event name for debugging
    /// * `interval` - Interval between executions (in CPU cycles)
    ///
    /// # Returns
    ///
    /// Event handle
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timing::TimingEventManager;
    ///
    /// let mut timing = TimingEventManager::new();
    /// // VBlank every 564,480 cycles (60 Hz)
    /// let vblank = timing.register_periodic_event("VBlank", 564_480);
    /// ```
    pub fn register_periodic_event(
        &mut self,
        name: &'static str,
        interval: TickCount,
    ) -> EventHandle {
        let handle = self.events.len();
        self.events.push(TimingEvent::new(handle, name, interval));
        handle
    }

    /// Schedule an event to run after a specific number of cycles
    ///
    /// Activates the event and schedules it to run after `ticks` CPU cycles
    /// from the current time.
    ///
    /// # Arguments
    ///
    /// * `handle` - Event handle (from `register_event()`)
    /// * `ticks` - Number of CPU cycles until execution
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timing::TimingEventManager;
    ///
    /// let mut timing = TimingEventManager::new();
    /// let event = timing.register_event("Delayed Command");
    /// timing.schedule(event, 7000);  // Run after 7000 cycles
    /// ```
    pub fn schedule(&mut self, handle: EventHandle, ticks: TickCount) {
        // Calculate next run time first (before borrowing event)
        let current_time = self.get_current_time();

        let event = &mut self.events[handle];
        event.next_run_time = current_time + ticks as GlobalTicks;
        event.last_run_time = current_time;
        event.active = true;

        // Resort events by next_run_time
        self.sort_events();

        // Update downcount
        self.update_downcount();
    }

    /// Deactivate an event
    ///
    /// Removes the event from the active event list.
    ///
    /// # Arguments
    ///
    /// * `handle` - Event handle
    pub fn deactivate(&mut self, handle: EventHandle) {
        self.events[handle].active = false;
        self.update_downcount();
    }

    /// Get current time (global_tick_counter + pending_ticks)
    ///
    /// # Returns
    ///
    /// Current global time in CPU cycles
    #[inline]
    fn get_current_time(&self) -> GlobalTicks {
        self.global_tick_counter + self.pending_ticks as GlobalTicks
    }

    /// Sort events by next_run_time (ascending order)
    ///
    /// Uses a simple bubble sort since the number of events is small (<50).
    /// Active events are sorted to the front.
    fn sort_events(&mut self) {
        // Simple selection sort for small number of events
        let len = self.events.len();
        for i in 0..len {
            for j in (i + 1)..len {
                let (active_i, time_i) = {
                    let event_i = &self.events[i];
                    (event_i.active, event_i.next_run_time)
                };
                let (active_j, time_j) = {
                    let event_j = &self.events[j];
                    (event_j.active, event_j.next_run_time)
                };

                // Sort active events first, then by next_run_time
                let should_swap = match (active_i, active_j) {
                    (false, true) => true,           // Inactive before active
                    (true, false) => false,          // Active before inactive
                    (true, true) => time_i > time_j, // Both active: sort by time
                    (false, false) => false,         // Both inactive: don't care
                };

                if should_swap {
                    self.events.swap(i, j);
                }
            }
        }
    }

    /// Update downcount to the next event's run time
    ///
    /// Calculates cycles until the next active event should run.
    /// If no events are active, sets downcount to maximum.
    pub fn update_downcount(&mut self) {
        // Find first active event (events are sorted)
        if let Some(event) = self.events.iter().find(|e| e.active) {
            let cycles_until_event = event.next_run_time.saturating_sub(self.global_tick_counter);
            self.downcount = cycles_until_event.min(i32::MAX as u64) as i32;
        } else {
            self.downcount = i32::MAX;
        }
    }

    /// Run pending timing events
    ///
    /// Advances global time by pending_ticks and executes all events
    /// whose execution time has been reached.
    ///
    /// Returns a vector of event handles that were executed, allowing
    /// the caller to process them.
    ///
    /// # Returns
    ///
    /// Vector of event handles that were triggered
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::timing::TimingEventManager;
    ///
    /// let mut timing = TimingEventManager::new();
    /// let event = timing.register_event("Test");
    /// timing.schedule(event, 1000);
    ///
    /// timing.pending_ticks = 1000;
    /// let triggered = timing.run_events();
    /// assert_eq!(triggered.len(), 1);
    /// assert_eq!(triggered[0], event);
    /// ```
    pub fn run_events(&mut self) -> Vec<EventHandle> {
        // Advance global time
        let new_global_ticks = self.event_run_tick_counter + self.pending_ticks as GlobalTicks;
        self.pending_ticks = 0;

        let mut triggered_events = Vec::new();

        // Execute all events whose time has come
        self.global_tick_counter = new_global_ticks;

        // Collect events to execute (to avoid mutable borrow issues)
        let mut events_to_execute = Vec::new();
        for (handle, event) in self.events.iter().enumerate() {
            if event.active && event.next_run_time <= self.global_tick_counter {
                events_to_execute.push(handle);
            }
        }

        // Execute events
        for handle in &events_to_execute {
            let event = &mut self.events[*handle];
            let ticks_late = (self.global_tick_counter - event.next_run_time) as TickCount;
            let event_id = event.id; // Save ID before mutable access

            log::trace!(
                "Timing: Event '{}' executed (late: {} ticks)",
                event.name,
                ticks_late
            );

            // Reschedule periodic events
            if event.interval > 0 {
                event.last_run_time = event.next_run_time;
                event.next_run_time += event.interval as GlobalTicks;
            } else {
                // One-shot event: deactivate
                event.active = false;
            }

            triggered_events.push(event_id); // Push original ID, not current index
        }

        // Resort events if any were executed
        if !events_to_execute.is_empty() {
            self.sort_events();
        }

        // Update downcount for next event
        self.update_downcount();

        // Update event run tick counter
        self.event_run_tick_counter = self.global_tick_counter;

        triggered_events
    }

    /// Set frame target for execution control
    ///
    /// Used by the system to stop CPU execution after a specific number of cycles.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of cycles for this frame
    pub fn set_frame_target(&mut self, cycles: GlobalTicks) {
        self.frame_target = Some(self.global_tick_counter + cycles);
    }

    /// Check if execution should exit (frame target reached)
    ///
    /// # Returns
    ///
    /// true if frame target has been reached
    pub fn should_exit_loop(&self) -> bool {
        if let Some(target) = self.frame_target {
            self.global_tick_counter >= target
        } else {
            false
        }
    }

    /// Reset the timing system
    ///
    /// Clears all state and deactivates all events.
    pub fn reset(&mut self) {
        self.global_tick_counter = 0;
        self.event_run_tick_counter = 0;
        self.pending_ticks = 0;
        self.downcount = i32::MAX;
        self.frame_target = None;

        for event in &mut self.events {
            event.active = false;
            event.next_run_time = 0;
            event.last_run_time = 0;
        }
    }
}

impl Default for TimingEventManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_manager_initialization() {
        let timing = TimingEventManager::new();
        assert_eq!(timing.global_tick_counter, 0);
        assert_eq!(timing.pending_ticks, 0);
        assert_eq!(timing.downcount, i32::MAX);
    }

    #[test]
    fn test_event_registration() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test Event");
        assert_eq!(event, 0);
        assert_eq!(timing.events.len(), 1);
        assert_eq!(timing.events[0].name, "Test Event");
        assert!(!timing.events[0].active);
    }

    #[test]
    fn test_event_scheduling() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test");

        timing.schedule(event, 1000);

        assert!(timing.events[0].active);
        assert_eq!(timing.events[0].next_run_time, 1000);
        assert_eq!(timing.downcount, 1000);
    }

    #[test]
    fn test_single_event_execution() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test");

        timing.schedule(event, 1000);

        // Advance time
        timing.pending_ticks = 1000;
        let triggered = timing.run_events();

        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], event);
        assert_eq!(timing.global_tick_counter, 1000);
        assert!(!timing.events[0].active); // One-shot deactivated
    }

    #[test]
    fn test_multiple_events_in_order() {
        let mut timing = TimingEventManager::new();
        let event1 = timing.register_event("Event 1");
        let event2 = timing.register_event("Event 2");
        let event3 = timing.register_event("Event 3");

        timing.schedule(event1, 1000);
        timing.schedule(event2, 500);
        timing.schedule(event3, 1500);

        // Execute at T=500
        timing.pending_ticks = 500;
        let triggered = timing.run_events();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], event2);

        // Execute at T=1000
        timing.pending_ticks = 500;
        let triggered = timing.run_events();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], event1);

        // Execute at T=1500
        timing.pending_ticks = 500;
        let triggered = timing.run_events();
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0], event3);
    }

    #[test]
    fn test_periodic_event() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_periodic_event("Periodic", 1000);

        timing.schedule(event, 1000);

        // First execution
        timing.pending_ticks = 1000;
        let triggered = timing.run_events();
        assert_eq!(triggered.len(), 1);
        assert!(timing.events[0].active); // Still active
        assert_eq!(timing.events[0].next_run_time, 2000);

        // Second execution
        timing.pending_ticks = 1000;
        let triggered = timing.run_events();
        assert_eq!(triggered.len(), 1);
        assert_eq!(timing.events[0].next_run_time, 3000);
    }

    #[test]
    fn test_event_deactivation() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test");

        timing.schedule(event, 1000);
        assert!(timing.events[0].active);

        timing.deactivate(event);
        assert!(!timing.events[0].active);
        assert_eq!(timing.downcount, i32::MAX);
    }

    #[test]
    fn test_late_event_execution() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test");

        timing.schedule(event, 1000);

        // Execute late (at 1500 instead of 1000)
        timing.pending_ticks = 1500;
        let triggered = timing.run_events();

        assert_eq!(triggered.len(), 1);
        assert_eq!(timing.global_tick_counter, 1500);
    }

    #[test]
    fn test_frame_target() {
        let mut timing = TimingEventManager::new();

        timing.set_frame_target(564_480);
        assert!(!timing.should_exit_loop());

        timing.pending_ticks = 564_480;
        timing.run_events();
        assert!(timing.should_exit_loop());
    }

    #[test]
    fn test_reset() {
        let mut timing = TimingEventManager::new();
        let event = timing.register_event("Test");

        timing.schedule(event, 1000);
        timing.pending_ticks = 500;
        timing.run_events();

        timing.reset();

        assert_eq!(timing.global_tick_counter, 0);
        assert_eq!(timing.pending_ticks, 0);
        assert!(!timing.events[0].active);
    }
}
