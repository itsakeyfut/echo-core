# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**PSRX** is a PlayStation (PSX) emulator written in Rust, implementing the Sony PlayStation hardware including the MIPS R3000A CPU, CXD8561Q GPU, SPU, and other peripherals. The project follows a phased development approach with a focus on accuracy, maintainability, and performance.

## Development Commands

### Building and Testing

```bash
# Build the project
cargo x build

# Build with optimizations
cargo x build --release

# Run all tests
cargo x test

# Run specific module tests
cargo x test --cpu
cargo x test --gpu
cargo x test --memory

# Run multiple module tests
cargo x test --cpu --gpu --memory --system

# Run a single test
cargo test test_cpu_initialization

# Run tests with output
cargo test -- --nocapture

# Run tests by xtask
cargo x test

# Run benchmarks
cargo x bench
```

### Code Quality

```bash
# Format code (required before committing)
cargo x fmt

# Check formatting without modifying files
cargo x check

# Run clippy linter
cargo x clippy

# Check for warnings without building
cargo x build 2>&1 | grep -E "(warning|error)"
```

### Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate docs for dependencies too
cargo doc --open --no-deps
```

## Architecture Overview

### Layered Architecture

The emulator uses a **layered architecture** with clear separation of concerns:

1. **Core Layer** (`src/core/`): Hardware emulation
   - CPU (MIPS R3000A) - instruction interpreter
   - GPU (CXD8561Q) - graphics processor with 1MB VRAM
   - SPU - sound processing unit
   - Memory Bus - unified memory access with region mapping
   - System - top-level coordinator

2. **Frontend Layer** (`src/frontend/`): User interface (future)
   - Slint-based UI
   - Input handling
   - Configuration management

3. **Utility Layer** (`src/util/`): Shared utilities

### Component Communication

- **CPU drives execution**: The CPU is the master clock - all other components advance based on CPU cycles
- **Memory Bus mediates access**: All memory-mapped I/O goes through the Bus
- **System coordinates timing**: The `System` struct orchestrates component interactions

### Key Data Flows

```
CPU.step() → fetch instruction from Bus
         → execute instruction
         → Bus.read/write() → routes to appropriate component
                            → GPU registers, Memory, etc.

System.run_frame() → loop CPU.step() until frame complete
                  → GPU.tick() advances GPU state
                  → handle interrupts and timers
```

## Critical Implementation Details

### CPU Execution Model

- **Load delay slots**: Loads take effect one instruction later (MIPS pipeline behavior)
- **Branch delay slots**: Branch target executes after the following instruction
- **Register $0 is hardwired to zero**: Always returns 0, writes are ignored

### GPU VRAM Management

- **1024×512 pixels, 16-bit per pixel** (1MB total)
- **Row-major layout**: `index = y * 1024 + x`
- **Coordinate wrapping**: Use `x & 0x3FF` and `y & 0x1FF` for automatic wrapping
- **Color format**: 5-5-5 RGB (bit 15 is mask bit)

### Memory Map (Critical Addresses)

```
0x00000000-0x001FFFFF : RAM (2MB)
0x1F800000-0x1F8003FF : Scratchpad (1KB fast RAM)
0x1F801000-0x1F801FFF : I/O Ports
0x1F801810           : GPU GP0 (commands)
0x1F801814           : GPU GP1 (control) / GPUSTAT (read)
0x1FC00000-0x1FC7FFFF : BIOS ROM (512KB)
0x80000000-0x9FFFFFFF : Cached mirror of RAM
0xA0000000-0xBFFFFFFF : Uncached mirror of RAM
```

### Error Handling Philosophy

- **Use `Result<T>` for recoverable errors**: Memory access, file I/O, parsing
- **Use CPU exceptions for invalid operations**: Address errors, reserved instructions
- **Panic only for programmer errors**: Invalid internal state, assertions
- **Log unknown operations**: Use `log::warn!` for unimplemented features, don't crash

## Coding Standards (Key Points)

### Naming Conventions

- **Types/Structs/Enums**: `PascalCase` (e.g., `CPU`, `MemoryBus`, `GPUStatus`)
- **Functions/methods**: `snake_case` (e.g., `read_memory`, `execute_instruction`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `VRAM_WIDTH`, `CYCLES_PER_FRAME`)
- **Hardware terms**: Use official names (e.g., `GPU`, `SPU`, `COP0`)

### Documentation Requirements

- **All public APIs must have rustdoc comments** with examples
- **Module-level docs** explaining purpose and architecture
- **Complex algorithms need explanation** in comments
- **Hardware register formats** should be documented with bit layouts

### Performance Patterns

```rust
// Hot paths (CPU register access, VRAM access): use #[inline(always)]
#[inline(always)]
pub fn read_vram(&self, x: u16, y: u16) -> u16 {
    let index = self.vram_index(x, y);
    self.vram[index]
}

// Use numeric separators for readability
const RAM_SIZE: usize = 2_097_152;  // 2MB
const BIOS_START: u32 = 0xBFC0_0000;
```

### Testing Requirements

- **Unit tests** for each module in `#[cfg(test)] mod tests`
- **Test naming**: `test_<feature>_<condition>_<expected>`
- **Example**: `test_vram_read_write`, `test_cpu_branch_taken`
- Target: **70%+ code coverage**

## Development Workflow

### Commit Message Format (Conventional Commits)

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

**Example**:
```
feat(gpu): implement VRAM transfer commands

Add CPU-to-VRAM and VRAM-to-CPU transfer support.
Includes command parsing and DMA ready flags.

Closes #29
```

### Before Committing

1. Run `cargo x fmt`
2. Run `cargo x clippy`
3. Run `cargo x test`
4. Update relevant documentation
5. Add/update tests for new functionality

## Project Structure Context

### Core Module Organization

```
src/core/
├── cpu/
│   ├── mod.rs           # Public API, CPU struct
│   ├── cop0.rs          # Coprocessor 0 (system control)
│   ├── decode.rs        # Instruction decoding
│   ├── instructions/    # Instruction implementations by category
│   │   ├── arithmetic.rs
│   │   ├── branch.rs
│   │   ├── load.rs
│   │   └── ...
│   └── tests.rs         # CPU tests
├── gpu/
│   └── mod.rs           # GPU with VRAM, status, rendering state
├── memory/
│   └── mod.rs           # Memory bus with region routing
├── error.rs             # Error types (EmulatorError, GpuError, etc.)
└── system.rs            # System coordinator
```

### Test Organization

- **Unit tests**: In `#[cfg(test)] mod tests` within each module file
- **Integration tests**: Would go in `tests/` directory (not yet created)
- **Benchmarks**: In `benches/` directory

## Development Phase Context

Currently in **Phase 2 (Week 5)**: GPU Core Structure implementation

**Completed**:
- Phase 1: CPU core with MIPS instruction set
- Phase 1: Memory system with bus architecture
- Phase 1: Basic System integration
- Phase 2 Week 5: GPU core structure with VRAM management

**Current Focus**:
- GPU drawing commands (GP0)
- GPU control commands (GP1)
- VRAM transfer operations

**See**: `specs/05-development/roadmap.md` for full development timeline

## Spec Documentation

The `specs/` directory contains comprehensive design documentation:

- `specs/00-overview/architecture.md` - System architecture
- `specs/01-design/` - Component design docs (CPU, GPU, Memory)
- `specs/02-implementation/` - Coding standards, error handling, testing
- `specs/03-hardware-specs/` - PSX hardware specifications
- `specs/04-reference/` - Instruction sets, command references
- `specs/05-development/roadmap.md` - Development phases and timeline

**Always consult relevant spec documents** when implementing new features.

## Common Patterns

### Implementing a New CPU Instruction

1. Add function in appropriate `src/core/cpu/instructions/*.rs`
2. Follow naming: `pub fn op_<instruction>()`
3. Add to dispatcher in `decode.rs`
4. Write unit test in `tests.rs`
5. Update documentation if needed

### Adding GPU Functionality

1. Check `specs/01-design/gpu-design.md` for design
2. Check `specs/03-hardware-specs/gpu-cxd8561.md` for hardware details
3. Update `GPU` struct if needed
4. Implement methods with inline docs
5. Add comprehensive tests

### Memory-Mapped I/O

1. Define address constants (e.g., `GPU_GP0: u32 = 0x1F801810`)
2. Add to `Bus::read*/write*` match statements
3. Route to appropriate component
4. Document the mapping

## Issue Tracking Context

Issues are tracked with labels:
- `component:cpu`, `component:gpu`, `component:memory`, etc.
- `type:feature`, `type:bug`, `type:enhancement`
- `phase:1`, `phase:2`, etc. (development phase)
- `priority:critical`, `priority:high`, etc.
- `difficulty:easy`, `difficulty:medium`, `difficulty:hard`

When implementing an issue:
1. Reference issue number in commits
2. Follow the requirements exactly as specified
3. Complete all acceptance criteria
4. Add requested tests
5. Close with `Closes #<number>` in commit message

## Important Notes

- **Comments and docs must be in English** (code was originally Japanese, being migrated)
- **BIOS files are NOT included** - users must provide their own
- **Performance matters**: This is a real-time emulator targeting 60fps
- **Accuracy matters**: Hardware behavior should match real PSX when practical
- **No `unwrap()` or `expect()` in production code** - use proper error handling

## References

- PSX-SPX: http://problemkaputt.de/psx-spx.htm (primary hardware reference)
- No$ PSX specifications
- DuckStation (modern PSX emulator) for implementation reference
