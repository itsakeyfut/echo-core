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

use super::*;
use crate::core::memory::Bus;

#[test]
fn test_cpu_initialization() {
    let cpu = CPU::new();
    assert_eq!(cpu.pc, 0xBFC00000);
    assert_eq!(cpu.next_pc, 0xBFC00004);
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn test_register_r0_is_hardwired() {
    let mut cpu = CPU::new();
    cpu.set_reg(0, 0xDEADBEEF);
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn test_register_read_write() {
    let mut cpu = CPU::new();
    cpu.set_reg(5, 0x12345678);
    assert_eq!(cpu.reg(5), 0x12345678);
}

#[test]
fn test_load_delay_slot() {
    let mut cpu = CPU::new();
    cpu.set_reg_delayed(3, 100);

    // Value not yet visible
    assert_eq!(cpu.reg(3), 0);

    // Execute load delay
    cpu.set_reg_delayed(4, 200);

    // Now r3 should have the value
    assert_eq!(cpu.reg(3), 100);
}

#[test]
fn test_cpu_reset() {
    let mut cpu = CPU::new();

    // Modify some state
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.pc = 0x80000000;
    cpu.hi = 0x12345678;
    cpu.lo = 0x87654321;

    // Reset
    cpu.reset();

    // Verify all state is reset
    assert_eq!(cpu.reg(1), 0);
    assert_eq!(cpu.pc, 0xBFC00000);
    assert_eq!(cpu.next_pc, 0xBFC00004);
    assert_eq!(cpu.hi, 0);
    assert_eq!(cpu.lo, 0);
}

#[test]
fn test_cop0_initialization() {
    let cpu = CPU::new();
    assert_eq!(cpu.cop0.regs[COP0::SR], 0x10900000);
    assert_eq!(cpu.cop0.regs[COP0::PRID], 0x00000002);
}

#[test]
fn test_multiple_registers() {
    let mut cpu = CPU::new();

    // Test writing to multiple registers
    for i in 1..32 {
        cpu.set_reg(i, i as u32 * 100);
    }

    // Verify all values
    for i in 1..32 {
        assert_eq!(cpu.reg(i), i as u32 * 100);
    }

    // r0 should still be 0
    assert_eq!(cpu.reg(0), 0);
}

// ==================== Exception Handling Tests ====================

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
fn test_rfe_restores_mode() {
    let mut cpu = CPU::new();

    // Simulate exception (shifts mode left by setting mode bits manually)
    cpu.cop0.regs[COP0::SR] = 0x0000000C; // Mode bits = 0b001100 (previous mode = 0b11)

    cpu.op_rfe(0).unwrap();

    // RFE should shift mode bits right
    let sr = cpu.cop0.regs[COP0::SR];
    assert_eq!(sr & 0x3F, 0x03); // Mode bits should be 0b000011
}

#[test]
fn test_mfc0_reads_cop0_register() {
    let mut cpu = CPU::new();

    // Set a value in COP0 SR register
    cpu.cop0.regs[COP0::SR] = 0x12345678;

    // MFC0 $t0, $12 (move SR to register 8)
    // Encoding: bits [25:21] = 0x00 (MFC0), bits [20:16] = rt, bits [15:11] = rd
    let instruction = (0x10 << 26) | (8 << 16) | (12 << 11);
    cpu.op_mfc0(instruction).unwrap();

    // Execute the load delay
    cpu.set_reg_delayed(9, 0); // Dummy operation to trigger delay

    // Register 8 should now have the SR value
    assert_eq!(cpu.reg(8), 0x12345678);
}

#[test]
fn test_mtc0_writes_cop0_register() {
    let mut cpu = CPU::new();

    // Set a value in general register
    cpu.set_reg(8, 0x87654321);

    // MTC0 $t0, $12 (move register 8 to SR)
    // Encoding: bits [25:21] = 0x04 (MTC0), bits [20:16] = rt, bits [15:11] = rd
    let instruction = (0x10 << 26) | (0x04 << 21) | (8 << 16) | (12 << 11);
    cpu.op_mtc0(instruction).unwrap();

    // SR should now have the value from register 8
    assert_eq!(cpu.cop0.regs[COP0::SR], 0x87654321);
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
fn test_load_delay_chain() {
    let mut cpu = CPU::new();

    // Chain multiple load delays
    cpu.set_reg_delayed(1, 10);
    assert_eq!(cpu.reg(1), 0);

    cpu.set_reg_delayed(2, 20);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 0);

    cpu.set_reg_delayed(3, 30);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 20);
    assert_eq!(cpu.reg(3), 0);

    // Final load delay to flush
    cpu.set_reg_delayed(4, 40);
    assert_eq!(cpu.reg(1), 10);
    assert_eq!(cpu.reg(2), 20);
    assert_eq!(cpu.reg(3), 30);
    assert_eq!(cpu.reg(4), 0);
}

#[test]
fn test_load_delay_r0_ignored() {
    let mut cpu = CPU::new();

    // Load delay to r0 should be ignored
    cpu.set_reg_delayed(0, 100);
    cpu.set_reg_delayed(1, 200);

    // r0 should still be 0, r1 should be 0 (delay not executed yet)
    assert_eq!(cpu.reg(0), 0);
    assert_eq!(cpu.reg(1), 0);

    // Execute another load to flush
    cpu.set_reg_delayed(2, 300);
    assert_eq!(cpu.reg(0), 0);
    assert_eq!(cpu.reg(1), 200);
}

// === Instruction Decode Tests ===

#[test]
fn test_decode_r_type() {
    use super::decode::decode_r_type;

    // ADD r3, r1, r2 -> 0x00221820
    let instr = 0x00221820;
    let (rs, rt, rd, shamt, funct) = decode_r_type(instr);
    assert_eq!(rs, 1);
    assert_eq!(rt, 2);
    assert_eq!(rd, 3);
    assert_eq!(shamt, 0);
    assert_eq!(funct, 0x20);
}

#[test]
fn test_decode_i_type() {
    use super::decode::decode_i_type;

    // ADDI r2, r1, 100 -> 0x20220064
    let instr = 0x20220064;
    let (op, rs, rt, imm) = decode_i_type(instr);
    assert_eq!(op, 0x08);
    assert_eq!(rs, 1);
    assert_eq!(rt, 2);
    assert_eq!(imm, 100);
}

#[test]
fn test_decode_j_type() {
    use super::decode::decode_j_type;

    // J 0x100000 -> 0x08040000
    let instr = 0x08040000;
    let (op, target) = decode_j_type(instr);
    assert_eq!(op, 0x02);
    assert_eq!(target, 0x040000);
}

// === Instruction Execution Tests ===

#[test]
fn test_instruction_fetch() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM instead of BIOS
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // Place a NOP in RAM
    bus.write32(0x80000000, 0x00000000).unwrap();

    let cycles = cpu.step(&mut bus).unwrap();
    assert_eq!(cycles, 1);
    assert_eq!(cpu.pc, 0x80000004);
}

#[test]
fn test_lui_instruction() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // LUI r5, 0x1234 -> 0x3C051234
    bus.write32(0x80000000, 0x3C051234).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(5), 0x12340000);
}

#[test]
fn test_sll_instruction() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // Set up r1 with a value to shift
    cpu.set_reg(1, 0x00000001);

    // SLL r2, r1, 4 -> Shift r1 left by 4, store in r2
    // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=4, funct=0
    // = (1 << 16) | (2 << 11) | (4 << 6) = 0x00011100
    bus.write32(0x80000000, 0x00011100).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0x00000010); // 1 << 4 = 16
}

#[test]
fn test_nop_instruction() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // Set up some registers
    cpu.set_reg(1, 0x12345678);
    cpu.set_reg(2, 0xABCDEF00);

    // NOP -> 0x00000000 (SLL with all fields = 0)
    bus.write32(0x80000000, 0x00000000).unwrap();

    cpu.step(&mut bus).unwrap();

    // All registers should be unchanged
    assert_eq!(cpu.reg(1), 0x12345678);
    assert_eq!(cpu.reg(2), 0xABCDEF00);
}

#[test]
fn test_pc_increment() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    let initial_pc = cpu.pc;

    // Execute NOP
    bus.write32(initial_pc, 0x00000000).unwrap();
    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.pc, initial_pc + 4);
    assert_eq!(cpu.next_pc, initial_pc + 8);
}

#[test]
fn test_delay_slot_pc_handling() {
    let mut cpu = CPU::new();

    // Simulate branch
    cpu.branch(100); // Branch forward by 100 bytes

    // Verify next_pc is updated
    let expected_target = cpu.next_pc;
    assert!(expected_target != cpu.pc + 4);
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_j_instruction() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // J 0x100000 -> 0x08040000
    // Target address = (0x80000000 & 0xF0000000) | (0x040000 << 2)
    //                = 0x80000000 | 0x00100000 = 0x80100000
    bus.write32(0x80000000, 0x08040000).unwrap();

    cpu.step(&mut bus).unwrap();

    // PC should be updated to point after the delay slot
    // next_pc should be the jump target
    assert_eq!(cpu.next_pc, 0x80100000);
}

#[test]
fn test_jal_instruction() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // JAL 0x100000 -> 0x0C040000
    bus.write32(0x80000000, 0x0C040000).unwrap();

    let initial_pc = cpu.pc;
    cpu.step(&mut bus).unwrap();

    // r31 should contain return address (address after delay slot)
    assert_eq!(cpu.reg(31), initial_pc + 8);

    // next_pc should be the jump target
    assert_eq!(cpu.next_pc, 0x80100000);
}

#[test]
fn test_multiple_instructions() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // LUI r1, 0x1234
    bus.write32(0x80000000, 0x3C011234).unwrap();
    // NOP
    bus.write32(0x80000004, 0x00000000).unwrap();
    // LUI r2, 0x5678
    bus.write32(0x80000008, 0x3C025678).unwrap();

    // Execute first instruction
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.reg(1), 0x12340000);
    assert_eq!(cpu.pc, 0x80000004);

    // Execute second instruction (NOP)
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.pc, 0x80000008);

    // Execute third instruction
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.reg(2), 0x56780000);
    assert_eq!(cpu.pc, 0x8000000C);
}

#[test]
fn test_branch_helper() {
    let mut cpu = CPU::new();

    let initial_next_pc = cpu.next_pc;

    // Branch forward by 100 bytes
    cpu.branch(100);

    assert_eq!(cpu.next_pc, initial_next_pc.wrapping_add(100));
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_branch_backward() {
    let mut cpu = CPU::new();

    let initial_next_pc = cpu.next_pc;

    // Branch backward by 100 bytes
    cpu.branch(-100);

    assert_eq!(cpu.next_pc, initial_next_pc.wrapping_sub(100));
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_pc_accessor() {
    let cpu = CPU::new();
    assert_eq!(cpu.pc(), 0xBFC00000);
}

#[test]
fn test_sll_zero_shift() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    cpu.set_reg(1, 0x12345678);

    // SLL r2, r1, 0 -> Should copy r1 to r2
    // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=0, funct=0
    // = (1 << 16) | (2 << 11) = 0x00011000
    bus.write32(0x80000000, 0x00011000).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0x12345678);
}

#[test]
fn test_sll_max_shift() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set CPU to execute from RAM
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    cpu.set_reg(1, 0xFFFFFFFF);

    // SLL r2, r1, 31 -> Shift left by 31 bits
    // Encoding: op=0, rs=0, rt=1(r1), rd=2(r2), shamt=31, funct=0
    // = (1 << 16) | (2 << 11) | (31 << 6) = 0x000117C0
    bus.write32(0x80000000, 0x000117C0).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0x80000000);
}

#[test]
fn test_instruction_at_different_pc() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set PC to RAM instead of BIOS
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;

    // LUI r3, 0xABCD
    bus.write32(0x80000000, 0x3C03ABCD).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(3), 0xABCD0000);
    assert_eq!(cpu.pc, 0x80000004);
}

// === Arithmetic Instruction Tests ===

#[test]
fn test_add_no_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 10);
    cpu.set_reg(2, 20);

    cpu.op_add(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 30);
}

#[test]
fn test_add_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x7FFFFFFF); // Max positive i32
    cpu.set_reg(2, 1);

    cpu.op_add(1, 2, 3).unwrap();

    // Should trigger overflow exception
    // Check that exception was raised (via COP0 CAUSE register)
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::Overflow as u32);
}

#[test]
fn test_add_negative_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x80000000_u32); // Min negative i32
    cpu.set_reg(2, 0xFFFFFFFF_u32); // -1 as u32

    cpu.op_add(1, 2, 3).unwrap();

    // Should trigger overflow exception
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::Overflow as u32);
}

#[test]
fn test_addu_no_exception_on_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.set_reg(2, 1);

    cpu.op_addu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 0); // Wraps around
}

#[test]
fn test_addu_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 200);

    cpu.op_addu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 300);
}

#[test]
fn test_addi_basic() {
    use crate::core::memory::Bus;

    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 100);

    // ADDI r2, r1, 50 -> 0x20220032
    bus.write32(0x80000000, 0x20220032).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 150);
}

#[test]
fn test_addi_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x7FFFFFFF); // Max positive i32

    // ADDI r2, r1, 1 -> 0x20220001
    let instr = 0x20220001;
    cpu.op_addi(instr).unwrap();

    // Should trigger overflow exception
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::Overflow as u32);
}

#[test]
fn test_addiu_sign_extension() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x00000100);

    // ADDIU r2, r1, -1 (0xFFFF sign extends to 0xFFFFFFFF)
    let instr = 0x2422FFFF; // addiu r2, r1, -1
    cpu.op_addiu(instr).unwrap();

    assert_eq!(cpu.reg(2), 0x000000FF);
}

#[test]
fn test_addiu_no_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);

    // ADDIU r2, r1, 1 -> 0x24220001
    let instr = 0x24220001;
    cpu.op_addiu(instr).unwrap();

    assert_eq!(cpu.reg(2), 0); // Wraps around
}

#[test]
fn test_sub() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 30);

    cpu.op_sub(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 70);
}

#[test]
fn test_sub_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x80000000_u32); // Min negative i32
    cpu.set_reg(2, 1);

    cpu.op_sub(1, 2, 3).unwrap();

    // Should trigger overflow exception
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::Overflow as u32);
}

#[test]
fn test_subu_underflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 10);
    cpu.set_reg(2, 20);

    cpu.op_subu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 0xFFFFFFF6_u32); // -10 as u32
}

#[test]
fn test_subu_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 30);

    cpu.op_subu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 70);
}

#[test]
fn test_slt_true() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 10_u32.wrapping_neg()); // -10 as u32
    cpu.set_reg(2, 5);

    cpu.op_slt(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 1); // -10 < 5
}

#[test]
fn test_slt_false() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 10);
    cpu.set_reg(2, 5);

    cpu.op_slt(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 0); // 10 >= 5
}

#[test]
fn test_slt_equal() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 100);

    cpu.op_slt(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 0); // 100 >= 100
}

#[test]
fn test_sltu() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.set_reg(2, 1);

    cpu.op_sltu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 0); // 0xFFFFFFFF > 1 (unsigned)
}

#[test]
fn test_sltu_true() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 5);
    cpu.set_reg(2, 10);

    cpu.op_sltu(1, 2, 3).unwrap();

    assert_eq!(cpu.reg(3), 1); // 5 < 10 (unsigned)
}

#[test]
fn test_slti_true() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 5);

    // SLTI r3, r1, 10 -> 0x2823000A
    let instr = 0x2823000A;
    cpu.op_slti(instr).unwrap();

    assert_eq!(cpu.reg(3), 1); // 5 < 10
}

#[test]
fn test_slti_false() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 15);

    // SLTI r3, r1, 10 -> 0x2823000A
    let instr = 0x2823000A;
    cpu.op_slti(instr).unwrap();

    assert_eq!(cpu.reg(3), 0); // 15 >= 10
}

#[test]
fn test_slti_negative_immediate() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFF6_u32); // -10 as u32

    // SLTI r3, r1, -5 (0xFFFB) -> 0x2823FFFB
    let instr = 0x2823FFFB;
    cpu.op_slti(instr).unwrap();

    assert_eq!(cpu.reg(3), 1); // -10 < -5
}

#[test]
fn test_sltiu_true() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 5);

    // SLTIU r3, r1, 10 -> 0x2C23000A
    let instr = 0x2C23000A;
    cpu.op_sltiu(instr).unwrap();

    assert_eq!(cpu.reg(3), 1); // 5 < 10 (unsigned)
}

#[test]
fn test_sltiu_false() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);

    // SLTIU r3, r1, 10 -> 0x2C23000A
    let instr = 0x2C23000A;
    cpu.op_sltiu(instr).unwrap();

    assert_eq!(cpu.reg(3), 0); // 0xFFFFFFFF > 10 (unsigned)
}

#[test]
fn test_sltiu_sign_extension() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFF0_u32);

    // SLTIU r3, r1, -1 (0xFFFF sign extends to 0xFFFFFFFF)
    let instr = 0x2C23FFFF;
    cpu.op_sltiu(instr).unwrap();

    assert_eq!(cpu.reg(3), 1); // 0xFFFFFFF0 < 0xFFFFFFFF
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

// === Logical Instruction Tests ===

#[test]
fn test_and() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0b11110000);
    cpu.set_reg(2, 0b10101010);
    cpu.op_and(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0b10100000);
}

#[test]
fn test_and_all_bits() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.set_reg(2, 0x12345678);
    cpu.op_and(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x12345678);
}

#[test]
fn test_andi_zero_extension() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);
    // ANDI r3, r1, 0xFFFF -> 0x3023FFFF
    let instr = 0x3023FFFF;
    cpu.op_andi(instr).unwrap();
    assert_eq!(cpu.reg(3), 0x0000FFFF);
}

#[test]
fn test_andi_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x12345678);
    // ANDI r3, r1, 0x00FF -> 0x302300FF
    let instr = 0x302300FF;
    cpu.op_andi(instr).unwrap();
    assert_eq!(cpu.reg(3), 0x00000078);
}

#[test]
fn test_or() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0b11110000);
    cpu.set_reg(2, 0b00001111);
    cpu.op_or(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0b11111111);
}

#[test]
fn test_or_identity() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x12345678);
    cpu.set_reg(2, 0x00000000);
    cpu.op_or(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x12345678);
}

#[test]
fn test_ori_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x12340000);
    // ORI r3, r1, 0x5678 -> 0x34235678
    let instr = 0x34235678;
    cpu.op_ori(instr).unwrap();
    assert_eq!(cpu.reg(3), 0x12345678);
}

#[test]
fn test_ori_zero_extension() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x00000000);
    // ORI r3, r1, 0xFFFF -> 0x3423FFFF
    let instr = 0x3423FFFF;
    cpu.op_ori(instr).unwrap();
    assert_eq!(cpu.reg(3), 0x0000FFFF);
}

#[test]
fn test_xor() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0b11110000);
    cpu.set_reg(2, 0b10101010);
    cpu.op_xor(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0b01011010);
}

#[test]
fn test_xor_same_value() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x12345678);
    cpu.set_reg(2, 0x12345678);
    cpu.op_xor(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x00000000);
}

#[test]
fn test_xori_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFF0000);
    // XORI r3, r1, 0xFFFF -> 0x3823FFFF
    let instr = 0x3823FFFF;
    cpu.op_xori(instr).unwrap();
    assert_eq!(cpu.reg(3), 0xFFFFFFFF);
}

#[test]
fn test_xori_toggle_bits() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x000000FF);
    // XORI r3, r1, 0x00FF -> 0x382300FF
    let instr = 0x382300FF;
    cpu.op_xori(instr).unwrap();
    assert_eq!(cpu.reg(3), 0x00000000);
}

#[test]
fn test_nor() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x00000000);
    cpu.set_reg(2, 0x00000000);
    cpu.op_nor(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0xFFFFFFFF);
}

#[test]
fn test_nor_with_values() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x0F0F0F0F);
    cpu.set_reg(2, 0xF0F0F0F0);
    cpu.op_nor(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x00000000);
}

// === Shift Instruction Tests ===

#[test]
fn test_sll_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x00000001);
    cpu.op_sll(1, 2, 4).unwrap();
    assert_eq!(cpu.reg(2), 0x00000010);
}

#[test]
fn test_sll_overflow() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x80000000);
    cpu.op_sll(1, 2, 1).unwrap();
    assert_eq!(cpu.reg(2), 0x00000000);
}

#[test]
fn test_srl_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xF0000000);
    cpu.op_srl(1, 2, 4).unwrap();
    assert_eq!(cpu.reg(2), 0x0F000000);
}

#[test]
fn test_srl_zero_fill() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xFFFFFFFF);
    cpu.op_srl(1, 2, 1).unwrap();
    assert_eq!(cpu.reg(2), 0x7FFFFFFF);
}

#[test]
fn test_sra_positive() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x70000000);
    cpu.op_sra(1, 2, 4).unwrap();
    assert_eq!(cpu.reg(2), 0x07000000);
}

#[test]
fn test_sra_negative() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0xF0000000); // Negative number
    cpu.op_sra(1, 2, 4).unwrap();
    assert_eq!(cpu.reg(2), 0xFF000000); // Sign-extended
}

#[test]
fn test_sra_negative_shift_one() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x80000000); // Most negative i32
    cpu.op_sra(1, 2, 1).unwrap();
    assert_eq!(cpu.reg(2), 0xC0000000); // Sign-extended
}

#[test]
fn test_sllv_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 4); // Shift amount
    cpu.set_reg(2, 1);
    cpu.op_sllv(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 16);
}

#[test]
fn test_sllv_mask() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x100); // Only lower 5 bits used (0)
    cpu.set_reg(2, 1);
    cpu.op_sllv(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 1); // Shift by 0
}

#[test]
fn test_sllv_max_shift() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 31); // Maximum shift
    cpu.set_reg(2, 1);
    cpu.op_sllv(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x80000000);
}

#[test]
fn test_srlv_basic() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 4); // Shift amount
    cpu.set_reg(2, 0x00000100);
    cpu.op_srlv(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x00000010);
}

#[test]
fn test_srlv_mask() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x120); // Only lower 5 bits used (0)
    cpu.set_reg(2, 0x12345678);
    cpu.op_srlv(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x12345678); // No shift
}

#[test]
fn test_srav_positive() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 4); // Shift amount
    cpu.set_reg(2, 0x70000000);
    cpu.op_srav(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0x07000000);
}

#[test]
fn test_srav_negative() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 4); // Shift amount
    cpu.set_reg(2, 0xF0000000); // Negative
    cpu.op_srav(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0xFF000000); // Sign-extended
}

#[test]
fn test_srav_mask() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 0x104); // Only lower 5 bits used (4)
    cpu.set_reg(2, 0x80000000); // Negative
    cpu.op_srav(1, 2, 3).unwrap();
    assert_eq!(cpu.reg(3), 0xF8000000); // Sign-extended
}

// === Load/Store Instruction Tests ===

#[test]
fn test_lw_basic() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up base address
    cpu.set_reg(1, 0x80000000);

    // Store value in memory
    bus.write32(0x80000000, 0x12345678).unwrap();

    // LW r2, 0(r1) -> load from 0x80000000 into r2
    let instr = 0x8C220000; // opcode=0x23, rs=1, rt=2, offset=0
    cpu.op_lw(instr, &mut bus).unwrap();

    // Value not yet visible due to load delay
    assert_eq!(cpu.reg(2), 0);

    // Execute another load to flush the delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x12345678);
}

#[test]
fn test_lw_with_offset() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up base address
    cpu.set_reg(1, 0x80000000);

    // Store value in memory
    bus.write32(0x80000010, 0xAABBCCDD).unwrap();

    // LW r2, 16(r1) -> load from 0x80000010 into r2
    let instr = 0x8C220010; // opcode=0x23, rs=1, rt=2, offset=16
    cpu.op_lw(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0xAABBCCDD);
}

#[test]
fn test_lw_negative_offset() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up base address
    cpu.set_reg(1, 0x80000020);

    // Store value in memory
    bus.write32(0x80000010, 0xDEADBEEF).unwrap();

    // LW r2, -16(r1) -> load from 0x80000010 into r2
    let instr = 0x8C22FFF0; // opcode=0x23, rs=1, rt=2, offset=-16 (0xFFF0)
    cpu.op_lw(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0xDEADBEEF);
}

#[test]
fn test_lw_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up base address (misaligned)
    cpu.set_reg(1, 0x80000001);

    // LW r2, 0(r1) -> should trigger exception
    let instr = 0x8C220000;
    cpu.op_lw(instr, &mut bus).unwrap();

    // Check exception was raised
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::AddressErrorLoad as u32);
}

#[test]
fn test_lh_sign_extension() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store negative halfword
    bus.write16(0x80000000, 0x8000).unwrap(); // -32768 as i16

    // LH r2, 0(r1)
    let instr = 0x84220000; // opcode=0x21, rs=1, rt=2, offset=0
    cpu.op_lh(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0xFFFF8000); // Sign-extended
}

#[test]
fn test_lh_positive() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store positive halfword
    bus.write16(0x80000000, 0x1234).unwrap();

    // LH r2, 0(r1)
    let instr = 0x84220000;
    cpu.op_lh(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x00001234); // Zero upper bits
}

#[test]
fn test_lh_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000001); // Misaligned

    // LH r2, 0(r1) -> should trigger exception
    let instr = 0x84220000;
    cpu.op_lh(instr, &mut bus).unwrap();

    // Check exception was raised
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::AddressErrorLoad as u32);
}

#[test]
fn test_lhu_zero_extension() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store halfword with high bit set
    bus.write16(0x80000000, 0x8000).unwrap();

    // LHU r2, 0(r1)
    let instr = 0x94220000; // opcode=0x25, rs=1, rt=2, offset=0
    cpu.op_lhu(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x00008000); // Zero-extended, not sign-extended
}

#[test]
fn test_lhu_max_value() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    bus.write16(0x80000000, 0xFFFF).unwrap();

    // LHU r2, 0(r1)
    let instr = 0x94220000;
    cpu.op_lhu(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x0000FFFF); // Zero-extended
}

#[test]
fn test_lb_sign_extension() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store negative byte
    bus.write8(0x80000000, 0x80).unwrap(); // -128 as i8

    // LB r2, 0(r1)
    let instr = 0x80220000; // opcode=0x20, rs=1, rt=2, offset=0
    cpu.op_lb(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0xFFFFFF80); // Sign-extended
}

#[test]
fn test_lb_positive() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store positive byte
    bus.write8(0x80000000, 0x42).unwrap();

    // LB r2, 0(r1)
    let instr = 0x80220000;
    cpu.op_lb(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x00000042);
}

#[test]
fn test_lbu_zero_extension() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store byte with high bit set
    bus.write8(0x80000000, 0xFF).unwrap();

    // LBU r2, 0(r1)
    let instr = 0x90220000; // opcode=0x24, rs=1, rt=2, offset=0
    cpu.op_lbu(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x000000FF); // Zero-extended, not sign-extended
}

#[test]
fn test_lbu_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Byte loads can be unaligned
    cpu.set_reg(1, 0x80000001);
    bus.write8(0x80000001, 0xAB).unwrap();

    // LBU r2, 0(r1)
    let instr = 0x90220000;
    cpu.op_lbu(instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(3, 0);
    assert_eq!(cpu.reg(2), 0x000000AB);
}

#[test]
fn test_sw_basic() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0x12345678);

    // SW r2, 0(r1) -> store to 0x80000000
    let instr = 0xAC220000; // opcode=0x2B, rs=1, rt=2, offset=0
    cpu.op_sw(instr, &mut bus).unwrap();

    // Verify value was written
    assert_eq!(bus.read32(0x80000000).unwrap(), 0x12345678);
}

#[test]
fn test_sw_with_offset() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0xDEADBEEF);

    // SW r2, 16(r1) -> store to 0x80000010
    let instr = 0xAC220010; // opcode=0x2B, rs=1, rt=2, offset=16
    cpu.op_sw(instr, &mut bus).unwrap();

    // Verify value was written
    assert_eq!(bus.read32(0x80000010).unwrap(), 0xDEADBEEF);
}

#[test]
fn test_sw_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000001); // Misaligned
    cpu.set_reg(2, 0x12345678);

    // SW r2, 0(r1) -> should trigger exception
    let instr = 0xAC220000;
    cpu.op_sw(instr, &mut bus).unwrap();

    // Check exception was raised
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::AddressErrorStore as u32);
}

#[test]
fn test_sh_basic() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0x12345678);

    // SH r2, 0(r1) -> store lower 16 bits to 0x80000000
    let instr = 0xA4220000; // opcode=0x29, rs=1, rt=2, offset=0
    cpu.op_sh(instr, &mut bus).unwrap();

    // Verify value was written (only lower 16 bits)
    assert_eq!(bus.read16(0x80000000).unwrap(), 0x5678);
}

#[test]
fn test_sh_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000001); // Misaligned
    cpu.set_reg(2, 0x1234);

    // SH r2, 0(r1) -> should trigger exception
    let instr = 0xA4220000;
    cpu.op_sh(instr, &mut bus).unwrap();

    // Check exception was raised
    let cause = cpu.cop0.regs[COP0::CAUSE];
    let exception_code = (cause >> 2) & 0x1F;
    assert_eq!(exception_code, ExceptionCause::AddressErrorStore as u32);
}

#[test]
fn test_sb_basic() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0x12345678);

    // SB r2, 0(r1) -> store lower 8 bits to 0x80000000
    let instr = 0xA0220000; // opcode=0x28, rs=1, rt=2, offset=0
    cpu.op_sb(instr, &mut bus).unwrap();

    // Verify value was written (only lower 8 bits)
    assert_eq!(bus.read8(0x80000000).unwrap(), 0x78);
}

#[test]
fn test_sb_unaligned() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Byte stores can be unaligned
    cpu.set_reg(1, 0x80000001);
    cpu.set_reg(2, 0xAB);

    // SB r2, 0(r1)
    let instr = 0xA0220000;
    cpu.op_sb(instr, &mut bus).unwrap();

    // Verify value was written
    assert_eq!(bus.read8(0x80000001).unwrap(), 0xAB);
}

#[test]
fn test_load_delay_slot_interaction() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up memory
    bus.write32(0x80000000, 0x11111111).unwrap();
    bus.write32(0x80000004, 0x22222222).unwrap();

    cpu.set_reg(1, 0x80000000);
    cpu.set_reg(2, 0x80000004);

    // LW r3, 0(r1) - Load first value
    let instr1 = 0x8C230000;
    cpu.op_lw(instr1, &mut bus).unwrap();

    // r3 not yet available
    assert_eq!(cpu.reg(3), 0);

    // LW r4, 0(r2) - Load second value, flushes first delay
    let instr2 = 0x8C440000;
    cpu.op_lw(instr2, &mut bus).unwrap();

    // Now r3 has first value, r4 still waiting
    assert_eq!(cpu.reg(3), 0x11111111);
    assert_eq!(cpu.reg(4), 0);

    // Another instruction flushes second delay
    cpu.set_reg_delayed(5, 0);
    assert_eq!(cpu.reg(4), 0x22222222);
}

#[test]
fn test_load_store_round_trip() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store sequence
    cpu.set_reg(2, 0x12345678);
    let sw_instr = 0xAC220000;
    cpu.op_sw(sw_instr, &mut bus).unwrap();

    // Load back
    let lw_instr = 0x8C230000;
    cpu.op_lw(lw_instr, &mut bus).unwrap();

    // Flush delay
    cpu.set_reg_delayed(4, 0);

    // Verify round trip
    assert_eq!(cpu.reg(3), 0x12345678);
}

#[test]
fn test_mixed_size_load_store() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    cpu.set_reg(1, 0x80000000);

    // Store a word
    cpu.set_reg(2, 0x12345678);
    let sw_instr = 0xAC220000;
    cpu.op_sw(sw_instr, &mut bus).unwrap();

    // Load individual bytes
    let lb_instr0 = 0x80230000; // LB r3, 0(r1)
    cpu.op_lb(lb_instr0, &mut bus).unwrap();
    cpu.set_reg_delayed(0, 0); // Flush
    assert_eq!(cpu.reg(3), 0x00000078); // Little-endian, byte 0

    let lb_instr1 = 0x80230001; // LB r3, 1(r1)
    cpu.op_lb(lb_instr1, &mut bus).unwrap();
    cpu.set_reg_delayed(0, 0); // Flush
    assert_eq!(cpu.reg(3), 0x00000056); // Byte 1

    // Load halfword
    let lh_instr = 0x84230000; // LH r3, 0(r1)
    cpu.op_lh(lh_instr, &mut bus).unwrap();
    cpu.set_reg_delayed(0, 0); // Flush
    assert_eq!(cpu.reg(3), 0x00005678); // Lower halfword
}

// === Branch and Jump Instruction Tests ===

#[test]
fn test_jr_instruction() {
    let mut cpu = CPU::new();
    cpu.set_reg(31, 0x80001234);

    // JR r31
    cpu.op_jr(31).unwrap();

    assert_eq!(cpu.next_pc, 0x80001234);
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_jalr_instruction() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 0x80005678);

    // JALR r1, r31
    cpu.op_jalr(1, 31).unwrap();

    assert_eq!(cpu.next_pc, 0x80005678);
    assert_eq!(cpu.reg(31), 0x80000004); // Return address
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_beq_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 100);

    // BEQ r1, r2, 8 (branch offset = 8)
    let beq_instr = 0x10220002; // offset = 2 words
    cpu.op_beq(beq_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_beq_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 200);

    // BEQ r1, r2, 8
    let beq_instr = 0x10220002;
    cpu.op_beq(beq_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bne_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 200);

    // BNE r1, r2, 8
    let bne_instr = 0x14220002;
    cpu.op_bne(bne_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bne_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 100);
    cpu.set_reg(2, 100);

    // BNE r1, r2, 8
    let bne_instr = 0x14220002;
    cpu.op_bne(bne_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_blez_taken_zero() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 0);

    // BLEZ r1, 8
    let blez_instr = 0x18200002;
    cpu.op_blez(blez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_blez_taken_negative() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BLEZ r1, 8
    let blez_instr = 0x18200002;
    cpu.op_blez(blez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_blez_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BLEZ r1, 8
    let blez_instr = 0x18200002;
    cpu.op_blez(blez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bgtz_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BGTZ r1, 8
    let bgtz_instr = 0x1C200002;
    cpu.op_bgtz(bgtz_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bgtz_not_taken_zero() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 0);

    // BGTZ r1, 8
    let bgtz_instr = 0x1C200002;
    cpu.op_bgtz(bgtz_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bgtz_not_taken_negative() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BGTZ r1, 8
    let bgtz_instr = 0x1C200002;
    cpu.op_bgtz(bgtz_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bltz_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BLTZ r1, 8 (rt=0x00 for BLTZ)
    let bltz_instr = 0x04200002;
    cpu.execute_bcondz(bltz_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bltz_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BLTZ r1, 8
    let bltz_instr = 0x04200002;
    cpu.execute_bcondz(bltz_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bgez_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BGEZ r1, 8 (rt=0x01 for BGEZ)
    let bgez_instr = 0x04210002;
    cpu.execute_bcondz(bgez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bgez_taken_zero() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 0);

    // BGEZ r1, 8
    let bgez_instr = 0x04210002;
    cpu.execute_bcondz(bgez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken (zero is >= 0)
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bgez_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BGEZ r1, 8
    let bgez_instr = 0x04210002;
    cpu.execute_bcondz(bgez_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bltzal_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BLTZAL r1, 8 (rt=0x10 for BLTZAL)
    let bltzal_instr = 0x04300002;
    cpu.execute_bcondz(bltzal_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert_eq!(cpu.reg(31), 0x80000004); // Return address saved
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bltzal_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BLTZAL r1, 8
    let bltzal_instr = 0x04300002;
    cpu.execute_bcondz(bltzal_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert_eq!(cpu.reg(31), 0x80000004); // Return address still saved
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_bgezal_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, 10);

    // BGEZAL r1, 8 (rt=0x11 for BGEZAL)
    let bgezal_instr = 0x04310002;
    cpu.execute_bcondz(bgezal_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004 + 8); // Branch taken
    assert_eq!(cpu.reg(31), 0x80000004); // Return address saved
    assert!(cpu.in_delay_slot());
}

#[test]
fn test_bgezal_not_taken() {
    let mut cpu = CPU::new();
    cpu.pc = 0x80000000;
    cpu.next_pc = 0x80000004;
    cpu.set_reg(1, (-10i32) as u32);

    // BGEZAL r1, 8
    let bgezal_instr = 0x04310002;
    cpu.execute_bcondz(bgezal_instr).unwrap();

    assert_eq!(cpu.next_pc, 0x80000004); // Branch not taken
    assert_eq!(cpu.reg(31), 0x80000004); // Return address still saved
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_branch_delay_slot_cleared_after_step() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Set up a simple program that doesn't branch
    cpu.pc = 0xBFC00000;
    cpu.next_pc = 0xBFC00004;
    cpu.in_branch_delay = true;

    // Write a NOP instruction (SLL r0, r0, 0)
    bus.write32(0xBFC00000, 0x00000000).unwrap();

    // Execute step
    cpu.step(&mut bus).unwrap();

    // Branch delay flag should be cleared
    assert!(!cpu.in_delay_slot());
}

#[test]
fn test_jump_preserves_upper_pc_bits() {
    let mut cpu = CPU::new();
    cpu.pc = 0xBFC00000;
    cpu.next_pc = 0xBFC00004;

    // J 0x00100000 (should preserve upper 4 bits of PC)
    let j_instr = 0x08100000;
    cpu.op_j(j_instr).unwrap();

    // Upper 4 bits should be 0xB (from 0xBFC00000)
    assert_eq!(cpu.next_pc & 0xF0000000, 0xB0000000);
    assert_eq!(cpu.next_pc, 0xB0400000);
}
