# PSRX (PlayStation Rust eXecutable) - PlayStation Emulator

A PlayStation (PSX) emulator written in Rust, implementing the Sony PlayStation hardware including the MIPS R3000A CPU, CXD8561Q GPU, SPU, and other peripherals.

## Building and Running

### Prerequisites

- Rust 1.70 or later
- A PlayStation BIOS file (e.g., SCPH1001.BIN) - not included due to copyright

### Building

```bash
# Build the project
cargo build --release

# Or use the xtask build system
cargo x build --release
```

### Running

```bash
# Run the emulator (CLI mode)
./target/release/psrx path/to/SCPH1001.BIN
```

> **Note**: GUI frontend is planned. Currently only CLI mode is available.

## Configuration

PSRX supports configuration via environment variables using a `.env` file. This allows you to customize various emulator settings without modifying code.

### Quick Start

1. Copy the example configuration file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` to customize settings:
   ```bash
   # Enable CPU tracing for debugging
   PSRX_TRACE_ENABLED=true
   PSRX_TRACE_LIMIT=10000
   ```

3. Run the emulator - settings will be loaded automatically

### Available Configuration Options

#### CPU Tracing

Control CPU instruction tracing for debugging and development:

```bash
# Enable CPU instruction tracing for debugging
# Set to "true" to enable, "false" to disable
# Default: false
PSRX_TRACE_ENABLED=false

# Maximum number of instructions to trace (0 = unlimited)
# Only used when PSRX_TRACE_ENABLED=true
# Default: 10000
PSRX_TRACE_LIMIT=10000

# Output file for CPU trace
# Default: bios_trace.log
PSRX_TRACE_FILE=bios_trace.log
```

**Example**: To trace the first 5000 BIOS instructions:
```bash
PSRX_TRACE_ENABLED=true
PSRX_TRACE_LIMIT=5000
PSRX_TRACE_FILE=bios_trace.log
```

#### Logging

Control the log level for the emulator:

```bash
# Log level: error, warn, info, debug, trace
# This sets the global log level for the emulator
# Default: info
RUST_LOG=info
```

**Log Levels**:
- `error` - Only critical errors
- `warn` - Warnings and errors
- `info` - General information (recommended for normal use)
- `debug` - Detailed debugging information
- `trace` - Very verbose output (all operations)

**Example**: Enable verbose logging for debugging:
```bash
RUST_LOG=debug
```

#### Development Options

Experimental features for development:

```bash
# Enable VBLANK interrupts (experimental)
# The BIOS may not be ready for interrupts during early boot
# Default: false
PSRX_VBLANK_ENABLED=false

# VBLANK interrupt frequency in CPU cycles
# PlayStation runs at ~33.8688 MHz, 60Hz = ~564,480 cycles
# Default: 564480
PSRX_VBLANK_CYCLES=564480
```

### Configuration Examples

#### Default Configuration (Normal Use)
```bash
PSRX_TRACE_ENABLED=false
RUST_LOG=info
PSRX_VBLANK_ENABLED=false
```

#### Debugging BIOS Boot Issues
```bash
# Enable tracing for first 50,000 instructions
PSRX_TRACE_ENABLED=true
PSRX_TRACE_LIMIT=50000
PSRX_TRACE_FILE=bios_boot_trace.log

# Enable detailed logging
RUST_LOG=debug
```

#### Performance Testing
```bash
# Disable tracing for maximum performance
PSRX_TRACE_ENABLED=false

# Minimal logging
RUST_LOG=warn
```

#### Full Debug Mode
```bash
# Unlimited tracing
PSRX_TRACE_ENABLED=true
PSRX_TRACE_LIMIT=0
PSRX_TRACE_FILE=full_trace.log

# Maximum verbosity
RUST_LOG=trace
```

## Development

For detailed development information, coding standards, and architecture documentation, see [CLAUDE.md](CLAUDE.md).

### Running Tests

```bash
# Run all tests
cargo x test

# Run specific module tests
cargo x test --cpu
cargo x test --gpu
cargo x test --memory
```

### Code Formatting

```bash
# Format code (required before committing)
cargo x fmt

# Check formatting
cargo x check
```

### Code Coverage

This project uses [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) to measure test coverage.

```bash
# Install required tools
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Generate HTML report (view in browser)
cargo llvm-cov --all-features --workspace --html
open target/llvm-cov/html/index.html  # macOS
# xdg-open target/llvm-cov/html/index.html  # Linux
# start target/llvm-cov/html/index.html  # Windows
```

GitHub Actions automatically measures coverage and uploads reports as artifacts. View them in the [Actions tab](../../actions).

**Target coverage: 70%+**

## Project Status

Currently in Phase 2 development. See [docs/05-development/roadmap.md](docs/05-development/roadmap.md) for the full roadmap.

**Implemented**:
- MIPS R3000A CPU core with full instruction set
- Memory bus with region mapping
- GPU core structure with VRAM management
- CPU tracing and debugging tools

**In Progress**:
- GPU drawing commands
- BIOS integration and debugging

**Planned**:
- DMA controller
- Timer/Counter peripherals
- CD-ROM controller
- Sound Processing Unit (SPU)
- Game loading and execution

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Legal Notice

BIOS files are copyrighted by Sony Computer Entertainment Inc. and are not included with this emulator. Users must provide their own legally obtained BIOS files.
