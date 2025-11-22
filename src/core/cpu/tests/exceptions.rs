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

use super::super::*;

#[test]
fn test_exception_saves_pc() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;

    cpu.exception(ExceptionCause::Syscall);

    // EPC should point to the instruction that caused the exception
    // Since step() updates pc to next_pc before executing, we need to subtract 4
    assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80000FFC);
}

#[test]
fn test_exception_in_delay_slot() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001004;
    cpu.next_pc = 0x80001008;
    cpu.in_branch_delay = true;

    cpu.exception(ExceptionCause::Syscall);

    // EPC should point to branch instruction (pc - 8)
    assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80000FFC);
    // BD flag should be set (bit 31 of CAUSE)
    assert_ne!(cpu.cop0.regs[COP0::CAUSE] & (1 << 31), 0);
}

#[test]
fn test_exception_not_in_delay_slot() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001004;
    cpu.next_pc = 0x80001008;
    cpu.in_branch_delay = false;

    cpu.exception(ExceptionCause::Syscall);

    // BD flag should not be set
    assert_eq!(cpu.cop0.regs[COP0::CAUSE] & (1 << 31), 0);
}

#[test]
fn test_exception_vector_bev0() {
    let mut cpu = CPU::new();
    cpu.cop0.regs[COP0::SR] &= !(1 << 22); // BEV = 0

    cpu.exception(ExceptionCause::Syscall);

    // Should jump to normal exception vector
    assert_eq!(cpu.pc, 0x80000080);
    assert_eq!(cpu.next_pc, 0x80000084);
}

#[test]
fn test_exception_vector_bev1() {
    let mut cpu = CPU::new();
    cpu.cop0.regs[COP0::SR] |= 1 << 22; // BEV = 1

    cpu.exception(ExceptionCause::Syscall);

    // Should jump to bootstrap exception vector
    assert_eq!(cpu.pc, 0xBFC00180);
    assert_eq!(cpu.next_pc, 0xBFC00184);
}

#[test]
fn test_exception_sets_cause() {
    let mut cpu = CPU::new();

    cpu.exception(ExceptionCause::Syscall);

    // Verify cause code is set correctly (bits [6:2])
    let cause_code = (cpu.cop0.regs[COP0::CAUSE] >> 2) & 0x1F;
    assert_eq!(cause_code, ExceptionCause::Syscall as u32);
}

#[test]
fn test_exception_mode_bits() {
    let mut cpu = CPU::new();

    // Set some initial mode bits
    cpu.cop0.regs[COP0::SR] = 0x00000003; // KUc=1, IEc=1

    cpu.exception(ExceptionCause::Syscall);

    // After exception, mode bits should be shifted left
    // KUc and IEc should be 0 (kernel mode, interrupts disabled)
    let sr = cpu.cop0.regs[COP0::SR];
    assert_eq!(sr & 0b11, 0); // Current mode should be kernel with interrupts disabled
    assert_eq!((sr >> 2) & 0b11, 0b11); // Previous mode should contain old values
}

#[test]
fn test_syscall_triggers_exception() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;
    cpu.cop0.regs[COP0::SR] &= !(1 << 22); // BEV = 0

    cpu.op_syscall(0).unwrap();

    // Should jump to exception handler
    assert_eq!(cpu.pc, 0x80000080);
    // Verify cause code
    let cause_code = (cpu.cop0.regs[COP0::CAUSE] >> 2) & 0x1F;
    assert_eq!(cause_code, ExceptionCause::Syscall as u32);
}

#[test]
fn test_break_triggers_exception() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;
    cpu.cop0.regs[COP0::SR] &= !(1 << 22); // BEV = 0

    cpu.op_break(0).unwrap();

    // Should jump to exception handler
    assert_eq!(cpu.pc, 0x80000080);
    // Verify cause code
    let cause_code = (cpu.cop0.regs[COP0::CAUSE] >> 2) & 0x1F;
    assert_eq!(cause_code, ExceptionCause::Breakpoint as u32);
}

#[test]
fn test_check_interrupts_disabled() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;

    // Disable interrupts
    cpu.cop0.regs[COP0::SR] &= !0x1;

    let original_pc = cpu.pc;
    cpu.check_interrupts(0xFF); // All interrupt sources pending

    // PC should not change (no exception)
    assert_eq!(cpu.pc, original_pc);
}

#[test]
fn test_check_interrupts_enabled_and_pending() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;

    // Enable interrupts and set interrupt mask
    // Bit 0: IE (Interrupt Enable) = 1
    // Bits [15:8]: IM (Interrupt Mask), bit 8 = 1 (interrupt source 0)
    cpu.cop0.regs[COP0::SR] = 0x0101; // IE=1, IM bit 0 = 1
    cpu.cop0.regs[COP0::SR] &= !(1 << 22); // BEV = 0

    cpu.check_interrupts(0x01); // Interrupt 0 pending

    // Should trigger interrupt exception
    assert_eq!(cpu.pc, 0x80000080);
    let cause_code = (cpu.cop0.regs[COP0::CAUSE] >> 2) & 0x1F;
    assert_eq!(cause_code, ExceptionCause::Interrupt as u32);
}

#[test]
fn test_check_interrupts_masked() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;

    // Enable interrupts but mask is 0
    cpu.cop0.regs[COP0::SR] = 0x0001; // IE=1, IM=0

    let original_pc = cpu.pc;
    cpu.check_interrupts(0xFF); // All interrupt sources pending

    // PC should not change (interrupts are masked)
    assert_eq!(cpu.pc, original_pc);
}

#[test]
fn test_check_interrupts_updates_cause() {
    let mut cpu = CPU::new();

    // Enable interrupts and set interrupt mask for bits 0 and 1
    cpu.cop0.regs[COP0::SR] = 0x0301; // IE=1, IM bits 0,1 = 1

    cpu.check_interrupts(0x03); // Interrupts 0 and 1 pending

    // CAUSE register should have pending interrupts in bits [15:8]
    let pending = (cpu.cop0.regs[COP0::CAUSE] >> 8) & 0xFF;
    assert_eq!(pending, 0x03);
}

#[test]
fn test_exception_clears_delay_slot() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;
    cpu.in_branch_delay = false;

    cpu.exception(ExceptionCause::Syscall);

    // Exception should clear the branch delay flag
    assert!(!cpu.in_branch_delay);
}

#[test]
fn test_exception_clears_load_delay() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80001000;
    cpu.next_pc = 0x80001004;
    cpu.set_reg_delayed(3, 100);

    cpu.exception(ExceptionCause::Syscall);

    // Load delay should be cleared
    assert!(cpu.load_delay.is_none());
}
#[test]
fn test_exception_handling() {
    let mut cpu = CPU::new();

    // Trigger an overflow exception
    cpu.exception(ExceptionCause::Overflow);

    // Check CAUSE register
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::Overflow as u32);

    // Check PC jumped to exception handler
    // BEV bit (bit 22) in initial SR (0x10900000) is not set, so should jump to normal handler
    assert_eq!(cpu.pc, 0x80000080);
}

#[test]
fn test_exception_handling_bootstrap() {
    let mut cpu = CPU::new();

    // Set BEV bit (bit 22) in Status Register
    cpu.cop0.regs[COP0::SR] |= 1 << 22;

    // Trigger an exception
    cpu.exception(ExceptionCause::Overflow);

    // Check PC jumped to bootstrap exception handler
    assert_eq!(cpu.pc, 0xBFC00180);
}

#[test]
fn test_exception_epc_saved() {
    let mut cpu = CPU::new();
    // In this core, self.pc points to (current_pc + 4) during execution.
    cpu.pc = 0x80001004;

    cpu.exception(ExceptionCause::Syscall);

    // Check EPC saved correctly
    assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80001000);
}

#[test]
fn test_exception_epc_and_bd_in_delay_slot() {
    let mut cpu = CPU::new();
    // Simulate executing a delay-slot instruction: pc = branch_pc + 8
    cpu.pc = 0x80001008;
    cpu.in_branch_delay = true;
    cpu.exception(ExceptionCause::Overflow);
    // EPC must point to branch instruction; BD must be set.
    assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80001000);
    assert_ne!(cpu.cop0.regs[COP0::CAUSE] & (1 << 31), 0);
}
