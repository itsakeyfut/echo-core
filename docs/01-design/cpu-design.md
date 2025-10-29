# CPU Implementation Design

## Overview

The PlayStation CPU is a **MIPS R3000A** compatible processor with a 32-bit RISC architecture. This document describes the detailed design of the CPU implementation in the emulator.

## Hardware Specifications

### Basic Specifications
- **Architecture**: MIPS I (32-bit)
- **Clock Frequency**: 33.8688 MHz
- **Pipeline**: 5-stage (actual hardware) → Simplified in emulator
- **Instruction Set**: Approximately 60 MIPS instructions
- **Registers**: 32 general-purpose registers + special registers

### Actual Hardware Behavior
- **Instruction Execution**: Average 1 cycle/instruction (ideal)
- **Memory Access**: With cache (8KB instruction, 4KB data)
- **Delay Slot**: 1 instruction after branch is always executed
- **Exception Handling**: Hardware exceptions and software interrupts

## Implementation Strategy

### Phased Implementation Approach

```
Phase 1: Interpreter (Week 1-4)
  ↓
Phase 2: Cached Interpreter (Week 8-10)
  ↓
Phase 3: Recompiler/JIT (Future)
```

We'll progressively optimize while maintaining a working state at each phase.

## Phase 1: Interpreter Implementation

### 1.1 Data Structures

```rust
/// Structure representing CPU state
pub struct CPU {
    /// General-purpose registers (r0-r31)
    /// r0 is always 0 (hardwired)
    regs: [u32; 32],

    /// Program counter
    pc: u32,

    /// Next PC (for delay slot handling)
    next_pc: u32,

    /// Upper 32 bits of multiplication/division results
    hi: u32,

    /// Lower 32 bits of multiplication/division results
    lo: u32,

    /// Coprocessor 0 (System Control Unit)
    cop0: COP0,

    /// Load delay slot
    /// On PSX, load results cannot be used in the next instruction
    load_delay: Option<LoadDelay>,

    /// Whether in branch delay slot
    in_branch_delay: bool,

    /// Current instruction (for debugging)
    current_instruction: u32,
}

/// Structure managing load delay
#[derive(Debug, Clone, Copy)]
pub struct LoadDelay {
    /// Target register
    reg: u8,
    /// Value to load
    value: u32,
}

/// Coprocessor 0 (System Control)
pub struct COP0 {
    /// Register array (32 registers)
    regs: [u32; 32],
}

// COP0 register indices
impl COP0 {
    pub const BPC: usize = 3;      // Breakpoint PC
    pub const BDA: usize = 5;      // Breakpoint Data Address
    pub const TAR: usize = 6;      // Target Address
    pub const DCIC: usize = 7;     // Cache control
    pub const BADA: usize = 8;     // Bad Virtual Address
    pub const BDAM: usize = 9;     // Data Address Mask
    pub const BPCM: usize = 11;    // PC Mask
    pub const SR: usize = 12;      // Status Register
    pub const CAUSE: usize = 13;   // Cause Register
    pub const EPC: usize = 14;     // Exception PC
    pub const PRID: usize = 15;    // Processor ID
}
```

### 1.2 Register Access

```rust
impl CPU {
    /// Read register (r0 always returns 0)
    #[inline(always)]
    pub fn reg(&self, index: u8) -> u32 {
        if index == 0 {
            0
        } else {
            self.regs[index as usize]
        }
    }

    /// Write register (writes to r0 are ignored)
    #[inline(always)]
    pub fn set_reg(&mut self, index: u8, value: u32) {
        if index != 0 {
            self.regs[index as usize] = value;
        }
    }

    /// Register write with load delay consideration
    pub fn set_reg_delayed(&mut self, index: u8, value: u32) {
        // Execute current load delay
        if let Some(delay) = self.load_delay.take() {
            self.set_reg(delay.reg, delay.value);
        }

        // Set new load delay
        if index != 0 {
            self.load_delay = Some(LoadDelay {
                reg: index,
                value,
            });
        }
    }
}
```

### 1.3 Instruction Execution Loop

```rust
impl CPU {
    /// Execute one instruction (returns cycle count)
    pub fn step(&mut self, bus: &mut Bus) -> Result<u32> {
        // Resolve load delay
        if let Some(delay) = self.load_delay.take() {
            self.set_reg(delay.reg, delay.value);
        }

        // Instruction fetch
        let pc = self.pc;
        self.current_instruction = bus.read32(pc)?;

        // Delay slot processing
        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        // Instruction execution
        self.execute_instruction(bus)?;

        // Basically 1 cycle (actual hardware is more complex)
        Ok(1)
    }

    /// Instruction decode and execution
    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<()> {
        let instruction = self.current_instruction;

        // Instruction decode
        let opcode = instruction >> 26;

        match opcode {
            0x00 => self.execute_special(instruction, bus),
            0x01 => self.execute_bcondz(instruction),
            0x02 => self.op_j(instruction),      // J
            0x03 => self.op_jal(instruction),    // JAL
            0x04 => self.op_beq(instruction),    // BEQ
            0x05 => self.op_bne(instruction),    // BNE
            0x06 => self.op_blez(instruction),   // BLEZ
            0x07 => self.op_bgtz(instruction),   // BGTZ
            0x08 => self.op_addi(instruction),   // ADDI
            0x09 => self.op_addiu(instruction),  // ADDIU
            0x0A => self.op_slti(instruction),   // SLTI
            0x0B => self.op_sltiu(instruction),  // SLTIU
            0x0C => self.op_andi(instruction),   // ANDI
            0x0D => self.op_ori(instruction),    // ORI
            0x0E => self.op_xori(instruction),   // XORI
            0x0F => self.op_lui(instruction),    // LUI
            0x10 => self.execute_cop0(instruction), // COP0
            0x11 => self.execute_cop1(instruction), // COP1 (not implemented)
            0x12 => self.execute_cop2(instruction), // COP2 (GTE)
            0x13 => self.execute_cop3(instruction), // COP3 (not implemented)
            0x20 => self.op_lb(instruction, bus),   // LB
            0x21 => self.op_lh(instruction, bus),   // LH
            0x22 => self.op_lwl(instruction, bus),  // LWL
            0x23 => self.op_lw(instruction, bus),   // LW
            0x24 => self.op_lbu(instruction, bus),  // LBU
            0x25 => self.op_lhu(instruction, bus),  // LHU
            0x26 => self.op_lwr(instruction, bus),  // LWR
            0x28 => self.op_sb(instruction, bus),   // SB
            0x29 => self.op_sh(instruction, bus),   // SH
            0x2A => self.op_swl(instruction, bus),  // SWL
            0x2B => self.op_sw(instruction, bus),   // SW
            0x2E => self.op_swr(instruction, bus),  // SWR
            _ => {
                log::warn!("Unimplemented opcode: 0x{:02X} at PC=0x{:08X}",
                          opcode, self.pc);
                self.exception(ExceptionCause::ReservedInstruction);
                Ok(())
            }
        }
    }
}
```

### 1.4 Instruction Formats

MIPS instructions are classified into three basic formats:

```rust
/// R-type instruction: Register operations
/// |31-26|25-21|20-16|15-11|10-6|5-0  |
/// |  op | rs  | rt  | rd  | sa | fct |
#[inline(always)]
fn decode_r_type(instr: u32) -> (u8, u8, u8, u8, u8) {
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let rd = ((instr >> 11) & 0x1F) as u8;
    let shamt = ((instr >> 6) & 0x1F) as u8;
    let funct = (instr & 0x3F) as u8;
    (rs, rt, rd, shamt, funct)
}

/// I-type instruction: Immediate operations
/// |31-26|25-21|20-16|15-0     |
/// |  op | rs  | rt  |   imm   |
#[inline(always)]
fn decode_i_type(instr: u32) -> (u8, u8, u8, u16) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let imm = (instr & 0xFFFF) as u16;
    (op, rs, rt, imm)
}

/// J-type instruction: Jump
/// |31-26|25-0              |
/// |  op |      target      |
#[inline(always)]
fn decode_j_type(instr: u32) -> (u8, u32) {
    let op = ((instr >> 26) & 0x3F) as u8;
    let target = instr & 0x03FFFFFF;
    (op, target)
}
```

### 1.5 Implementation Examples of Key Instructions

#### Arithmetic Instructions

```rust
impl CPU {
    /// ADD: Addition (with overflow exception)
    fn op_add(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs) as i32;
        let b = self.reg(rt) as i32;

        match a.checked_add(b) {
            Some(result) => {
                self.set_reg(rd, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }

    /// ADDU: Addition (without overflow exception)
    fn op_addu(&mut self, rs: u8, rt: u8, rd: u8) {
        let result = self.reg(rs).wrapping_add(self.reg(rt));
        self.set_reg(rd, result);
    }

    /// ADDI: Immediate addition (with overflow exception)
    fn op_addi(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as i32; // Sign extension
        let a = self.reg(rs) as i32;

        match a.checked_add(imm) {
            Some(result) => {
                self.set_reg(rt, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }

    /// ADDIU: Immediate addition (without overflow exception)
    fn op_addiu(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let imm = (imm as i16) as u32; // Sign extension
        let result = self.reg(rs).wrapping_add(imm);
        self.set_reg(rt, result);
        Ok(())
    }

    /// SUB: Subtraction
    fn op_sub(&mut self, rs: u8, rt: u8, rd: u8) -> Result<()> {
        let a = self.reg(rs) as i32;
        let b = self.reg(rt) as i32;

        match a.checked_sub(b) {
            Some(result) => {
                self.set_reg(rd, result as u32);
                Ok(())
            }
            None => {
                self.exception(ExceptionCause::Overflow);
                Ok(())
            }
        }
    }
}
```

#### Logical Instructions

```rust
impl CPU {
    /// AND: Logical AND
    fn op_and(&mut self, rs: u8, rt: u8, rd: u8) {
        let result = self.reg(rs) & self.reg(rt);
        self.set_reg(rd, result);
    }

    /// ANDI: Immediate AND (zero extension)
    fn op_andi(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let result = self.reg(rs) & (imm as u32);
        self.set_reg(rt, result);
        Ok(())
    }

    /// OR: Logical OR
    fn op_or(&mut self, rs: u8, rt: u8, rd: u8) {
        let result = self.reg(rs) | self.reg(rt);
        self.set_reg(rd, result);
    }

    /// ORI: Immediate OR
    fn op_ori(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let result = self.reg(rs) | (imm as u32);
        self.set_reg(rt, result);
        Ok(())
    }

    /// XOR: Exclusive OR
    fn op_xor(&mut self, rs: u8, rt: u8, rd: u8) {
        let result = self.reg(rs) ^ self.reg(rt);
        self.set_reg(rd, result);
    }

    /// NOR: Logical NOR
    fn op_nor(&mut self, rs: u8, rt: u8, rd: u8) {
        let result = !(self.reg(rs) | self.reg(rt));
        self.set_reg(rd, result);
    }
}
```

#### Shift Instructions

```rust
impl CPU {
    /// SLL: Logical left shift
    fn op_sll(&mut self, rt: u8, rd: u8, shamt: u8) {
        let result = self.reg(rt) << shamt;
        self.set_reg(rd, result);
    }

    /// SRL: Logical right shift
    fn op_srl(&mut self, rt: u8, rd: u8, shamt: u8) {
        let result = self.reg(rt) >> shamt;
        self.set_reg(rd, result);
    }

    /// SRA: Arithmetic right shift (sign preserved)
    fn op_sra(&mut self, rt: u8, rd: u8, shamt: u8) {
        let result = ((self.reg(rt) as i32) >> shamt) as u32;
        self.set_reg(rd, result);
    }

    /// SLLV: Variable logical left shift
    fn op_sllv(&mut self, rs: u8, rt: u8, rd: u8) {
        let shamt = self.reg(rs) & 0x1F; // Use only lower 5 bits
        let result = self.reg(rt) << shamt;
        self.set_reg(rd, result);
    }
}
```

#### Branch Instructions

```rust
impl CPU {
    /// BEQ: Branch if equal
    fn op_beq(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2; // Sign extend and multiply by 4

        if self.reg(rs) == self.reg(rt) {
            self.branch(offset);
        }
        Ok(())
    }

    /// BNE: Branch if not equal
    fn op_bne(&mut self, instruction: u32) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = ((imm as i16) as i32) << 2;

        if self.reg(rs) != self.reg(rt) {
            self.branch(offset);
        }
        Ok(())
    }

    /// Branch processing (with delay slot consideration)
    fn branch(&mut self, offset: i32) {
        // next_pc is already +4, so add offset from there
        self.next_pc = self.next_pc.wrapping_add(offset as u32);
        self.in_branch_delay = true;
    }
}
```

#### Jump Instructions

```rust
impl CPU {
    /// J: Jump
    fn op_j(&mut self, instruction: u32) -> Result<()> {
        let (_, target) = decode_j_type(instruction);
        // Upper 4 bits of PC + target << 2
        let pc_high = self.pc & 0xF0000000;
        self.next_pc = pc_high | (target << 2);
        Ok(())
    }

    /// JAL: Jump and link (function call)
    fn op_jal(&mut self, instruction: u32) -> Result<()> {
        let (_, target) = decode_j_type(instruction);
        // Save return address to r31
        self.set_reg(31, self.next_pc);

        let pc_high = self.pc & 0xF0000000;
        self.next_pc = pc_high | (target << 2);
        Ok(())
    }

    /// JR: Register jump (function return)
    fn op_jr(&mut self, rs: u8) {
        self.next_pc = self.reg(rs);
    }

    /// JALR: Register jump and link
    fn op_jalr(&mut self, rs: u8, rd: u8) {
        self.set_reg(rd, self.next_pc);
        self.next_pc = self.reg(rs);
    }
}
```

#### Load/Store Instructions

```rust
impl CPU {
    /// LW: Load word (32 bits)
    fn op_lw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32; // Sign extension
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Alignment check
        if addr & 0x3 != 0 {
            self.exception(ExceptionCause::AddressErrorLoad);
            return Ok(());
        }

        let value = bus.read32(addr)?;
        self.set_reg_delayed(rt, value); // Load delay
        Ok(())
    }

    /// LB: Load byte (sign extension)
    fn op_lb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32;
        let addr = self.reg(rs).wrapping_add(offset as u32);

        let value = bus.read8(addr)? as i8 as i32 as u32; // Sign extension
        self.set_reg_delayed(rt, value);
        Ok(())
    }

    /// LBU: Load byte (zero extension)
    fn op_lbu(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32;
        let addr = self.reg(rs).wrapping_add(offset as u32);

        let value = bus.read8(addr)? as u32;
        self.set_reg_delayed(rt, value);
        Ok(())
    }

    /// SW: Store word (32 bits)
    fn op_sw(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32;
        let addr = self.reg(rs).wrapping_add(offset as u32);

        // Alignment check
        if addr & 0x3 != 0 {
            self.exception(ExceptionCause::AddressErrorStore);
            return Ok(());
        }

        bus.write32(addr, self.reg(rt))?;
        Ok(())
    }

    /// SB: Store byte
    fn op_sb(&mut self, instruction: u32, bus: &mut Bus) -> Result<()> {
        let (_, rs, rt, imm) = decode_i_type(instruction);
        let offset = (imm as i16) as i32;
        let addr = self.reg(rs).wrapping_add(offset as u32);

        bus.write8(addr, self.reg(rt) as u8)?;
        Ok(())
    }
}
```

#### Multiplication/Division Instructions

```rust
impl CPU {
    /// MULT: Signed multiplication
    fn op_mult(&mut self, rs: u8, rt: u8) {
        let a = self.reg(rs) as i32 as i64;
        let b = self.reg(rt) as i32 as i64;
        let result = a * b;

        self.lo = result as u32;
        self.hi = (result >> 32) as u32;
    }

    /// MULTU: Unsigned multiplication
    fn op_multu(&mut self, rs: u8, rt: u8) {
        let a = self.reg(rs) as u64;
        let b = self.reg(rt) as u64;
        let result = a * b;

        self.lo = result as u32;
        self.hi = (result >> 32) as u32;
    }

    /// DIV: Signed division
    fn op_div(&mut self, rs: u8, rt: u8) {
        let numerator = self.reg(rs) as i32;
        let denominator = self.reg(rt) as i32;

        if denominator == 0 {
            // Division by zero (not an exception on PSX)
            self.lo = if numerator >= 0 { 0xFFFFFFFF } else { 1 };
            self.hi = numerator as u32;
        } else if numerator as u32 == 0x80000000 && denominator == -1 {
            // Overflow
            self.lo = 0x80000000;
            self.hi = 0;
        } else {
            self.lo = (numerator / denominator) as u32;
            self.hi = (numerator % denominator) as u32;
        }
    }

    /// DIVU: Unsigned division
    fn op_divu(&mut self, rs: u8, rt: u8) {
        let numerator = self.reg(rs);
        let denominator = self.reg(rt);

        if denominator == 0 {
            self.lo = 0xFFFFFFFF;
            self.hi = numerator;
        } else {
            self.lo = numerator / denominator;
            self.hi = numerator % denominator;
        }
    }

    /// MFHI: Move from HI
    fn op_mfhi(&mut self, rd: u8) {
        self.set_reg(rd, self.hi);
    }

    /// MFLO: Move from LO
    fn op_mflo(&mut self, rd: u8) {
        self.set_reg(rd, self.lo);
    }

    /// MTHI: Move to HI
    fn op_mthi(&mut self, rs: u8) {
        self.hi = self.reg(rs);
    }

    /// MTLO: Move to LO
    fn op_mtlo(&mut self, rs: u8) {
        self.lo = self.reg(rs);
    }
}
```

### 1.6 Coprocessor 0 (System Control)

```rust
impl CPU {
    /// Execute COP0 instruction
    fn execute_cop0(&mut self, instruction: u32) -> Result<()> {
        let rs = ((instruction >> 21) & 0x1F) as u8;

        match rs {
            0x00 => self.op_mfc0(instruction),  // MFC0: Load from COP0
            0x04 => self.op_mtc0(instruction),  // MTC0: Store to COP0
            0x10 => self.op_rfe(instruction),   // RFE: Return from exception
            _ => {
                log::warn!("Unimplemented COP0 function: 0x{:02X}", rs);
                Ok(())
            }
        }
    }

    /// MFC0: Load COP0 register to general register
    fn op_mfc0(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.cop0.regs[rd as usize];
        self.set_reg_delayed(rt, value);
        Ok(())
    }

    /// MTC0: Store general register to COP0 register
    fn op_mtc0(&mut self, instruction: u32) -> Result<()> {
        let rt = ((instruction >> 16) & 0x1F) as u8;
        let rd = ((instruction >> 11) & 0x1F) as u8;

        let value = self.reg(rt);
        self.cop0.regs[rd as usize] = value;
        Ok(())
    }

    /// RFE: Return from exception (restore status register)
    fn op_rfe(&mut self, _instruction: u32) -> Result<()> {
        let sr = self.cop0.regs[COP0::SR];
        // Shift status register bits
        let mode = sr & 0x3F;
        self.cop0.regs[COP0::SR] = (sr & !0x3F) | (mode >> 2);
        Ok(())
    }
}
```

### 1.7 Exception Handling

```rust
/// Exception types
#[derive(Debug, Clone, Copy)]
pub enum ExceptionCause {
    Interrupt = 0,
    AddressErrorLoad = 4,
    AddressErrorStore = 5,
    BusErrorInstruction = 6,
    BusErrorData = 7,
    Syscall = 8,
    Breakpoint = 9,
    ReservedInstruction = 10,
    CoprocessorUnusable = 11,
    Overflow = 12,
}

impl CPU {
    /// Raise exception
    pub fn exception(&mut self, cause: ExceptionCause) {
        // Save current status
        let sr = self.cop0.regs[COP0::SR];
        let mode = sr & 0x3F;
        self.cop0.regs[COP0::SR] = (sr & !0x3F) | ((mode << 2) & 0x3F);

        // Set exception code in Cause register
        let cause_reg = self.cop0.regs[COP0::CAUSE];
        self.cop0.regs[COP0::CAUSE] = (cause_reg & !0x7C) | ((cause as u32) << 2);

        // Save PC at exception time
        self.cop0.regs[COP0::EPC] = if self.in_branch_delay {
            self.pc.wrapping_sub(4)
        } else {
            self.pc
        };

        // Record whether in delay slot in CAUSE
        if self.in_branch_delay {
            self.cop0.regs[COP0::CAUSE] |= 1 << 31;
        }

        // Jump to exception handler
        let handler = if (sr & (1 << 22)) != 0 {
            0xBFC00180 // BEV=1: Bootstrap
        } else {
            0x80000080 // BEV=0: Normal
        };

        self.pc = handler;
        self.next_pc = handler.wrapping_add(4);
    }
}
```

## Phase 2: Cached Interpreter

Once Phase 1 is working, we'll implement a cached interpreter for performance improvement.

### 2.1 Basic Concept

```rust
/// Decoded instruction
enum DecodedInstruction {
    Add { rs: u8, rt: u8, rd: u8 },
    Addiu { rs: u8, rt: u8, imm: u32 },
    Lw { rs: u8, rt: u8, offset: i16 },
    Sw { rs: u8, rt: u8, offset: i16 },
    Beq { rs: u8, rt: u8, offset: i32 },
    // ... other instructions
}

/// Instruction cache
struct InstructionCache {
    /// Instruction cache keyed by PC
    cache: HashMap<u32, DecodedInstruction>,
}

impl CPU {
    /// Execute via cache
    fn step_cached(&mut self, bus: &mut Bus, cache: &mut InstructionCache) -> Result<u32> {
        let pc = self.pc;

        // Cache hit?
        if let Some(decoded) = cache.cache.get(&pc) {
            self.execute_decoded(decoded, bus)?;
        } else {
            // Cache miss: Decode and add to cache
            let instruction = bus.read32(pc)?;
            let decoded = self.decode_instruction(instruction);
            cache.cache.insert(pc, decoded.clone());
            self.execute_decoded(&decoded, bus)?;
        }

        Ok(1)
    }
}
```

## Phase 3: Recompiler (Future)

A recompiler (JIT compiler) translates MIPS code to x86/ARM instructions for execution.

### 3.1 Basic Strategy

```rust
/// Basic block: A sequence of instructions up to a branch
struct BasicBlock {
    start_pc: u32,
    end_pc: u32,
    native_code: Vec<u8>, // x86/ARM instructions
}

/// Recompiler
struct Recompiler {
    blocks: HashMap<u32, BasicBlock>,
    code_buffer: ExecutableBuffer,
}
```

**Phase 3 will not be implemented for now** (Phase 1 and 2 provide sufficient performance)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addu() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 20);

        // ADDU r3, r1, r2
        cpu.op_addu(1, 2, 3);

        assert_eq!(cpu.reg(3), 30);
    }

    #[test]
    fn test_overflow_detection() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x7FFFFFFF);
        cpu.set_reg(2, 1);

        // ADD r3, r1, r2 (should cause overflow)
        let result = cpu.op_add(1, 2, 3);

        // Verify exception occurred
        assert!(result.is_err() || cpu.cop0.regs[COP0::CAUSE] != 0);
    }
}
```

### CPU Instruction Test ROMs

We'll validate using existing test ROMs (such as amidog's CPU tests).

## Performance Optimization

### Hot Path Optimization

```rust
// Frequently called functions should be inlined
#[inline(always)]
pub fn reg(&self, index: u8) -> u32 { ... }

#[inline(always)]
pub fn set_reg(&mut self, index: u8, value: u32) { ... }

// Branch prediction hints (Rust doesn't have #[likely], but match order can optimize)
match opcode {
    0x00 => { /* SPECIAL: most frequent */ }
    0x23 => { /* LW: frequent */ }
    0x2B => { /* SW: frequent */ }
    _ => { /* others */ }
}
```

## Debug Features

### Debugger Interface

```rust
pub struct CPUDebugger {
    breakpoints: HashSet<u32>,
    watchpoints: HashMap<u32, WatchType>,
    step_mode: bool,
}

pub enum WatchType {
    Read,
    Write,
    ReadWrite,
}

impl CPUDebugger {
    pub fn add_breakpoint(&mut self, pc: u32) {
        self.breakpoints.insert(pc);
    }

    pub fn check_breakpoint(&self, pc: u32) -> bool {
        self.breakpoints.contains(&pc)
    }

    pub fn disassemble(&self, instruction: u32) -> String {
        // Convert MIPS instruction to human-readable format
        // Example: "ADDIU r2, r1, 0x10"
        todo!()
    }
}
```

## Summary

### Implementation Checklist

**Phase 1 (Required):**
- [ ] Basic data structures (CPU, COP0)
- [ ] Register access
- [ ] Instruction decoding
- [ ] Arithmetic instructions (20 types)
- [ ] Logical instructions (10 types)
- [ ] Shift instructions (6 types)
- [ ] Branch/jump instructions (12 types)
- [ ] Load/store instructions (14 types)
- [ ] Multiplication/division instructions (8 types)
- [ ] COP0 instructions (3 types)
- [ ] Exception handling
- [ ] Load delay slot
- [ ] Branch delay slot

**Phase 2 (Recommended):**
- [ ] Instruction cache
- [ ] Basic block detection

**Phase 3 (Future):**
- [ ] Recompiler

### References

- [PSX-SPX CPU Section](http://problemkaputt.de/psx-spx.htm#cpuspecifications)
- [MIPS Instruction Set Reference](https://www.mips.com/)
- [amidog's PSX CPU Tests](https://github.com/amidog/mips_tests)

## Related Documents

- [System Architecture](../00-overview/architecture.md)
- [Memory Design](./memory-design.md)
- [MIPS Instruction Set Reference](../04-reference/mips-instruction-set.md)
