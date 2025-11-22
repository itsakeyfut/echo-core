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

//! Interrupt controller integration tests

use super::super::*;

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
