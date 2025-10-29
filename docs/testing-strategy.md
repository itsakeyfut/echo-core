# Testing Strategy

## Overview

In emulator development, testing is essential to ensure accuracy and quality. This document defines a comprehensive testing strategy for the PSX emulator.

## Types of Tests

```
┌─────────────────────────────────────────┐
│         Test Pyramid                    │
├─────────────────────────────────────────┤
│                                         │
│            ┌─────────┐                 │
│            │  E2E    │  Compatibility  │
│            │  Tests  │  (Real games)   │
│            └─────────┘                 │
│         ┌─────────────────┐            │
│         │ Integration     │            │
│         │    Tests        │            │
│         │  (Components)   │            │
│         └─────────────────┘            │
│    ┌──────────────────────────┐       │
│    │     Unit Tests           │       │
│    │  (Individual functions)  │       │
│    └──────────────────────────┘       │
└─────────────────────────────────────────┘
```

### 1. Unit Tests

**Target:** Individual functions, methods, modules
**Frequency:** Every commit, CI/CD
**Coverage Goal:** 70%+ (90%+ for core modules)

### 2. Integration Tests

**Target:** Interaction between multiple components
**Frequency:** Pull requests, CI/CD
**Examples:** CPU + Memory Bus, GPU + DMA

### 3. CPU Instruction Tests

**Target:** MIPS instruction set accuracy
**Frequency:** Manual + CI/CD
**Using:** Test ROMs (amidog's tests, etc.)

### 4. Compatibility Tests

**Target:** Actual game ROMs
**Frequency:** Manual, weekly
**Goal:** 95%+ boot success rate

### 5. Performance Tests

**Target:** Frame rate, latency
**Frequency:** Before releases
**Tools:** cargo-criterion

---

## Unit Tests

### Basic Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange (setup)
        let mut cpu = CPU::new();

        // Act (execute)
        let result = cpu.some_operation();

        // Assert (verify)
        assert_eq!(result, expected_value);
    }
}
```

### CPU Instruction Test Examples

```rust
#[cfg(test)]
mod cpu_tests {
    use super::*;

    /// ADDU instruction test
    #[test]
    fn test_addu_basic() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 10);
        cpu.set_reg(2, 20);

        // ADDU r3, r1, r2
        cpu.op_addu(1, 2, 3);

        assert_eq!(cpu.reg(3), 30);
        assert_eq!(cpu.reg(1), 10);  // r1 unchanged
        assert_eq!(cpu.reg(2), 20);  // r2 unchanged
    }

    /// ADDU overflow test (no exception)
    #[test]
    fn test_addu_overflow_no_exception() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFFFFFF);
        cpu.set_reg(2, 1);

        // No exception on overflow
        cpu.op_addu(1, 2, 3);

        assert_eq!(cpu.reg(3), 0);  // Wrap around
    }

    /// r0 write test
    #[test]
    fn test_reg0_always_zero() {
        let mut cpu = CPU::new();

        // Writes to r0 are ignored
        cpu.set_reg(0, 0xDEADBEEF);

        assert_eq!(cpu.reg(0), 0);
    }

    /// ADD instruction overflow exception test
    #[test]
    fn test_add_overflow_exception() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0x7FFFFFFF);  // Maximum positive number
        cpu.set_reg(2, 1);

        let result = cpu.op_add(1, 2, 3);

        // Overflow exception should occur
        assert!(result.is_err() || cpu.exception_occurred());
    }

    /// Branch delay slot test
    #[test]
    fn test_branch_delay_slot() {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Load program to RAM
        // BEQ + ADDIU (delay slot)
        bus.write32(0x80000000, 0x10200001).unwrap();  // BEQ r1, r0, +1
        bus.write32(0x80000004, 0x24020005).unwrap();  // ADDIU r2, r0, 5
        bus.write32(0x80000008, 0x24030007).unwrap();  // ADDIU r3, r0, 7

        cpu.pc = 0x80000000;
        cpu.set_reg(1, 0);  // r1 == 0 so branch taken

        // Execute BEQ
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.reg(2), 0);  // Not executed yet

        // Execute delay slot (ADDIU r2, r0, 5)
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.reg(2), 5);  // Delay slot executed

        // Branch target (ADDIU r3, r0, 7 is skipped)
        assert_eq!(cpu.reg(3), 0);
    }
}
```

### Memory System Tests

```rust
#[cfg(test)]
mod memory_tests {
    use super::*;

    #[test]
    fn test_address_translation() {
        let bus = Bus::new();

        // KUSEG
        assert_eq!(bus.translate_address(0x00001234), 0x00001234);

        // KSEG0 (cache enabled)
        assert_eq!(bus.translate_address(0x80001234), 0x00001234);

        // KSEG1 (cache disabled)
        assert_eq!(bus.translate_address(0xA0001234), 0x00001234);
    }

    #[test]
    fn test_ram_read_write_consistency() {
        let mut bus = Bus::new();

        // Write in each segment
        bus.write32(0x00001000, 0x12345678).unwrap();

        // Read from different segments, same value
        assert_eq!(bus.read32(0x00001000).unwrap(), 0x12345678);
        assert_eq!(bus.read32(0x80001000).unwrap(), 0x12345678);
        assert_eq!(bus.read32(0xA0001000).unwrap(), 0x12345678);
    }

    #[test]
    fn test_bios_read_only() {
        let mut bus = Bus::new();

        // Read initial BIOS value
        let original = bus.read32(0xBFC00000).unwrap();

        // Writes to BIOS are ignored
        bus.write32(0xBFC00000, 0xDEADBEEF).unwrap();

        // Value unchanged
        assert_eq!(bus.read32(0xBFC00000).unwrap(), original);
    }

    #[test]
    fn test_alignment_check() {
        let bus = Bus::new();

        // Misaligned (32-bit read)
        assert!(bus.read32(0x80000001).is_err());
        assert!(bus.read32(0x80000002).is_err());
        assert!(bus.read32(0x80000003).is_err());

        // Proper alignment
        assert!(bus.read32(0x80000000).is_ok());
        assert!(bus.read32(0x80000004).is_ok());

        // 16-bit read alignment
        assert!(bus.read16(0x80000001).is_err());
        assert!(bus.read16(0x80000000).is_ok());
        assert!(bus.read16(0x80000002).is_ok());
    }

    #[test]
    fn test_little_endian() {
        let mut bus = Bus::new();

        // Write in little-endian
        bus.write8(0x80001000, 0x78).unwrap();
        bus.write8(0x80001001, 0x56).unwrap();
        bus.write8(0x80001002, 0x34).unwrap();
        bus.write8(0x80001003, 0x12).unwrap();

        // 32-bit read
        assert_eq!(bus.read32(0x80001000).unwrap(), 0x12345678);
    }
}
```

### GPU Tests

```rust
#[cfg(test)]
mod gpu_tests {
    use super::*;

    #[test]
    fn test_vram_initialization() {
        let gpu = GPU::new();

        // Verify VRAM size
        assert_eq!(gpu.vram.len(), 1024 * 512);

        // Initial value is 0
        assert_eq!(gpu.vram[0], 0);
    }

    #[test]
    fn test_gp1_reset() {
        let mut gpu = GPU::new();

        // Modify drawing state
        gpu.draw_offset = (100, 200);

        // GP1(0x00): Reset
        gpu.write_gp1(0x00000000);

        // Reset to initial state
        assert_eq!(gpu.draw_offset, (0, 0));
    }

    #[test]
    fn test_vram_transfer() {
        let mut gpu = GPU::new();

        // VRAM write command (GP0 0xA0)
        gpu.write_gp0(0xA0000000);  // CPU→VRAM transfer
        gpu.write_gp0(0x00000000);  // Coordinates (0, 0)
        gpu.write_gp0(0x00010001);  // Size 1x1
        gpu.write_gp0(0x12345678);  // Data

        // Written to VRAM
        let pixel = gpu.read_vram(0, 0);
        assert_eq!(pixel, 0x5678);  // Lower 16 bits
    }
}
```

---

## Integration Tests

Integration tests are placed in the `tests/` directory.

```rust
// tests/cpu_memory_integration.rs

use psx_emulator::core::{CPU, Bus};

#[test]
fn test_cpu_executes_from_ram() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Load simple program to RAM
    // ADDIU r1, r0, 5
    bus.write32(0x80000000, 0x24010005).unwrap();
    // ADDIU r2, r1, 10
    bus.write32(0x80000004, 0x24220010).unwrap();

    cpu.pc = 0x80000000;

    // Execute first instruction
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.reg(1), 5);

    // Execute second instruction
    cpu.step(&mut bus).unwrap();
    assert_eq!(cpu.reg(2), 15);
}

#[test]
fn test_exception_handling() {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Invalid instruction
    bus.write32(0x80000000, 0xFFFFFFFF).unwrap();

    cpu.pc = 0x80000000;

    // Exception occurs
    let result = cpu.step(&mut bus);

    // EPC stores PC at exception
    assert_eq!(cpu.cop0.regs[COP0::EPC], 0x80000000);

    // CAUSE has exception code
    let cause = cpu.cop0.regs[COP0::CAUSE];
    assert_ne!(cause & 0x7C, 0);  // Exception code non-zero
}
```

---

## CPU Instruction Test ROMs

### Using Existing Test ROMs

```rust
// tests/cpu_instruction_tests.rs

use psx_emulator::core::{System};
use std::fs;

#[test]
#[ignore]  // Skip by default (manual execution)
fn test_amidog_cpu_tests() {
    let rom = fs::read("tests/roms/cpu_tests.bin")
        .expect("CPU test ROM not found");

    let mut system = System::new();
    system.load_rom(&rom).unwrap();

    // Execute test ROM
    for _ in 0..1_000_000 {
        system.step().unwrap();
    }

    // Check test result (check specific memory address)
    let result = system.bus.read32(0x80001000).unwrap();
    assert_eq!(result, 0);  // 0 = all tests passed
}
```

### Creating Custom Test ROMs

```assembly
; simple_test.asm (MIPS assembly)
.org 0x80000000

    ; Test 1: ADDI
    addi $t0, $zero, 42

    ; Test 2: LW/SW
    sw $t0, 0x1000($zero)
    lw $t1, 0x1000($zero)

    ; Result check
    bne $t0, $t1, fail
    nop

success:
    li $v0, 0
    j end
    nop

fail:
    li $v0, 1
    j end
    nop

end:
    break
```

---

## Property-Based Testing

Property-based testing with random inputs (using proptest)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_address_translation_consistency(addr in 0u32..0x20000000) {
        let bus = Bus::new();

        // KUSEG, KSEG0, KSEG1 map to same physical address
        let kuseg_addr = addr;
        let kseg0_addr = addr | 0x80000000;
        let kseg1_addr = addr | 0xA0000000;

        let physical1 = bus.translate_address(kuseg_addr);
        let physical2 = bus.translate_address(kseg0_addr);
        let physical3 = bus.translate_address(kseg1_addr);

        prop_assert_eq!(physical1, physical2);
        prop_assert_eq!(physical2, physical3);
    }

    #[test]
    fn test_memory_write_read_consistency(
        addr in (0u32..0x200000).prop_map(|a| (a & !0x3)),  // Aligned
        value in any::<u32>()
    ) {
        let mut bus = Bus::new();

        bus.write32(addr, value).unwrap();
        let read_value = bus.read32(addr).unwrap();

        prop_assert_eq!(value, read_value);
    }
}
```

---

## Compatibility Tests

### Test Game List

```rust
// tests/compatibility_tests.rs

struct GameTest {
    name: &'static str,
    iso_path: &'static str,
    expected_result: TestResult,
}

enum TestResult {
    BootToMenu,      // Boot from BIOS menu
    InGame,          // Reach game screen
    Playable,        // Playable
}

const TEST_GAMES: &[GameTest] = &[
    GameTest {
        name: "Final Fantasy VII",
        iso_path: "tests/games/ff7.bin",
        expected_result: TestResult::Playable,
    },
    GameTest {
        name: "Metal Gear Solid",
        iso_path: "tests/games/mgs.bin",
        expected_result: TestResult::InGame,
    },
    // ... more games
];

#[test]
#[ignore]
fn test_game_compatibility() {
    for game in TEST_GAMES {
        let mut system = System::new();
        system.load_game(game.iso_path).unwrap();

        // Run for certain frames
        for _ in 0..600 {  // 10 seconds @ 60fps
            system.run_frame().unwrap();
        }

        // Verify result (specific screen pattern, memory state, etc.)
        assert!(system.check_game_state(&game.expected_result));
    }
}
```

---

## Performance Tests

### Benchmarking with Criterion

```rust
// benches/cpu_benchmark.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use psx_emulator::core::{CPU, Bus};

fn benchmark_cpu_instruction_execution(c: &mut Criterion) {
    let mut cpu = CPU::new();
    let mut bus = Bus::new();

    // Load test program
    bus.write32(0x80000000, 0x24010005).unwrap();  // ADDIU
    cpu.pc = 0x80000000;

    c.bench_function("cpu_step", |b| {
        b.iter(|| {
            cpu.step(black_box(&mut bus))
        })
    });
}

fn benchmark_memory_access(c: &mut Criterion) {
    let mut bus = Bus::new();

    c.bench_function("memory_read32", |b| {
        b.iter(|| {
            bus.read32(black_box(0x80001000))
        })
    });

    c.bench_function("memory_write32", |b| {
        b.iter(|| {
            bus.write32(black_box(0x80001000), black_box(0x12345678))
        })
    });
}

criterion_group!(benches, benchmark_cpu_instruction_execution, benchmark_memory_access);
criterion_main!(benches);
```

### Running Benchmarks

```bash
cargo criterion
```

### Target Performance

```
CPU instruction execution:   < 100ns/instruction
Memory read:                 < 50ns
Memory write:                < 50ns
1 frame execution:           < 16ms (60fps)
```

---

## CI/CD Integration

### GitHub Actions Configuration Example

```yaml
# .github/workflows/test.yml

name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Run benchmarks
        run: cargo criterion --no-run
```

---

## Test Coverage

### Coverage Measurement with Tarpaulin

```bash
# Install
cargo install cargo-tarpaulin

# Measure coverage
cargo tarpaulin --out Html --output-dir coverage
```

### Coverage Goals

| Module | Target Coverage |
|--------|----------------|
| CPU | 90% |
| Memory | 90% |
| GPU | 80% |
| SPU | 70% |
| Overall | 70% |

---

## Test Execution Guide

### Run All Tests

```bash
cargo test
```

### Run Specific Tests Only

```bash
# CPU-related tests only
cargo test cpu

# Integration tests only
cargo test --test '*'

# Benchmarks only
cargo criterion
```

### Run Ignored Tests

```bash
# Run time-consuming tests too
cargo test -- --ignored

# Run all
cargo test -- --include-ignored
```

### Detailed Output

```bash
cargo test -- --nocapture
```

---

## Mocks and Fixtures

### Creating Mocks

```rust
// tests/mocks.rs

pub struct MockBus {
    pub reads: Vec<u32>,
    pub writes: Vec<(u32, u32)>,
}

impl MockBus {
    pub fn new() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }
}

impl MemoryAccess for MockBus {
    fn read32(&self, addr: u32) -> Result<u32> {
        self.reads.push(addr);
        Ok(0)
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<()> {
        self.writes.push((addr, value));
        Ok(())
    }
}

// Usage example
#[test]
fn test_with_mock() {
    let mut cpu = CPU::new();
    let mut mock_bus = MockBus::new();

    cpu.step(&mut mock_bus).unwrap();

    // Verify memory access
    assert_eq!(mock_bus.reads.len(), 1);
}
```

---

## Debug Support

### Assertion Macros

```rust
// src/util/test_helpers.rs

#[macro_export]
macro_rules! assert_cpu_reg {
    ($cpu:expr, $reg:expr, $expected:expr) => {
        assert_eq!(
            $cpu.reg($reg),
            $expected,
            "Register r{} mismatch: expected 0x{:08X}, got 0x{:08X}",
            $reg,
            $expected,
            $cpu.reg($reg)
        );
    };
}

// Usage example
#[test]
fn test_with_helper() {
    let mut cpu = CPU::new();
    cpu.set_reg(1, 42);

    assert_cpu_reg!(cpu, 1, 42);
}
```

---

## Summary

### Test Implementation Checklist

**During Development:**
- [ ] Add unit tests for new features
- [ ] All `cargo test` pass
- [ ] No errors from `cargo clippy`

**Before Pull Request:**
- [ ] Integration tests added
- [ ] Coverage meets target
- [ ] No performance degradation in benchmarks

**Before Release:**
- [ ] Compatibility tests achieve 95%+ success rate
- [ ] Performance tests meet targets
- [ ] Run all ignored tests

---

## Related Documents

- [Coding Standards](./coding-standards.md)
- [Development Environment Setup](../05-development/setup.md)
- [CPU Design](../01-design/cpu-design.md)

---

## Revision History

- 2025-10-28: Initial version
