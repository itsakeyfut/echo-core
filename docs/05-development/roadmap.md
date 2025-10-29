# Development Roadmap

## Overview

This document presents a detailed roadmap for PSX emulator development. Each phase has clear goals and deliverables, with functionality being added progressively.

## Overall Schedule

```
Week 0:    Project Setup
Week 1-4:  Phase 1 - Foundation
Week 5-8:  Phase 2 - Graphics
Week 9-12: Phase 3 - Peripherals
Week 13-16: Phase 4 - Audio & Completeness
Week 17-20: Phase 5 - Optimization & Release Preparation
```

**Estimated Total Development Period: 5 months (20 weeks)**

## Week 0: Project Setup

### Goals
- Build development environment
- Create project skeleton
- Prepare documentation

### Tasks

#### Environment Setup
- [ ] Install latest stable Rust
- [ ] Setup VSCode + rust-analyzer
- [ ] Initialize Git repository
- [ ] Create `.gitignore`
- [ ] Setup CI/CD pipeline (GitHub Actions)

#### Project Initialization
```bash
cargo new --lib psx-emulator
cd psx-emulator

# Add required crates
cargo add slint wgpu bytemuck cpal serde thiserror log env_logger bitflags

# Development crates
cargo add --dev criterion proptest
```

#### Directory Structure Creation
```
src/
├── lib.rs
├── core/
│   ├── mod.rs
│   ├── cpu/
│   │   └── mod.rs
│   ├── memory/
│   │   └── mod.rs
│   └── system.rs
├── frontend/
│   └── mod.rs
└── util/
    └── mod.rs
```

#### Documentation Creation
- [x] `docs/00-overview/` all files
- [x] `docs/01-design/cpu-design.md`
- [ ] `docs/03-hardware-specs/memory-map.md`
- [ ] `docs/05-development/setup.md`

#### Deliverables
- Runnable project skeleton
- Passing CI/CD
- Basic documentation complete

---

## Phase 1: Foundation (Week 1-4)

### Goals
**BIOS boot and menu display**

### Week 1: CPU Basic Structure + Memory System

#### Tasks

**CPU Core:**
- [ ] Define `CPU` struct
  ```rust
  pub struct CPU {
      regs: [u32; 32],
      pc: u32,
      next_pc: u32,
      hi: u32,
      lo: u32,
      cop0: COP0,
  }
  ```
- [ ] Register access (`reg()`, `set_reg()`)
- [ ] Instruction fetch functionality
- [ ] Basic instruction decoder skeleton

**Memory System:**
- [ ] Implement `Bus` struct
  ```rust
  pub struct Bus {
      ram: Vec<u8>,
      bios: Vec<u8>,
      scratchpad: [u8; 1024],
  }
  ```
- [ ] Implement memory map
- [ ] `read8/16/32`, `write8/16/32` methods
- [ ] Address mirroring handling
- [ ] BIOS file loading

**Tests:**
- [ ] Memory read/write tests
- [ ] BIOS loading test

#### Deliverables
- Able to load BIOS into memory

---

### Week 2: Basic Instruction Implementation (Arithmetic, Logic, Load/Store)

#### Tasks

**Arithmetic Instructions (High Priority):**
- [ ] `ADDI`, `ADDIU` - Immediate addition
- [ ] `ADD`, `ADDU` - Register addition
- [ ] `SUB`, `SUBU` - Subtraction
- [ ] `SLT`, `SLTI` - Comparison

**Logical Instructions:**
- [ ] `ANDI` - Logical AND (immediate)
- [ ] `ORI` - Logical OR (immediate)
- [ ] `XORI` - Logical XOR (immediate)
- [ ] `LUI` - Load upper 16 bits
- [ ] `AND`, `OR`, `XOR`, `NOR` - Register logical operations

**Shift Instructions:**
- [ ] `SLL` - Logical left shift
- [ ] `SRL` - Logical right shift
- [ ] `SRA` - Arithmetic right shift
- [ ] `SLLV`, `SRLV`, `SRAV` - Variable shift

**Load/Store:**
- [ ] `LW` - Load word
- [ ] `SW` - Store word
- [ ] `LB`, `LBU` - Load byte
- [ ] `SB` - Store byte
- [ ] `LH`, `LHU` - Load halfword
- [ ] `SH` - Store halfword

**Load Delay Slot:**
- [ ] `LoadDelay` struct
- [ ] `set_reg_delayed()` method

**Tests:**
- [ ] Unit tests for each instruction
- [ ] Load delay slot tests

#### Deliverables
- Basic instructions working

---

### Week 3: Branch/Jump Instructions + Exception Handling

#### Tasks

**Branch Instructions:**
- [ ] `BEQ` - Branch if equal
- [ ] `BNE` - Branch if not equal
- [ ] `BLEZ` - Branch if less than or equal to zero
- [ ] `BGTZ` - Branch if greater than zero
- [ ] `BLTZ`, `BGEZ` - Conditional branches
- [ ] `BLTZAL`, `BGEZAL` - Branch and link

**Jump Instructions:**
- [ ] `J` - Jump
- [ ] `JAL` - Jump and link
- [ ] `JR` - Register jump
- [ ] `JALR` - Register jump and link

**Branch Delay Slot:**
- [ ] Delay slot processing implementation
- [ ] `branch()` method

**Exception Handling:**
- [ ] `ExceptionCause` enum
- [ ] `exception()` method
- [ ] COP0 register implementation (SR, CAUSE, EPC)
- [ ] Jump to exception handler

**COP0 Instructions:**
- [ ] `MFC0` - Load from COP0
- [ ] `MTC0` - Store to COP0
- [ ] `RFE` - Return from exception

**Tests:**
- [ ] Branch instruction tests
- [ ] Delay slot tests
- [ ] Exception handling tests

#### Deliverables
- Control flow instructions working

---

### Week 4: Multiplication/Division Instructions + BIOS Boot Test

#### Tasks

**Multiplication/Division Instructions:**
- [ ] `MULT` - Signed multiplication
- [ ] `MULTU` - Unsigned multiplication
- [ ] `DIV` - Signed division
- [ ] `DIVU` - Unsigned division
- [ ] `MFHI`, `MFLO` - Read HI/LO
- [ ] `MTHI`, `MTLO` - Write HI/LO

**Other Instructions:**
- [ ] `SYSCALL` - System call
- [ ] `BREAK` - Breakpoint

**System Integration:**
- [ ] Create `System` struct
  ```rust
  pub struct System {
      cpu: CPU,
      bus: Bus,
      cycles: u64,
  }
  ```
- [ ] `System::step()` - Execute one instruction
- [ ] `System::run_frame()` - Execute one frame

**BIOS Boot Test:**
- [ ] Execute BIOS boot sequence
- [ ] Trace processing until PSX logo display
- [ ] Prepare log output

**Debug Tools:**
- [ ] Disassembler (instruction to string)
- [ ] Register dump functionality
- [ ] Memory dump functionality

**Tests:**
- [ ] Multiplication/division instruction tests
- [ ] BIOS boot test (verify with actual BIOS)

#### Deliverables
- BIOS boots and reaches PSX logo display processing

**Phase 1 Milestone Achievement Criteria:**
- [ ] 90%+ CPU instruction set implemented
- [ ] BIOS menu displayed (not visible on screen as GPU not implemented yet, but processing proceeds)
- [ ] Memory system working normally
- [ ] All unit tests passing

---

## Phase 2: Graphics Implementation (Week 5-8)

### Goals
**Display PSX logo and menu on screen**

### Week 5: GPU Basic Structure + VRAM Management

#### Tasks

**GPU Core:**
- [ ] Define `GPU` struct
  ```rust
  pub struct GPU {
      vram: Vec<u16>,  // 1024x512, 16bit/pixel
      draw_mode: DrawMode,
      draw_area: DrawingArea,
      texture_window: TextureWindow,
  }
  ```
- [ ] VRAM initialization (1MB = 512K pixels)
- [ ] GP0/GP1 register memory map implementation

**GP1 Commands (Control):**
- [ ] `0x00` - GPU reset
- [ ] `0x01` - Command buffer reset
- [ ] `0x02` - Acknowledge interrupt
- [ ] `0x03` - Display enable/disable
- [ ] `0x04` - DMA mode setting
- [ ] `0x05` - Display area start position
- [ ] `0x06` - Horizontal display range
- [ ] `0x07` - Vertical display range
- [ ] `0x08` - Display mode setting

**GP0 Commands (Drawing) Foundation:**
- [ ] Command FIFO implementation
- [ ] Command parser skeleton

**VRAM Transfer:**
- [ ] `0xA0` - CPU → VRAM transfer
- [ ] `0xC0` - VRAM → CPU transfer

**Bus Integration:**
- [ ] GPU register memory mapping (0x1F801810 etc.)
- [ ] GPUREAD/GPUSTAT registers

**Tests:**
- [ ] VRAM read/write tests
- [ ] Command reception tests

#### Deliverables
- GPU basic structure working

---

### Week 6: Drawing Primitives Implementation

#### Tasks

**Solid Color Polygon Drawing:**
- [ ] `0x28` - Solid opaque quad
- [ ] `0x2C` - Solid opaque textured quad
- [ ] `0x30` - Solid semi-transparent triangle
- [ ] `0x38` - Solid semi-transparent quad

**Gradient Polygons:**
- [ ] `0x34` - Gradient opaque triangle
- [ ] `0x3C` - Gradient opaque quad

**Line Drawing:**
- [ ] `0x40` - Solid line
- [ ] `0x48` - Continuous line (polyline)

**Rasterizer Implementation:**
- [ ] Scanline-based triangle drawing
- [ ] Quad → 2 triangles split
- [ ] Edge equation for inside/outside determination

**Framebuffer Generation:**
- [ ] VRAM → RGB24 format conversion
- [ ] Display area extraction
- [ ] `get_framebuffer()` method

**Tests:**
- [ ] Each primitive drawing test
- [ ] Rasterizer tests

#### Deliverables
- Can draw simple shapes

---

### Week 7: Texture Mapping + Blending

#### Tasks

**Texture Mapping:**
- [ ] Texture coordinate processing
- [ ] CLUT (Color Lookup Table) implementation
- [ ] 4bit, 8bit, 15bit texture support
- [ ] Texture window processing

**Textured Primitives:**
- [ ] `0x24` - Textured opaque triangle
- [ ] `0x2C` - Textured opaque quad
- [ ] `0x26` - Textured semi-transparent triangle
- [ ] `0x2E` - Textured semi-transparent quad

**Semi-transparent Blending:**
- [ ] Blending modes (B/2 + F/2, B + F, B - F, B + F/4)
- [ ] `DrawMode` semi-transparent flag processing

**Drawing Settings Commands:**
- [ ] `0xE1` - Drawing mode setting
- [ ] `0xE2` - Texture window setting
- [ ] `0xE3` - Drawing area top-left setting
- [ ] `0xE4` - Drawing area bottom-right setting
- [ ] `0xE5` - Drawing offset setting
- [ ] `0xE6` - Mask setting

**Tests:**
- [ ] Texture mapping tests
- [ ] Blending tests

#### Deliverables
- Can draw textured polygons

---

### Week 8: Frontend Integration + Screen Display

#### Tasks

**Slint UI Foundation:**
- [ ] Slint project setup
- [ ] Basic window creation
  ```slint
  export component MainWindow inherits Window {
      title: "PSX Emulator";
      width: 640px;
      height: 480px;

      Image {
          source: @image-url("framebuffer");
          width: 100%;
          height: 100%;
      }
  }
  ```

**Screen Display:**
- [ ] Pass GPU framebuffer to Slint
- [ ] 60fps display loop
- [ ] Window resize support

**wgpu Integration (Optional):**
- [ ] wgpu backend initialization
- [ ] Texture upload
- [ ] Simple shader (display pixels as-is)

**Input Handling:**
- [ ] Accept keyboard input (don't pass to CPU yet)

**Debug Display:**
- [ ] FPS counter
- [ ] CPU usage display

**BIOS Boot Confirmation:**
- [ ] PSX logo displayed on screen
- [ ] BIOS menu displayed

**Tests:**
- [ ] Screen display test
- [ ] Frame rate measurement

#### Deliverables
- BIOS menu displayed on screen

**Phase 2 Milestone Achievement Criteria:**
- [ ] PSX logo displayed correctly
- [ ] BIOS menu operable (input not yet)
- [ ] Stable 60fps
- [ ] 70% GPU drawing commands implemented

---

## Phase 3: Peripheral Implementation (Week 9-12)

### Goals
**Game boot and playable state**

### Week 9: Controller Input + Timers

#### Tasks

**Controller:**
- [ ] Implement `Controller` struct
  ```rust
  pub struct Controller {
      buttons: u16,  // Bit field
      state: ControllerState,
  }
  ```
- [ ] Digital pad button mapping
- [ ] Emulate controller port (0x1F801040-0x1F80104F)
- [ ] Serial communication protocol

**Input Mapping:**
- [ ] Keyboard → PSX button mapping
  ```
  D-Pad: WASD or Arrow Keys
  ○×△□: IJKL or ZXCV
  L1/R1: Q/E
  L2/R2: 1/3
  Start/Select: Enter/Shift
  ```

**Timers:**
- [ ] `Timers` struct (3 channels)
  ```rust
  pub struct Timer {
      counter: u16,
      mode: u16,
      target: u16,
  }
  ```
- [ ] Timer registers (0x1F801100-0x1F80112F)
- [ ] Count modes (system clock, horizontal sync, dot)
- [ ] Interrupt generation

**Interrupt Controller:**
- [ ] `InterruptController` struct
- [ ] I_STAT, I_MASK registers
- [ ] Interrupt source management
- [ ] CPU interrupt notification

**VBlank/HBlank:**
- [ ] VBlank interrupt generation (every 1/60 second)
- [ ] HBlank interrupt (optional)

**Tests:**
- [ ] Controller input tests
- [ ] Timer operation tests
- [ ] Interrupt generation tests

#### Deliverables
- BIOS menu operable with controller

---

### Week 10: CD-ROM Basic Functionality

#### Tasks

**CD-ROM Structure:**
- [ ] Implement `CDROM` struct
  ```rust
  pub struct CDROM {
      disc: Option<Disc>,
      position: CDPosition,
      status: CDStatus,
      interrupt_flag: u8,
  }
  ```
- [ ] CD-ROM registers (0x1F801800-0x1F801803)

**Disc Image:**
- [ ] `.bin/.cue` file loading
- [ ] Basic ISO9660 filesystem support
- [ ] Sector reading (2352 bytes/sector)

**CD-ROM Commands:**
- [ ] `0x01` - GetStat
- [ ] `0x02` - SetLoc (set seek position)
- [ ] `0x06` - ReadN (data read)
- [ ] `0x0A` - Init (initialize)
- [ ] `0x15` - SeekL (seek)
- [ ] `0x1A` - GetID (get disc ID)
- [ ] `0x1E` - ReadTOC (read TOC)

**Seek Processing:**
- [ ] Simulate seek time
- [ ] Seek complete interrupt

**Data Transfer:**
- [ ] Sector buffer
- [ ] DMA transfer (implement in later week)

**Tests:**
- [ ] Disc reading tests
- [ ] Seek tests

#### Deliverables
- Can read data from CD-ROM

---

### Week 11: DMA + Game Boot

#### Tasks

**DMA Controller:**
- [ ] `DMA` struct (7 channels)
  ```rust
  pub struct DMAChannel {
      base_address: u32,
      block_control: u32,
      channel_control: u32,
  }
  ```
- [ ] DMA registers (0x1F801080-0x1F8010FF)
- [ ] Transfer modes (immediate, sync, linked list)

**Each Channel Implementation:**
- [ ] Ch2: GPU (drawing list transfer)
- [ ] Ch3: CD-ROM (data read)
- [ ] Ch4: SPU (audio data transfer)
- [ ] Ch6: OTC (ordering table, for GPU)

**DMA Transfer Processing:**
- [ ] Memory → Device transfer
- [ ] Device → Memory transfer
- [ ] Linked list method (GPU commands)
- [ ] Transfer complete interrupt

**Game Boot Sequence:**
- [ ] Load game from BIOS
- [ ] Parse `SYSTEM.CNF`
- [ ] Load executable (PSX-EXE)
- [ ] Jump to game code

**Debug:**
- [ ] Detailed game boot logs
- [ ] Memory map visualization

**Tests:**
- [ ] DMA transfer tests
- [ ] Simple game (demo) boot test

#### Deliverables
- Commercial games boot (no audio, some graphics glitches)

---

### Week 12: GPU Optimization + Compatibility Improvement

#### Tasks

**GPU Optimization:**
- [ ] Speed up drawing commands
- [ ] Optimize VRAM access
- [ ] Parallelize software renderer (using rayon)

**Unimplemented GPU Commands:**
- [ ] `0x01` - Cache clear
- [ ] `0x02` - Framebuffer rectangle fill
- [ ] `0x60-0x7F` - Sprite commands
- [ ] Other minor commands

**GTE (Geometry Transformation Engine):**
- [ ] Basic GTE commands
  - RTPS: 3D coordinate transformation
  - NCLIP: Normal clipping
  - AVSZ3/4: Average Z value calculation
- [ ] GTE registers (COP2)
- [ ] Matrix/vector operations

**Compatibility Testing:**
- [ ] Test 10 major titles
  - Final Fantasy VII
  - Metal Gear Solid
  - Gran Turismo
  - Resident Evil
  - Crash Bandicoot
  - etc.
- [ ] Fix graphics issues

**Tests:**
- [ ] GTE instruction tests
- [ ] Operation confirmation with multiple games

#### Deliverables
- Major titles boot and playable (no audio)

**Phase 3 Milestone Achievement Criteria:**
- [ ] 10+ commercial games boot
- [ ] Controller input works normally
- [ ] CD-ROM loading functional
- [ ] DMA working normally
- [ ] Basic GTE instructions working

---

## Phase 4: Audio & Completeness (Week 13-16)

### Goals
**Audio playback and save state functionality**

### Week 13: SPU Basic Implementation

#### Tasks

**SPU Structure:**
- [ ] Implement `SPU` struct
  ```rust
  pub struct SPU {
      ram: Vec<u8>,  // 512KB
      voices: [Voice; 24],
      reverb: ReverbUnit,
  }
  ```
- [ ] Audio RAM (512KB)
- [ ] SPU registers (0x1F801C00-0x1F801FFF)

**Voice:**
- [ ] ADPCM decoder
- [ ] ADSR envelope
- [ ] Pitch control
- [ ] Volume control

**Audio Output:**
- [ ] cpal audio stream initialization
- [ ] 44.1kHz stereo output
- [ ] Sample buffer management

**Basic SPU Commands:**
- [ ] Voice on/off
- [ ] Pitch setting
- [ ] Volume setting
- [ ] ADSR setting

**Tests:**
- [ ] ADPCM decode tests
- [ ] Single tone playback test

#### Deliverables
- Simple audio plays

---

### Week 14: SPU Advanced Features + CD-DA

#### Tasks

**Reverb:**
- [ ] Reverb buffer
- [ ] Reverb parameters
- [ ] Reverb effect processing

**Noise Generator:**
- [ ] Noise voice
- [ ] Pseudo-random number generation

**CD-DA Audio:**
- [ ] Audio track playback from CD-ROM
- [ ] XA-ADPCM decode
- [ ] CD-DA mixing

**Audio Sync:**
- [ ] Audio/video sync
- [ ] Buffer underrun countermeasures

**SPU DMA:**
- [ ] Ch4 (SPU) DMA transfer
- [ ] Fast audio data transfer

**Tests:**
- [ ] Reverb tests
- [ ] CD-DA playback tests
- [ ] Audio verification with multiple games

#### Deliverables
- Game audio and BGM play normally

---

### Week 15: Save State Functionality

#### Tasks

**State Structure:**
- [ ] `EmulatorState` struct
  ```rust
  pub struct EmulatorState {
      cpu: CPUState,
      gpu: GPUState,
      spu: SPUState,
      memory: MemoryState,
      timestamp: SystemTime,
  }
  ```

**Serialization:**
- [ ] State saving using serde
- [ ] Binary format (bincode)
- [ ] Compression (optional)

**Save/Load:**
- [ ] `save_state()` - Save current state
- [ ] `load_state()` - Restore state
- [ ] Quick save/load (hotkey support)
- [ ] Slot management (10 slots)

**UI Integration:**
- [ ] Save state selection screen
- [ ] Thumbnail image saving
- [ ] Save date/time display

**Compatibility:**
- [ ] Embed version information
- [ ] Consider future version compatibility

**Tests:**
- [ ] Save/load tests
- [ ] Operation confirmation with multiple games

#### Deliverables
- Can save and restore game at any point

---

### Week 16: Memory Card + Compatibility Improvement

#### Tasks

**Memory Card:**
- [ ] `MemoryCard` struct (128KB)
- [ ] `.mcr` file format
- [ ] Save/load processing
- [ ] Format processing

**Memory Card Commands:**
- [ ] Read command
- [ ] Write command
- [ ] ID command

**UI Features:**
- [ ] Memory card management screen
- [ ] Save data display
- [ ] Save data copy/delete

**Compatibility Testing:**
- [ ] Test with 50 games
- [ ] Create compatibility list
- [ ] Bug fixes

**Performance Testing:**
- [ ] Run benchmarks
- [ ] Identify bottlenecks
- [ ] Optimization

**Tests:**
- [ ] Memory card read/write tests
- [ ] Compatibility confirmation with many games

#### Deliverables
- Memory card functional
- 95% compatibility achieved

**Phase 4 Milestone Achievement Criteria:**
- [ ] Audio plays normally
- [ ] Save state functionality works
- [ ] Memory card works
- [ ] Operation confirmed with 50+ commercial games
- [ ] 90%+ compatibility

---

## Phase 5: Optimization & Release Preparation (Week 17-20)

### Goals
**v1.0 Release**

### Week 17: Performance Optimization

#### Tasks

**Profiling:**
- [ ] Identify hotspots with cargo-flamegraph
- [ ] CPU bottleneck analysis
- [ ] GPU drawing optimization point identification

**CPU Optimization:**
- [ ] Implement cached interpreter
- [ ] Optimize frequent instruction paths
- [ ] Branch prediction optimization

**GPU Optimization:**
- [ ] Implement wgpu hardware renderer
- [ ] Shader optimization
- [ ] Batch drawing

**Memory Optimization:**
- [ ] Improve memory access patterns
- [ ] Improve cache efficiency
- [ ] Reduce memory allocations

**Multithreading:**
- [ ] Parallelize GPU drawing
- [ ] Parallelize SPU processing

**Target Performance:**
- [ ] Average 60fps
- [ ] CPU usage 50% or less
- [ ] Memory usage 200MB or less

#### Deliverables
- Target performance achieved

---

### Week 18: UI/UX Completion

#### Tasks

**Main UI:**
- [ ] Game library screen
  - Grid display
  - List display
  - Thumbnail display
- [ ] Add game (drag & drop)
- [ ] Game information display

**Emulation Screen:**
- [ ] Menu bar (File, Emulation, Settings, Help)
- [ ] Toolbar (Start, Stop, Reset, Screenshot)
- [ ] Status bar (FPS, audio buffer level)

**Settings Screen:**
- [ ] Video settings
  - Internal resolution
  - Aspect ratio
  - Filters
- [ ] Audio settings
  - Volume
  - Latency
- [ ] Input settings
  - Key mapping
  - Controller settings
- [ ] Path settings
  - BIOS
  - Memory card
  - Save states

**Hotkeys:**
- [ ] F1: Quick save
- [ ] F2: Quick load
- [ ] F3: Save state screen
- [ ] F9: Screenshot
- [ ] F11: Fullscreen
- [ ] Esc: Menu

**Tests:**
- [ ] UI operation tests
- [ ] Usability tests

#### Deliverables
- User-friendly UI complete

---

### Week 19: Documentation & Testing Complete

#### Tasks

**User Documentation:**
- [ ] User guide
  - Installation method
  - Basic usage
  - Save state usage
- [ ] FAQ
- [ ] Troubleshooting
- [ ] System requirements

**Developer Documentation:**
- [ ] API documentation (rustdoc)
- [ ] Architecture document updates
- [ ] Contribution guide

**Testing Complete:**
- [ ] Unit test coverage 70%+
- [ ] Integration tests
- [ ] Compatibility tests (100 titles)

**Bug Fixes:**
- [ ] Fix known issues
- [ ] Fix crashes
- [ ] Fix graphics glitches

**CI/CD:**
- [ ] Automate release builds
- [ ] Windows/macOS/Linux builds
- [ ] Create installers

#### Deliverables
- Documentation complete
- Stable builds

---

### Week 20: Release Preparation

#### Tasks

**Final Testing:**
- [ ] Regression testing
- [ ] Performance testing
- [ ] Operation confirmation on all platforms

**Release Notes Creation:**
- [ ] Feature list
- [ ] Known issues
- [ ] Credits

**License Organization:**
- [ ] License file
- [ ] Third-party license documentation

**Distribution Preparation:**
- [ ] Create GitHub release
- [ ] Upload binaries
- [ ] Create website (optional)

**Promotion:**
- [ ] Take screenshots
- [ ] Create demo video
- [ ] SNS announcement

**Community:**
- [ ] Create Discord server (optional)
- [ ] Enable GitHub Discussions
- [ ] Organize issue tracker

#### Deliverables
- v1.0 official release

**Phase 5 Milestone Achievement Criteria:**
- [ ] Stable 60fps
- [ ] 95%+ compatibility
- [ ] 1% or less crash rate
- [ ] All documentation complete
- [ ] Release build creation complete

---

## Post-Release (Week 21+)

### Short-term Goals (v1.1-v1.3)

**v1.1 (1 month later):**
- [ ] Bug fixes
- [ ] Compatibility improvements
- [ ] Minor UI improvements

**v1.2 (2 months later):**
- [ ] Cheat functionality
- [ ] Screenshot functionality
- [ ] Gamepad vibration support

**v1.3 (3 months later):**
- [ ] Resolution enhancement (2x/4x)
- [ ] Texture filtering
- [ ] Shader support

### Long-term Goals (v2.0+)

- [ ] Recompiler implementation
- [ ] Network multiplayer
- [ ] Mobile support (Android)
- [ ] Plugin system

---

## Risk Management

### Expected Risks and Countermeasures

**Technical Risks:**

1. **Insufficient Performance**
   - Countermeasure: Early profiling, progressive optimization
   - Workaround: Early recompiler introduction

2. **Compatibility Issues**
   - Countermeasure: Continuous testing with various games
   - Workaround: Game-specific patch system

3. **GPU Accuracy Issues**
   - Countermeasure: Reference DuckStation implementation
   - Workaround: Add accuracy settings

**Schedule Risks:**

1. **Development Delays**
   - Countermeasure: Weekly progress checks, strict milestone adherence
   - Workaround: Scope reduction

2. **Loss of Motivation**
   - Countermeasure: Small success experiences, regular demos
   - Workaround: Community interaction

---

## Progress Management

### Weekly Tasks

**Every Week:**
- [ ] Create weekly log (`docs/09-progress/weekly-logs/`)
- [ ] Check task progress
- [ ] Plan next week

**Every Month:**
- [ ] Monthly review
- [ ] Performance measurement
- [ ] Roadmap review

### Deliverable Checklist

**At End of Each Phase:**
- [ ] Are milestone achievement criteria met?
- [ ] Is documentation updated?
- [ ] Are tests passing?
- [ ] Was demo video created?

---

## Summary

This roadmap is a **phased and realistic plan**. We'll steadily add functionality while maintaining a working state at each phase.

**Keys to Success:**
1. Thorough documentation-driven development
2. Accumulation of small success experiences
3. Continuous testing and quality control
4. Realistic scope management

By following this roadmap, we can release a **high-quality PSX emulator** in 5 months.

Let's do our best!
