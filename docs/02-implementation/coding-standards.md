# Coding Standards

## Overview

This document defines unified coding standards for PSX emulator development. The goal is to maintain a consistent codebase and enhance maintainability.

## Basic Principles

1. **Readability First**: In performance vs. readability tradeoffs, prioritize readability first
2. **Explicit > Implicit**: Express intent clearly
3. **Safety First**: Minimize use of `unsafe`
4. **DRY (Don't Repeat Yourself)**: Avoid duplication
5. **YAGNI (You Aren't Gonna Need It)**: Don't implement until needed

---

## Naming Conventions

### General Naming Rules

| Type | Convention | Example |
|------|-----------|---------|
| Types, Structs, Enums | PascalCase | `CPU`, `MemoryBus`, `ExceptionCause` |
| Functions, Methods | snake_case | `read_memory`, `execute_instruction` |
| Variables, Fields | snake_case | `program_counter`, `cycle_count` |
| Constants | UPPER_SNAKE_CASE | `CYCLES_PER_FRAME`, `RAM_SIZE` |
| Modules | snake_case | `cpu`, `memory`, `gpu` |
| Traits | PascalCase | `MemoryAccess`, `Debuggable` |
| Lifetimes | single lowercase letter | `'a`, `'b` |
| Generic Types | single uppercase letter | `T`, `E`, `R` |

### Concrete Examples

```rust
// ✅ Good example
pub struct CPU {
    pub regs: [u32; 32],
    program_counter: u32,
}

const MAX_CYCLE_COUNT: u64 = 1_000_000;

impl CPU {
    pub fn new() -> Self {
        Self {
            regs: [0; 32],
            program_counter: 0xBFC00000,
        }
    }

    pub fn read_register(&self, index: u8) -> u32 {
        self.regs[index as usize]
    }
}

// ❌ Bad example
pub struct cpu {  // Type names should be PascalCase
    pub Regs: [u32; 32],  // Fields should be snake_case
    PC: u32,  // Fields should be snake_case
}

const maxCycleCount: u64 = 1_000_000;  // Constants should be UPPER_SNAKE_CASE

impl cpu {
    pub fn ReadRegister(&self, index: u8) -> u32 {  // Methods should be snake_case
        self.Regs[index as usize]
    }
}
```

### Handling Hardware Terminology

PSX-specific terms follow official naming:

```rust
// ✅ Good example
pub struct GPU {
    vram: Vec<u16>,
}

pub struct SPU {
    voices: [Voice; 24],
}

pub enum COP0Register {
    SR = 12,   // Status Register
    CAUSE = 13,
    EPC = 14,
}

// CPU instructions use uppercase (following MIPS specification)
fn op_addiu(&mut self, rs: u8, rt: u8, imm: u16) { }
fn op_lw(&mut self, rs: u8, rt: u8, offset: i16) { }
```

---

## File Structure

### Directory Structure

```
src/
├── lib.rs                  # Library root
├── main.rs                 # Application entry point
├── core/                   # Emulator core
│   ├── mod.rs
│   ├── cpu/
│   │   ├── mod.rs         # Public API definition
│   │   ├── core.rs        # CPU struct and main logic
│   │   ├── instructions.rs # Instruction implementation
│   │   ├── cop0.rs        # Coprocessor 0
│   │   └── tests.rs       # Tests
│   ├── gpu/
│   │   ├── mod.rs
│   │   ├── core.rs
│   │   ├── renderer.rs
│   │   └── commands.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   └── bus.rs
│   └── system.rs
├── frontend/               # UI
│   ├── mod.rs
│   └── ui/
│       └── main.slint
└── util/                   # Utilities
    ├── mod.rs
    └── bitfield.rs
```

### Module Organization Rules

1. **1 file = 1 responsibility**
2. **mod.rs exposes only public API**
3. **Implementation details are private**

```rust
// src/core/cpu/mod.rs
mod core;
mod instructions;
mod cop0;

#[cfg(test)]
mod tests;

// public API
pub use core::CPU;
pub use cop0::COP0;

// Internal use only
pub(crate) use instructions::execute_instruction;
```

---

## Comments and Documentation

### Documentation Comments

**Documentation comments are required for all public APIs**

```rust
/// CPU (MIPS R3000A) emulation implementation
///
/// # Specifications
/// - Clock: 33.8688 MHz
/// - Instruction Set: MIPS I (32-bit)
/// - Registers: 32 general-purpose registers
///
/// # Examples
/// ```
/// use psx_emulator::core::CPU;
///
/// let mut cpu = CPU::new();
/// cpu.reset();
/// ```
pub struct CPU {
    /// General-purpose registers (r0-r31)
    ///
    /// r0 is hardwired to always return 0
    regs: [u32; 32],

    /// Program counter
    pc: u32,
}

impl CPU {
    /// Create a new CPU instance
    ///
    /// # Returns
    /// Initialized CPU instance
    pub fn new() -> Self {
        Self {
            regs: [0; 32],
            pc: 0xBFC00000,  // BIOS start address
        }
    }

    /// Read register value
    ///
    /// # Arguments
    /// - `index`: Register number (0-31)
    ///
    /// # Returns
    /// Register value. Always 0 for r0
    ///
    /// # Examples
    /// ```
    /// let value = cpu.reg(1);  // Get r1 value
    /// ```
    #[inline(always)]
    pub fn reg(&self, index: u8) -> u32 {
        if index == 0 {
            0
        } else {
            self.regs[index as usize]
        }
    }
}
```

### Implementation Comments

```rust
impl CPU {
    fn execute_instruction(&mut self, bus: &mut Bus) -> Result<()> {
        let instruction = self.fetch_instruction(bus)?;

        // Decode instruction (upper 6 bits are opcode)
        let opcode = instruction >> 26;

        match opcode {
            0x00 => {
                // SPECIAL instruction (determined by funct field)
                let funct = instruction & 0x3F;
                self.execute_special(funct, instruction)
            }
            0x08 => {
                // ADDI: Add immediate (with overflow exception)
                self.op_addi(instruction)
            }
            // ... other instructions
            _ => {
                log::warn!("Unknown opcode: 0x{:02X}", opcode);
                self.exception(ExceptionCause::ReservedInstruction);
                Ok(())
            }
        }
    }
}
```

### TODO Comments

```rust
// TODO: Implement recompiler
// TODO(performance): SIMD-ize this section
// TODO(spec check): Verify timing on actual hardware
// FIXME: Bug in edge cases
// HACK: Temporary workaround, fix later
```

### Complex Algorithm Explanations

```rust
/// Rasterize polygon (scanline method)
///
/// Algorithm:
/// 1. Sort vertices by Y coordinate
/// 2. Calculate left/right edges for each scanline
/// 3. Fill pixels between edges
fn rasterize_triangle(&mut self, v0: Vertex, v1: Vertex, v2: Vertex) {
    // Sort vertices by Y coordinate (v0.y <= v1.y <= v2.y)
    let (v0, v1, v2) = self.sort_vertices_by_y(v0, v1, v2);

    // ... implementation
}
```

---

## Error Handling

### Using Result Type

```rust
use thiserror::Error;

/// Errors during emulator execution
#[derive(Debug, Error)]
pub enum EmulatorError {
    #[error("Invalid memory access at address 0x{address:08X}")]
    InvalidMemoryAccess { address: u32 },

    #[error("Unknown instruction: 0x{0:08X}")]
    UnknownInstruction(u32),

    #[error("BIOS file not found: {0}")]
    BiosNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EmulatorError>;

// Usage example
impl CPU {
    pub fn step(&mut self, bus: &mut Bus) -> Result<u32> {
        let instruction = bus.read32(self.pc)?;
        self.execute(instruction)?;
        Ok(1)
    }
}
```

### When to Panic

```rust
// ✅ Good: Unrecoverable programming errors
pub fn new(ram_size: usize) -> Self {
    assert!(ram_size > 0, "RAM size must be positive");
    // ...
}

// ✅ Good: Internal invariant violation
fn get_register_unchecked(&self, index: u8) -> u32 {
    debug_assert!(index < 32, "Register index out of bounds");
    unsafe { *self.regs.get_unchecked(index as usize) }
}

// ❌ Bad: Panic on user input
pub fn load_bios(&mut self, path: &str) -> Result<()> {
    let data = std::fs::read(path)
        .expect("BIOS file must exist");  // ❌ Use ? instead of expect
    // ...
}
```

### Error Propagation

```rust
// ✅ Good: Propagate with ? operator
fn load_game(&mut self, path: &str) -> Result<()> {
    let data = std::fs::read(path)?;
    self.parse_iso(data)?;
    Ok(())
}

// ❌ Bad: Overuse of unwrap
fn load_game(&mut self, path: &str) {
    let data = std::fs::read(path).unwrap();  // ❌
    self.parse_iso(data).unwrap();  // ❌
}
```

---

## unsafe Usage

### Permitted Use Cases

1. **Performance-critical code**
2. **FFI (Foreign Function Interface)**
3. **Low-level memory operations**

### unsafe Usage Rules

```rust
// ✅ Good: Clear safety comments
impl CPU {
    /// Read register (without bounds checking)
    ///
    /// # Safety
    /// `index` must be in the range 0-31
    /// Caller must guarantee bounds checking
    #[inline(always)]
    pub unsafe fn reg_unchecked(&self, index: u8) -> u32 {
        debug_assert!(index < 32);
        *self.regs.get_unchecked(index as usize)
    }
}

// ✅ Good: Limit unsafe to minimal scope
pub fn read_memory(&self, addr: u32) -> u32 {
    let offset = self.translate_address(addr);

    // Keep unsafe block small
    unsafe {
        let ptr = self.ram.as_ptr().add(offset);
        ptr.read_unaligned()
    }
}

// ❌ Bad: unsafe block too large
pub fn process_data(&mut self) {
    unsafe {
        // 100 lines of unsafe code...
    }
}
```

---

## Test Code

### Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// CPU initialization test
    #[test]
    fn test_cpu_initialization() {
        let cpu = CPU::new();

        assert_eq!(cpu.pc, 0xBFC00000);
        assert_eq!(cpu.reg(0), 0);
    }

    /// ADDU instruction test
    #[test]
    fn test_addu_instruction() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 20);

        // ADDU r3, r1, r2
        cpu.op_addu(1, 2, 3);

        assert_eq!(cpu.reg(3), 30);
    }

    /// Overflow test
    #[test]
    fn test_add_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x7FFFFFFF);
        cpu.set_reg(2, 1);

        let result = cpu.op_add(1, 2, 3);

        // Overflow exception should occur
        assert!(result.is_err());
    }
}
```

### Test Naming Convention

```rust
#[test]
fn test_<feature>_<condition>_<expected_result>() { }

// Examples:
#[test]
fn test_memory_read_valid_address_returns_correct_value() { }

#[test]
fn test_cpu_branch_taken_updates_pc() { }

#[test]
fn test_gpu_command_invalid_code_returns_error() { }
```

---

## Formatting

### rustfmt Configuration

`.rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"

# Imports
imports_granularity = "Crate"
group_imports = "StdExternalCrate"

# Newlines
fn_single_line = false
where_single_line = true

# Comments
normalize_comments = true
wrap_comments = true
```

### Import Order

```rust
// 1. std
use std::collections::HashMap;
use std::fs;

// 2. External crates
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 3. Internal crates
use crate::core::cpu::CPU;
use crate::core::memory::Bus;

// 4. Current module
use super::instructions;
```

---

## Performance Considerations

### Inlining

```rust
// Aggressively inline hot paths
#[inline(always)]
pub fn reg(&self, index: u8) -> u32 {
    if index == 0 {
        0
    } else {
        self.regs[index as usize]
    }
}

// Don't inline large functions
#[inline(never)]
pub fn complex_operation(&mut self) {
    // 100+ lines of processing...
}
```

### Numeric Literals

```rust
// ✅ Good: Underscores for readability
const RAM_SIZE: usize = 2_097_152;  // 2MB
const CYCLES_PER_FRAME: u64 = 33_868_800 / 60;

// ✅ Good: Hexadecimal representation
const BIOS_START: u32 = 0xBFC0_0000;
const RAM_START: u32 = 0x0000_0000;
```

### Type Conversions

```rust
// ✅ Good: Explicit as, only when type is clear
let value: u32 = some_u8 as u32;

// ✅ Good: Use From/Into
let value: u32 = u32::from(some_u8);

// ❌ Bad: Implicit conversion
let value = some_u8 as _;  // Type unclear
```

---

## Clippy Configuration

`Cargo.toml`:
```toml
[lints.clippy]
# Treat as warnings
all = "warn"
pedantic = "warn"

# Treat as errors (fail CI)
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"

# Allowed rules
too_many_arguments = "allow"  # Common with hardware registers
similar_names = "allow"  # rs, rt, rd, etc.
```

---

## Git Commit Messages

### Conventional Commits

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only changes
- `style`: Formatting changes (no code behavior impact)
- `refactor`: Refactoring
- `perf`: Performance improvements
- `test`: Adding/fixing tests
- `chore`: Build/tool related

### Examples

```
feat(cpu): implement ADDI instruction

Add support for ADDI (Add Immediate) instruction.
Includes overflow detection and exception handling.

Closes #123
```

---

## Summary

### Checklist

Before development, verify:
- [ ] Following naming conventions
- [ ] Added documentation comments to public API
- [ ] Proper error handling
- [ ] Justified use of unsafe
- [ ] Added tests
- [ ] Ran `cargo fmt`
- [ ] No errors from `cargo clippy`

---

## Revision History

- 2025-10-28: Initial version
