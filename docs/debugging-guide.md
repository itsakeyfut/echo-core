# Debugging Guide

## Overview

This guide covers the debugging tools and techniques available in the echo-core PlayStation emulator. These tools are essential for understanding CPU execution, troubleshooting issues, and verifying correct behavior.

## Obtaining a BIOS File

To run the emulator, you need a PlayStation BIOS file. **Important legal notes:**

- You must own an actual PlayStation console to legally use its BIOS
- BIOS files cannot be distributed or downloaded from unauthorized sources
- Common BIOS files include: SCPH1001.BIN (US), SCPH7502.BIN (EU), SCPH5500.BIN (JP)

The BIOS file should be 512KB (524,288 bytes) in size.

## Debugging Tools

### 1. Disassembler

The `Disassembler` converts binary MIPS instruction encodings into human-readable assembly format.

#### Usage

```rust
use echo_core::core::cpu::Disassembler;

let instruction = 0x3C011234; // LUI r1, 0x1234
let pc = 0xBFC00000;
let disasm = Disassembler::disassemble(instruction, pc);
println!("{}", disasm); // Output: "lui r1, 0x1234"
```

#### Supported Instructions

The disassembler supports all common MIPS R3000A instructions including:

- **Arithmetic**: ADD, ADDI, ADDU, ADDIU, SUB, SUBU
- **Logical**: AND, ANDI, OR, ORI, XOR, XORI, NOR
- **Shift**: SLL, SRL, SRA, SLLV, SRLV, SRAV
- **Branch**: BEQ, BNE, BLEZ, BGTZ, BLTZ, BGEZ
- **Jump**: J, JAL, JR, JALR
- **Load/Store**: LW, LH, LB, LBU, LHU, SW, SH, SB, LWL, LWR, SWL, SWR
- **Multiply/Divide**: MULT, MULTU, DIV, DIVU, MFHI, MFLO, MTHI, MTLO
- **Compare**: SLT, SLTI, SLTU, SLTIU
- **COP0**: MFC0, MTC0, RFE
- **System**: SYSCALL, BREAK

Unknown instructions are displayed as `??? 0xXXXXXXXX`.

### 2. CPU Tracer

The `CpuTracer` logs CPU execution state to a file for analysis. Each trace line includes:

- Program counter (PC)
- Raw instruction encoding
- Disassembled instruction
- Selected register values

#### Basic Usage

```rust
use echo_core::core::cpu::{CPU, CpuTracer};
use echo_core::core::memory::Bus;

let mut cpu = CPU::new();
let mut bus = Bus::new();
let mut tracer = CpuTracer::new("trace.log").unwrap();

// Before each instruction execution
tracer.trace(&cpu, &bus).unwrap();
cpu.step(&mut bus).unwrap();
```

#### Trace Output Format

```
PC=0xBFC00000 [0x3C1C1F80] lui r28, 0x1F80              | r1=00000000 r2=00000000 r3=00000000
PC=0xBFC00004 [0x27BDFFE0] addiu r29, r29, -32         | r1=00000000 r2=00000000 r3=00000000
PC=0xBFC00008 [0xAFBF001C] sw r31, 28(r29)             | r1=00000000 r2=00000000 r3=00000000
```

#### Advanced Usage

Trace with custom register selection:

```rust
// Trace specific registers
tracer.trace_with_regs(&cpu, &bus, &[4, 5, 6, 7]).unwrap();
```

Enable/disable tracing dynamically:

```rust
tracer.set_enabled(false); // Pause tracing
// ... some operations ...
tracer.set_enabled(true);  // Resume tracing
```

Flush output to disk:

```rust
tracer.flush().unwrap();
```

### 3. Register Dump

The `dump_registers()` method prints a formatted view of all CPU state.

#### Usage

```rust
use echo_core::core::cpu::CPU;

let cpu = CPU::new();
cpu.dump_registers();
```

#### Output Format

```
CPU Registers:
PC: 0xBFC00000  Next PC: 0xBFC00004
HI: 0x00000000  LO: 0x00000000

r 0: 0x00000000  r 1: 0x00000000  r 2: 0x00000000  r 3: 0x00000000
r 4: 0x00000000  r 5: 0x00000000  r 6: 0x00000000  r 7: 0x00000000
r 8: 0x00000000  r 9: 0x00000000  r10: 0x00000000  r11: 0x00000000
r12: 0x00000000  r13: 0x00000000  r14: 0x00000000  r15: 0x00000000
r16: 0x00000000  r17: 0x00000000  r18: 0x00000000  r19: 0x00000000
r20: 0x00000000  r21: 0x00000000  r22: 0x00000000  r23: 0x00000000
r24: 0x00000000  r25: 0x00000000  r26: 0x00000000  r27: 0x00000000
r28: 0x00000000  r29: 0x00000000  r30: 0x00000000  r31: 0x00000000

COP0 Registers:
SR:    0x10900000
CAUSE: 0x00000000
EPC:   0x00000000
BADA:  0x00000000
PRID:  0x00000002
```

## Command-Line Tool

The `echo-core` CLI tool allows you to load and run a BIOS file.

### Usage

```bash
cargo run --release -- <bios_file>
```

### Examples

```bash
# Run with BIOS file in current directory
cargo run --release -- SCPH1001.BIN

# Run with absolute path
cargo run --release -- /path/to/SCPH1001.BIN

# With logging enabled
RUST_LOG=debug cargo run --release -- SCPH1001.BIN
```

### Environment Variables

- `RUST_LOG`: Set logging level (trace, debug, info, warn, error)
  - `trace`: Very verbose, logs every detail
  - `debug`: Detailed debugging information
  - `info`: General information (default)
  - `warn`: Warnings only
  - `error`: Errors only

### Example Output

```
[INFO] echo-core v0.1.0
[INFO] PlayStation emulator
[INFO] Loading BIOS from: SCPH1001.BIN
[INFO] BIOS loaded successfully
[INFO] Starting emulation...
[INFO] Progress: 10000/100000 instructions | PC: 0xBFC00234 | Cycles: 10000
[INFO] Progress: 20000/100000 instructions | PC: 0xBFC00456 | Cycles: 20000
...
[INFO] Emulation completed successfully!
[INFO] Total instructions: 100000
[INFO] Total cycles: 100000
[INFO] Final PC: 0xBFC01234
```

### Error Handling

If an error occurs during execution, the tool will:

1. Display the error message
2. Show the instruction count where the error occurred
3. Dump all CPU registers for debugging

Example error output:

```
[ERROR] Error at PC=0xBFC00234: Unsupported instruction: 0xFFFFFFFF
[ERROR] Instruction count: 1234
CPU Registers:
PC: 0xBFC00234  Next PC: 0xBFC00238
...
```

## Testing

### BIOS Boot Test

An integration test verifies that the emulator can successfully boot and execute the PlayStation BIOS.

#### Running the Test

The BIOS boot test is marked with `#[ignore]` because it requires an actual BIOS file.

```bash
# Set BIOS path and run the test
PSX_BIOS_PATH=SCPH1001.BIN cargo test test_bios_boot -- --ignored --nocapture

# Or place SCPH1001.BIN in project root
cargo test test_bios_boot -- --ignored --nocapture
```

#### Test Criteria

The test verifies:

1. BIOS file can be loaded successfully
2. Emulator can execute at least 10,000 instructions without crashing
3. PC advances from the initial BIOS entry point (0xBFC00000)
4. No unexpected exceptions occur

#### Expected Output

```
running 1 test
BIOS loaded successfully from: SCPH1001.BIN
Starting BIOS execution test...
Initial PC: 0xBFC00000
Progress: 1000/10000 | PC: 0xBFC00124 | Cycles: 1000
Progress: 2000/10000 | PC: 0xBFC00234 | Cycles: 2000
...

BIOS boot test completed successfully!
Executed 10000 instructions
Total cycles: 10000
Final PC: 0xBFC01234
test core::system::tests::test_bios_boot ... ok
```

## Debugging Workflow

### Basic Debugging Session

1. **Start with unit tests**: Verify individual instructions work correctly

```bash
cargo test
```

2. **Run BIOS boot test**: Verify system integration

```bash
PSX_BIOS_PATH=SCPH1001.BIN cargo test test_bios_boot -- --ignored --nocapture
```

3. **Enable tracing for detailed analysis**:

```rust
let mut tracer = CpuTracer::new("trace.log").unwrap();
for _ in 0..1000 {
    tracer.trace(&cpu, &bus).unwrap();
    cpu.step(&mut bus).unwrap();
}
```

4. **Analyze trace file**: Look for unexpected behavior

```bash
# View trace file
less trace.log

# Search for specific instructions
grep "sw r" trace.log

# Find branches
grep -E "(beq|bne|j |jr)" trace.log
```

5. **Use register dump on error**:

```rust
match cpu.step(&mut bus) {
    Ok(_) => {},
    Err(e) => {
        println!("Error: {}", e);
        cpu.dump_registers();
    }
}
```

### Common Issues and Solutions

#### Issue: Invalid instruction exception

**Symptoms**: Emulator crashes with "Unsupported instruction"

**Debugging**:
1. Use register dump to see the PC and instruction
2. Use disassembler to verify the instruction encoding
3. Check if the instruction is implemented

```rust
let instruction = bus.read32(cpu.pc()).unwrap();
println!("Instruction: {:08X}", instruction);
println!("Disassembly: {}", Disassembler::disassemble(instruction, cpu.pc()));
```

#### Issue: Infinite loop

**Symptoms**: PC doesn't advance or stays in small range

**Debugging**:
1. Enable trace for a few hundred instructions
2. Look for repeated PC values
3. Check branch conditions

```bash
# Find repeated PCs in trace
cut -d' ' -f1 trace.log | uniq -c | sort -rn | head
```

#### Issue: Wrong register values

**Symptoms**: Tests fail with incorrect register values

**Debugging**:
1. Use tracer to log register values
2. Compare with expected execution
3. Verify load delay slot handling

```rust
tracer.trace_with_regs(&cpu, &bus, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
```

## Performance Profiling

To identify performance bottlenecks:

```bash
# Build with release optimizations
cargo build --release

# Run with timing
time ./target/release/echo-core SCPH1001.BIN

# Profile with perf (Linux)
perf record ./target/release/echo-core SCPH1001.BIN
perf report
```

## Reference

### Key Files

- `src/core/cpu/disassembler.rs` - Instruction disassembler
- `src/core/cpu/tracer.rs` - Execution tracer
- `src/core/cpu/mod.rs` - CPU implementation with register dump
- `src/bin/echo-core.rs` - CLI tool
- `src/core/system.rs` - BIOS boot test

### Useful Resources

- [PSX-SPX Documentation](http://problemkaputt.de/psx-spx.htm) - Complete PlayStation hardware reference
- [MIPS R3000A Manual](https://www.linux-mips.org/pub/linux/mips/doc/R3000.pdf) - CPU instruction set
- [No$ PSX](https://problemkaputt.de/psx.htm) - Reference emulator with debugger

## Best Practices

1. **Always verify with tests**: Write unit tests before using interactive debugging
2. **Start simple**: Debug with simple programs before trying full BIOS
3. **Use appropriate tools**: Disassembler for quick checks, tracer for detailed analysis
4. **Flush trace output**: Always flush the tracer before analyzing files
5. **Check PC progression**: Ensure PC advances correctly and doesn't get stuck
6. **Compare with reference**: Compare register values and behavior with known-good emulators
7. **Version control traces**: Keep trace files of working states for regression testing

## Troubleshooting

### BIOS won't load

```
Error: BIOS file not found: SCPH1001.BIN
```

**Solution**: Verify the file path and ensure the BIOS file exists

### Test fails immediately

```
Error at PC=0xBFC00000: Invalid memory access at 0xBFC00000
```

**Solution**: Check that BIOS is correctly loaded into memory at 0xBFC00000

### Trace file is empty

**Solution**: Ensure you call `tracer.flush()` or the file handle is dropped before reading

### Too much output

**Solution**: Disable tracing for known-good sections:

```rust
tracer.set_enabled(false);
// Run through known-good code
system.step_n(10000).unwrap();
tracer.set_enabled(true);
// Now trace the interesting part
```

## Next Steps

After successfully booting the BIOS:

1. Implement GPU rendering (Phase 2)
2. Add controller input
3. Implement CD-ROM reading
4. Load and run game executables
5. Add save state support

For more information, see the project roadmap in `specs/05-development/roadmap.md`.
