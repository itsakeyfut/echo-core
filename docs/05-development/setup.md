# Development Environment Setup Guide

## Overview

This document describes the environment setup required for PSX emulator development.

## Prerequisites

### Supported OS
- **Windows 10/11** (64-bit)
- **macOS 12+** (Intel/Apple Silicon)
- **Linux** (Ubuntu 22.04+, Fedora 38+, Arch Linux)

### Minimum System Requirements
- **CPU**: Intel Core i5 8th gen / AMD Ryzen 3000 series or higher
- **RAM**: 8GB or more (16GB recommended)
- **Storage**: 5GB or more free space
- **GPU**: OpenGL 3.3 compatible / Vulkan 1.1 compatible

## Step 1: Installing Rust

### Windows

1. **Install Rustup**
   - Download [rustup-init.exe](https://rustup.rs/)
   - Run and follow instructions
   - Visual Studio C++ Build Tools installation recommended (automatic prompt)

2. **Verify Installation**
   ```powershell
   rustc --version
   cargo --version
   ```
   Sample output:
   ```
   rustc 1.75.0 (82e1608df 2023-12-21)
   cargo 1.75.0 (1d8b05cdd 2023-11-20)
   ```

3. **Verify Visual Studio Build Tools**
   - If not automatically installed:
   - Download [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - Install "Desktop development with C++"

### macOS

1. **Install Xcode Command Line Tools**
   ```bash
   xcode-select --install
   ```

2. **Install Rustup**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   - Default settings are OK (press Enter)

3. **Set Path**
   ```bash
   source "$HOME/.cargo/env"
   ```

4. **Verify Installation**
   ```bash
   rustc --version
   cargo --version
   ```

### Linux (Ubuntu/Debian)

1. **Install Dependencies**
   ```bash
   sudo apt update
   sudo apt install -y build-essential curl git pkg-config libssl-dev

   # Audio development libraries
   sudo apt install -y libasound2-dev

   # Linux window system (Wayland/X11)
   sudo apt install -y libwayland-dev libxkbcommon-dev
   ```

2. **Install Rustup**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

3. **Verify Installation**
   ```bash
   rustc --version
   cargo --version
   ```

### Linux (Fedora)

1. **Install Dependencies**
   ```bash
   sudo dnf install -y gcc gcc-c++ make git openssl-devel
   sudo dnf install -y alsa-lib-devel
   sudo dnf install -y wayland-devel libxkbcommon-devel
   ```

2. **Install Rustup**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

### Linux (Arch Linux)

1. **Install Dependencies**
   ```bash
   sudo pacman -S base-devel git openssl alsa-lib wayland libxkbcommon
   ```

2. **Install Rustup**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

---

## Step 2: Rust Toolchain Configuration

### Update to Latest Stable

```bash
rustup update stable
rustup default stable
```

### Install Components

```bash
# Components for rust-analyzer
rustup component add rust-src

# Formatter
rustup component add rustfmt

# Linter
rustup component add clippy
```

### Add Targets (for cross-compilation, optional)

```bash
# For Windows (from macOS/Linux)
rustup target add x86_64-pc-windows-gnu

# For macOS (from Linux/Windows)
rustup target add x86_64-apple-darwin

# For Linux (from Windows/macOS)
rustup target add x86_64-unknown-linux-gnu
```

---

## Step 3: Installing Development Tools

### Required Tools

#### 1. Git

**Windows:**
- Download and install [Git for Windows](https://git-scm.com/download/win)

**macOS:**
```bash
# If Homebrew is installed
brew install git

# Or included in Xcode Command Line Tools
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install git

# Fedora
sudo dnf install git

# Arch
sudo pacman -S git
```

#### 2. VSCode (Recommended Editor)

- Download and install [Visual Studio Code](https://code.visualstudio.com/)

**Required Extensions:**
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb"
  ]
}
```

**Installation Method:**
1. Open VSCode
2. Extensions tab (Ctrl+Shift+X / Cmd+Shift+X)
3. Search and install each extension

**Recommended Settings (`.vscode/settings.json`):**
```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### Recommended Tools

#### cargo-watch (Auto-build)

```bash
cargo install cargo-watch
```

**Usage:**
```bash
# Watch files and auto-check
cargo watch -x check

# Check + test
cargo watch -x check -x test

# Check + run
cargo watch -x check -x run
```

#### cargo-nextest (Fast Test Execution)

```bash
cargo install cargo-nextest
```

**Usage:**
```bash
# Run tests normally
cargo nextest run

# Re-run only failed tests
cargo nextest run --failed
```

#### cargo-criterion (Benchmarking)

```bash
cargo install cargo-criterion
```

**Usage:**
```bash
cargo criterion
```

#### cargo-flamegraph (Profiling)

**Linux:**
```bash
# Install perf (if needed)
sudo apt install linux-tools-common linux-tools-generic

cargo install flamegraph
```

**macOS:**
```bash
# Use DTrace (system standard)
cargo install cargo-instruments
```

**Usage:**
```bash
# Generate flamegraph
cargo flamegraph --bin psx-emulator
```

#### sccache (Compilation Cache, optional)

```bash
cargo install sccache

# Add to environment variables (~/.bashrc or ~/.zshrc)
export RUSTC_WRAPPER=sccache
```

---

## Step 4: Project Setup

### Clone Repository

```bash
git clone https://github.com/YOUR_USERNAME/psx-emulator.git
cd psx-emulator
```

### Build Dependencies

```bash
# Initial build (takes time)
cargo build

# Release build
cargo build --release
```

### Using the Task Automation Tool (xtask)

This project includes a custom task automation tool (`cargo x`) that simplifies common development tasks.

#### Available Commands

```bash
# Show all available commands
cargo x --help

# Run all CI checks (fmt, clippy, build, test)
cargo x ci

# Quick checks before commit (fmt, clippy)
cargo x check

# Format code
cargo x fmt

# Run clippy
cargo x clippy

# Build the project
cargo x build --release

# Run tests
cargo x test

# Run BIOS boot test
cargo x bios-boot --release

# Install git pre-commit hooks
cargo x install-hooks
```

#### BIOS Boot Test

The `bios-boot` command runs the emulator with a BIOS file for testing:

```bash
# Run BIOS boot test with default settings (SCPH1001.BIN, 100k instructions)
cargo x bios-boot --release

# Specify custom BIOS file
cargo x bios-boot path/to/BIOS.BIN

# Specify number of instructions
cargo x bios-boot -n 200000 --release

# Run in debug mode (slower)
cargo x bios-boot
```

**Requirements:**
- BIOS file (e.g., `SCPH1001.BIN`) must be in the project root
- BIOS file must be exactly 512KB (524,288 bytes)

**Expected Output:**
```
=== BIOS Boot Test ===
✓ BIOS file: SCPH1001.BIN
→ Instructions: 100000
→ Build mode: release

→ Building in release mode...
[Build output...]

[Emulator output showing progress through BIOS execution]

✓ BIOS boot test completed in 0.10s
```

#### Installing Git Hooks

Install pre-commit hooks to automatically run checks before committing:

```bash
cargo x install-hooks
```

This will run format checks, clippy, and tests before each commit.

### Manual Commands (Alternative to xtask)

If you prefer to run commands manually:

```bash
# Run tests
cargo test

# Or use nextest
cargo nextest run

# Run lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Check formatting
cargo fmt -- --check
```

---

## Step 5: Preparing BIOS Files

### Importance of BIOS

PSX emulator **requires** a BIOS file. The BIOS must be dumped from actual hardware.

### BIOS File Placement

1. **Create Directory**
   ```bash
   mkdir -p ~/.psx-emulator/bios
   ```

2. **Place BIOS File**
   - Place BIOS file dumped from actual hardware (e.g., `SCPH1001.BIN`)
   - Path: `~/.psx-emulator/bios/SCPH1001.BIN`

3. **Verify**
   ```bash
   # Check file size (512KB = 524,288 bytes)
   ls -lh ~/.psx-emulator/bios/SCPH1001.BIN
   ```

### Supported BIOS Versions

| Filename | Region | Version |
|----------|--------|---------|
| SCPH1000.BIN | Japan | 1.0 |
| SCPH1001.BIN | North America | 2.2 |
| SCPH1002.BIN | Europe | 2.2 |
| SCPH5500.BIN | Japan | 3.0 |
| SCPH5501.BIN | North America | 3.0 |
| SCPH5502.BIN | Europe | 3.0 |
| SCPH7003.BIN | Japan | 4.1 |

**Recommended:** `SCPH5501.BIN` (North America v3.0) - Most compatible

### Open Source BIOS (Experimental)

**OpenBIOS for PSX** (under development)
- Low compatibility with commercial games
- Recommended for testing/development only
- https://github.com/pcsx-redux/nugget

---

## Step 6: Preparing Development Resources

### Test ROMs

#### CPU Instruction Test ROM

```bash
mkdir -p tests/roms
cd tests/roms

# amidog's PSX tests (requires: build yourself)
git clone https://github.com/amidog/mips_tests.git
```

#### PSX Demo Scene

- Download freely distributed demo ROMs
- Example: [Pouet.net PSX Demos](https://www.pouet.net/prodlist.php?platform%5B%5D=Playstation)

### Game ISOs (For Testing)

- **Use only legitimate game discs you own**
- ISO creation method omitted (legal reasons)

---

## Step 7: Initial Build and Run

### Debug Build

```bash
cd psx-emulator

# Build
cargo build

# Run
cargo run
```

### Release Build

```bash
# Optimized build
cargo build --release

# Run
./target/release/psx-emulator
```

### Build Time Estimates

| Build Type | Initial | Subsequent |
|-----------|---------|-----------|
| Debug | 5-10 min | 30 sec-2 min |
| Release | 10-20 min | 1-3 min |

**Note:** Can be significantly reduced using sccache

---

## Step 8: Development in VSCode

### Open Project

```bash
code .
```

### Debug Configuration (`.vscode/launch.json`)

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug PSX Emulator",
      "cargo": {
        "args": [
          "build",
          "--bin=psx-emulator",
          "--package=psx-emulator"
        ],
        "filter": {
          "name": "psx-emulator",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Unit Tests",
      "cargo": {
        "args": [
          "test",
          "--no-run",
          "--lib",
          "--package=psx-emulator"
        ],
        "filter": {
          "name": "psx-emulator",
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Task Configuration (`.vscode/tasks.json`)

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "cargo check",
      "type": "shell",
      "command": "cargo",
      "args": ["check"],
      "group": "build",
      "presentation": {
        "reveal": "always",
        "panel": "new"
      },
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo build",
      "type": "shell",
      "command": "cargo",
      "args": ["build"],
      "group": {
        "kind": "build",
        "isDefault": true
      },
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo test",
      "type": "shell",
      "command": "cargo",
      "args": ["test"],
      "group": {
        "kind": "test",
        "isDefault": true
      },
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo clippy",
      "type": "shell",
      "command": "cargo",
      "args": ["clippy", "--", "-D", "warnings"],
      "group": "build",
      "problemMatcher": ["$rustc"]
    }
  ]
}
```

---

## Troubleshooting

### Windows

#### Issue: Linker error (link.exe not found)

**Solution:**
```powershell
# Reinstall Visual Studio Build Tools
# Select "Desktop development with C++"
```

#### Issue: OpenSSL-related errors

**Solution:**
```powershell
# Use vcpkg
git clone https://github.com/Microsoft/vcpkg.git
cd vcpkg
./bootstrap-vcpkg.bat
./vcpkg install openssl:x64-windows

# Set environment variable
set OPENSSL_DIR=C:\path\to\vcpkg\installed\x64-windows
```

### macOS

#### Issue: xcrun: error: invalid active developer path

**Solution:**
```bash
xcode-select --install
```

#### Issue: Linker error (on Apple Silicon)

**Solution:**
```bash
# If using Rosetta
arch -x86_64 cargo build

# Or configure for native build
rustup target add aarch64-apple-darwin
```

### Linux

#### Issue: ALSA-related errors

**Solution:**
```bash
# Ubuntu/Debian
sudo apt install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel

# Arch
sudo pacman -S alsa-lib
```

#### Issue: Wayland/X11-related errors

**Solution:**
```bash
# Ubuntu/Debian
sudo apt install libwayland-dev libxkbcommon-dev

# Fedora
sudo dnf install wayland-devel libxkbcommon-devel

# Arch
sudo pacman -S wayland libxkbcommon
```

### Common Issues

#### Issue: Slow compilation

**Solution 1: Use sccache**
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```

**Solution 2: Change linker**

Create `.cargo/config.toml`:
```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-pc-windows-msvc]
linker = "lld-link"

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/bin/ld"]
```

#### Issue: Out of memory

**Solution:**
```bash
# Limit parallel compilation
export CARGO_BUILD_JOBS=2

# Or specify in build command
cargo build -j 2
```

---

## Development Workflow

### Daily Development Flow (Recommended)

Using the xtask automation tool:

```bash
# 1. Create branch
git checkout -b feature/cpu-implementation

# 2. Install pre-commit hooks (one-time setup)
cargo x install-hooks

# 3. Auto-check with cargo-watch (optional)
cargo watch -x check -x test

# 4. Edit code (VSCode, etc.)

# 5. Run quick checks
cargo x check

# 6. Run all tests including BIOS boot
cargo x test
cargo x bios-boot --release

# 7. Commit (pre-commit hooks will run automatically)
git add .
git commit -m "feat: implement ADD instruction"

# 8. Push
git push origin feature/cpu-implementation
```

### Alternative: Manual Workflow

```bash
# 1. Create branch
git checkout -b feature/cpu-implementation

# 2. Auto-check with cargo-watch
cargo watch -x check -x test

# 3. Edit code (VSCode, etc.)

# 4. Run tests
cargo test

# 5. Run BIOS boot test
cargo run --release -- SCPH1001.BIN

# 6. Lint
cargo clippy -- -D warnings

# 7. Format
cargo fmt

# 8. Commit
git add .
git commit -m "feat: implement ADD instruction"

# 9. Push
git push origin feature/cpu-implementation
```

### Pre-Pull Request Checklist

- [ ] All `cargo test` pass (or `cargo x test`)
- [ ] No errors from `cargo clippy -- -D warnings` (or `cargo x clippy`)
- [ ] No errors from `cargo fmt -- --check` (or `cargo x fmt --check`)
- [ ] BIOS boot test passes (`cargo x bios-boot --release`)
- [ ] Added documentation comments
- [ ] Updated `CHANGELOG.md` with changes (future)

---

## Next Steps

Once environment setup is complete, read the following documents to start development:

1. [Coding Standards](../02-implementation/coding-standards.md)
2. [Development Roadmap](./roadmap.md)
3. [CPU Design](../01-design/cpu-design.md)
4. [Testing Strategy](../02-implementation/testing-strategy.md)

---

## Support

### If Problems Persist

1. **Check Documentation**
   - [Rust Official Documentation](https://doc.rust-lang.org/)
   - [Cargo Book](https://doc.rust-lang.org/cargo/)

2. **Search**
   - Google search error messages
   - Stack Overflow

3. **Community**
   - Rust Users Forum
   - Rust Discord

---

## Revision History

- 2025-10-28: Initial version
