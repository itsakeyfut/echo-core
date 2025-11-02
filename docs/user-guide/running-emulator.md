# Running the PSRX Emulator

This guide explains how to run the PSRX PlayStation emulator with a BIOS file.

## Prerequisites

### Required Files

1. **PSX BIOS File**: You must own a PlayStation console to legally use its BIOS
   - Common BIOS files: `SCPH1001.BIN`, `SCPH7502.BIN`, `SCPH5501.BIN`
   - Size: Must be exactly 512 KB (524,288 bytes)
   - Location: Can be placed anywhere, path will be specified as command-line argument

### Building the Emulator

```bash
# Build release version (optimized)
cargo build --release

# Or use the xtask build system
cargo x build --release
```

## Running with UI

### Basic Usage

```bash
# Run with BIOS file
./target/release/psrx-ui path/to/SCPH1001.BIN

# Or using cargo run
cargo run --release --bin psrx-ui -- path/to/SCPH1001.BIN
```

### Example

```bash
# If BIOS is in current directory
./target/release/psrx-ui SCPH1001.BIN

# If BIOS is in a different location
./target/release/psrx-ui ~/games/bios/SCPH1001.BIN
```

## UI Features

### Display

- **Main Window**: Shows the PlayStation GPU framebuffer (1024×768 pixels)
- **Status Bar**: Shows FPS counter, performance metrics, and running state
- **Debug Overlay**: Can be toggled to show CPU and GPU status (future feature)

### Performance Metrics

The status bar displays:
- **FPS**: Current frames per second (target: 60 FPS)
- **Frame Time**: Time to render each frame in milliseconds
- **Running State**: Whether emulation is running or stopped

### Controls

#### Window Controls
- **Close Window**: Quit the emulator (use window close button)

#### Future Keyboard Controls (Planned)
- `Q`: Quit emulator
- `R`: Reset emulator
- `P`: Pause/Resume emulation
- `D`: Toggle debug overlay

*Note: Keyboard shortcuts are not yet implemented in the current version*

## What to Expect

### BIOS Boot Sequence

When you run the emulator with a valid BIOS:

1. **Initial Boot** (0-2 seconds)
   - BIOS loads from ROM
   - CPU starts executing at reset vector (0xBFC00000)
   - Hardware initialization begins

2. **PSX Logo Display** (~3-5 seconds)
   - White "PlayStation" logo appears on black background
   - Logo may animate or pulsate
   - GPU is rendering graphics to VRAM

3. **BIOS Menu** (after logo)
   - Memory card manager
   - CD player interface
   - Console information

### Performance

- **Target FPS**: 60 FPS
- **Target Frame Time**: ~16.67ms per frame
- **CPU Speed**: Emulated at 33.8688 MHz
- **Cycles per Frame**: ~564,480 CPU cycles

### Visual Output

The emulator displays:
- 1024×512 VRAM content (scaled to fit window)
- Display area configured by BIOS (usually 320×240 or 640×480)
- RGB color output (5-5-5 format converted to RGB888)

## Troubleshooting

### BIOS File Not Found

**Error**: `Failed to load BIOS: No such file or directory`

**Solution**:
- Check the BIOS file path is correct
- Use absolute path or ensure file is in current directory
- Verify file name matches exactly (Linux is case-sensitive)

### Invalid BIOS Size

**Error**: `Invalid BIOS size: X bytes`

**Solution**:
- BIOS must be exactly 512 KB (524,288 bytes)
- Re-dump BIOS from your PlayStation console
- Ensure file is not compressed or modified

### Black Screen

**Symptom**: Window opens but displays only black screen

**Possible Causes**:
1. BIOS is still initializing (wait a few seconds)
2. GPU display is disabled (check GPU status)
3. Display area is configured incorrectly

**Debug**:
```bash
# Run with debug logging
RUST_LOG=debug cargo run --release --bin psrx-ui -- SCPH1001.BIN
```

### Low FPS

**Symptom**: FPS counter shows less than 60 FPS

**Possible Causes**:
1. Running in debug mode (use `--release` flag)
2. System is under load
3. GPU rendering is slow

**Solutions**:
- Always use release builds for performance
- Close other applications
- Check system resource usage

### Emulation Errors

**Symptom**: Emulator crashes or shows errors during execution

**Debug Steps**:
1. Enable debug logging:
   ```bash
   RUST_LOG=debug ./target/release/psrx-ui SCPH1001.BIN
   ```

2. Check for unimplemented CPU instructions
3. Verify BIOS file integrity
4. Report issue with debug log output

## Advanced Usage

### Environment Variables

```bash
# Set log level
export RUST_LOG=info    # Default
export RUST_LOG=debug   # Verbose output
export RUST_LOG=warn    # Warnings only

# Run with specific log level
RUST_LOG=debug ./target/release/psrx-ui SCPH1001.BIN
```

### Performance Logging

The emulator logs performance statistics every 5 seconds:

```
[INFO] Performance: avg 14.23ms/frame (70.3 fps)
```

This shows:
- Average frame time in milliseconds
- Average FPS over the last 5 seconds

### Debug Mode (Future)

Debug mode will display additional information:
- **CPU PC**: Current program counter
- **GPU Status**: GPU status register value
- **Cycle Count**: Total CPU cycles executed
- **Frame Time**: Per-frame rendering time

## Integration Testing

### Running BIOS Boot Tests

The emulator includes integration tests that verify BIOS boot functionality:

```bash
# Run all BIOS boot tests (requires BIOS file)
cargo test --test bios_boot -- --ignored --nocapture

# Run specific test
cargo test --test bios_boot test_bios_boot -- --ignored --nocapture

# Set custom BIOS path for tests
PSX_BIOS_PATH=/path/to/bios.bin cargo test --test bios_boot -- --ignored
```

### Available Tests

1. **test_bios_boot**: Runs BIOS for 1 second (60 frames)
2. **test_logo_display**: Runs for 5 seconds, verifies graphics output
3. **test_bios_stability**: Runs for 30 seconds, checks for crashes
4. **test_gpu_status_during_boot**: Verifies GPU status flags

## Legal Notice

**IMPORTANT**: You must legally own a PlayStation console to use its BIOS ROM.

BIOS files are copyrighted by Sony Computer Entertainment and cannot be distributed.
This emulator does not include any BIOS files.

For information on dumping your own BIOS, consult PlayStation homebrew documentation.

## Next Steps

- **Phase 3**: Peripheral implementation (controllers, memory cards)
- **Game Loading**: Support for loading and running game ISOs/BINs
- **Audio**: SPU implementation for sound output
- **Save States**: Save and restore emulation state

## Support

For issues, questions, or feature requests:
- GitHub Issues: https://github.com/itsakeyfut/psrx/issues
- Documentation: See `docs/` directory
- Specs: See `specs/` directory for technical details
