# GPU Design Document

## Overview

Architecture and implementation design for the PlayStation GPU (CXD8561Q). Covers 2D rendering, VRAM access, DMA transfers, and display output.

## GPU Architecture

### Basic Specifications

- **Processor**: CXD8561Q (Sony custom GPU)
- **Rendering Performance**:
  - Flat shading: 180,000 polygons/sec
  - Gouraud shading: 90,000 polygons/sec
  - Texture mapping: 50,000 polygons/sec
- **VRAM**: 1MB (1024×512 16-bit)
- **Color Depth**: 16-bit/24-bit
- **Resolution**: 256×224 to 640×480
- **Framebuffer**: Supports double buffering

### Memory Map

```
GPU Registers:
0x1F801810: GP0 (Drawing commands)
0x1F801814: GP1 (GPU control)
0x1F801814: GPUSTAT (read)

DMA Registers:
0x1F8010F0-0x1F8010FF: DMA control
0x1F8010A0-0x1F8010AF: DMA Channel 2 (GPU)
```

## Data Structures

### GPU State

```rust
pub struct Gpu {
    /// VRAM (1024×512 pixels, 16bit/pixel)
    vram: Box<[u16; 1024 * 512]>,

    /// Framebuffer configuration
    display_area: DisplayArea,
    drawing_area: DrawingArea,

    /// Texture configuration
    texture_window: TextureWindow,
    texture_page: TexturePage,

    /// Drawing settings
    drawing_offset: (i16, i16),
    draw_mode: DrawMode,

    /// Display settings
    display_mode: DisplayMode,
    display_enabled: bool,

    /// GP0 command buffer
    command_buffer: Vec<u32>,
    remaining_words: usize,

    /// Status
    status: GpuStatus,

    /// DMA configuration
    dma_direction: DmaDirection,

    /// Cycle counter
    cycles: u64,

    /// Renderer backend
    renderer: Box<dyn Renderer>,
}

#[derive(Clone)]
pub struct DisplayArea {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone)]
pub struct DrawingArea {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

#[derive(Clone)]
pub struct TextureWindow {
    pub mask_x: u8,
    pub mask_y: u8,
    pub offset_x: u8,
    pub offset_y: u8,
}

#[derive(Clone)]
pub struct TexturePage {
    pub base_x: u16,
    pub base_y: u16,
    pub semi_transparency: SemiTransparency,
    pub color_mode: TextureColorMode,
}

#[derive(Clone, Copy)]
pub enum TextureColorMode {
    T4Bit,   // 4-bit CLUT
    T8Bit,   // 8-bit CLUT
    T16Bit,  // 16-bit direct
}

#[derive(Clone, Copy)]
pub enum SemiTransparency {
    Half,           // 0.5×B + 0.5×F
    Add,            // 1.0×B + 1.0×F
    Subtract,       // 1.0×B - 1.0×F
    AddQuarter,     // 1.0×B + 0.25×F
}

pub struct DrawMode {
    pub texture_disable: bool,
    pub dither_enable: bool,
    pub drawing_to_display_area: bool,
    pub check_mask_bit: bool,
    pub set_mask_bit: bool,
}

pub struct DisplayMode {
    pub horizontal_res: HorizontalResolution,
    pub vertical_res: VerticalResolution,
    pub video_mode: VideoMode,
    pub color_depth: ColorDepth,
    pub vertical_interlace: bool,
}

#[derive(Clone, Copy)]
pub enum HorizontalResolution {
    R256,
    R320,
    R512,
    R640,
    R368,  // Rarely used
}

#[derive(Clone, Copy)]
pub enum VerticalResolution {
    R240,
    R480,
}

#[derive(Clone, Copy)]
pub enum VideoMode {
    Ntsc,
    Pal,
}

#[derive(Clone, Copy)]
pub enum ColorDepth {
    C15Bit,
    C24Bit,
}

bitflags::bitflags! {
    pub struct GpuStatus: u32 {
        const TEXTURE_PAGE_X_BASE       = 0x0000000F;
        const TEXTURE_PAGE_Y_BASE       = 0x00000010;
        const SEMI_TRANSPARENCY         = 0x00000060;
        const TEXTURE_DEPTH             = 0x00000180;
        const DITHER_ENABLED            = 0x00000200;
        const DRAWING_TO_DISPLAY        = 0x00000400;
        const MASK_BIT_FORCE            = 0x00000800;
        const DRAW_PIXELS               = 0x00001000;
        const INTERLACE_FIELD           = 0x00002000;
        const REVERSE_FLAG              = 0x00004000;
        const TEXTURE_DISABLE           = 0x00008000;
        const HORIZONTAL_RES_2          = 0x00010000;
        const HORIZONTAL_RES_1          = 0x00060000;
        const VERTICAL_RES              = 0x00080000;
        const VIDEO_MODE                = 0x00100000;
        const COLOR_DEPTH               = 0x00200000;
        const VERTICAL_INTERLACE        = 0x00400000;
        const DISPLAY_DISABLED          = 0x00800000;
        const INTERRUPT_REQUEST         = 0x01000000;
        const DMA_DATA_REQUEST          = 0x02000000;
        const READY_RECEIVE_CMD         = 0x04000000;
        const READY_SEND_VRAM           = 0x08000000;
        const READY_RECEIVE_DMA         = 0x10000000;
        const DMA_DIRECTION             = 0x60000000;
        const INTERLACE_ODD             = 0x80000000;
    }
}

#[derive(Clone, Copy)]
pub enum DmaDirection {
    Off,
    Fifo,
    CpuToGp0,
    VramToCpu,
}
```

## GP0 Command Processing

### Command Dispatch

```rust
impl Gpu {
    pub fn gp0(&mut self, value: u32) -> Result<(), GpuError> {
        if self.remaining_words == 0 {
            // New command
            let opcode = (value >> 24) as u8;
            self.remaining_words = Self::command_size(opcode);
            self.command_buffer.clear();
        }

        self.command_buffer.push(value);
        self.remaining_words -= 1;

        if self.remaining_words == 0 {
            self.execute_command()?;
        }

        Ok(())
    }

    fn command_size(opcode: u8) -> usize {
        match opcode {
            // NOP
            0x00 => 1,

            // Drawing commands
            0x20..=0x3f => Self::poly_size(opcode),
            0x40..=0x5f => Self::line_size(opcode),
            0x60..=0x7f => Self::rect_size(opcode),

            // VRAM transfers
            0x80..=0x9f => 4,  // VRAM-to-CPU
            0xa0..=0xbf => 3,  // CPU-to-VRAM
            0xc0..=0xdf => 3,  // VRAM-to-VRAM

            // Environment settings
            0xe1 => 1,  // Draw mode setting
            0xe2 => 1,  // Texture window setting
            0xe3 => 1,  // Drawing area top left
            0xe4 => 1,  // Drawing area bottom right
            0xe5 => 1,  // Drawing offset
            0xe6 => 1,  // Mask bit setting

            _ => 1,
        }
    }

    fn poly_size(opcode: u8) -> usize {
        let is_quad = (opcode & 0x08) != 0;
        let has_texture = (opcode & 0x04) != 0;
        let is_gouraud = (opcode & 0x10) != 0;

        let vertex_count = if is_quad { 4 } else { 3 };
        let words_per_vertex = if has_texture { 2 } else { 1 };
        let color_words = if is_gouraud { vertex_count } else { 1 };

        color_words + (vertex_count * words_per_vertex)
    }

    fn line_size(opcode: u8) -> usize {
        let is_polyline = (opcode & 0x08) != 0;
        let is_gouraud = (opcode & 0x10) != 0;

        if is_polyline {
            // Variable length (terminated with 0x55555555)
            // Set maximum length in implementation
            256
        } else {
            // Single line
            if is_gouraud { 4 } else { 3 }
        }
    }

    fn rect_size(opcode: u8) -> usize {
        let has_texture = (opcode & 0x04) != 0;
        let is_variable = (opcode & 0x18) == 0x00;

        let base = if has_texture { 2 } else { 1 };
        if is_variable { base + 1 } else { base }
    }

    fn execute_command(&mut self) -> Result<(), GpuError> {
        let opcode = (self.command_buffer[0] >> 24) as u8;

        match opcode {
            0x00 => Ok(()), // NOP

            // Fill rectangle
            0x02 => self.fill_rectangle(),

            // Polygon drawing
            0x20..=0x3f => self.draw_polygon(opcode),

            // Line drawing
            0x40..=0x5f => self.draw_line(opcode),

            // Rectangle drawing
            0x60..=0x7f => self.draw_rectangle(opcode),

            // VRAM transfers
            0x80..=0x9f => self.vram_to_cpu(),
            0xa0..=0xbf => self.cpu_to_vram(),
            0xc0..=0xdf => self.vram_to_vram(),

            // Environment settings
            0xe1 => self.set_draw_mode(),
            0xe2 => self.set_texture_window(),
            0xe3 => self.set_drawing_area_top_left(),
            0xe4 => self.set_drawing_area_bottom_right(),
            0xe5 => self.set_drawing_offset(),
            0xe6 => self.set_mask_bit_setting(),

            _ => {
                warn!("Unknown GP0 command: {:#04x}", opcode);
                Ok(())
            }
        }
    }
}
```

### Polygon Drawing

```rust
impl Gpu {
    fn draw_polygon(&mut self, opcode: u8) -> Result<(), GpuError> {
        let is_quad = (opcode & 0x08) != 0;
        let has_texture = (opcode & 0x04) != 0;
        let is_semi_transparent = (opcode & 0x02) != 0;
        let is_raw_texture = (opcode & 0x01) != 0;
        let is_gouraud = (opcode & 0x10) != 0;

        let mut vertices = Vec::new();
        let vertex_count = if is_quad { 4 } else { 3 };

        let mut cmd_index = 0;

        for i in 0..vertex_count {
            let color = if is_gouraud || i == 0 {
                cmd_index += 1;
                Color::from_word(self.command_buffer[cmd_index - 1])
            } else {
                vertices[0].color
            };

            cmd_index += 1;
            let pos_word = self.command_buffer[cmd_index - 1];
            let x = (pos_word & 0xffff) as i16;
            let y = (pos_word >> 16) as i16;

            let (u, v, clut) = if has_texture {
                cmd_index += 1;
                let tex_word = self.command_buffer[cmd_index - 1];
                let u = (tex_word & 0xff) as u8;
                let v = ((tex_word >> 8) & 0xff) as u8;
                let clut = if i == 0 {
                    ((tex_word >> 16) & 0xffff) as u16
                } else {
                    vertices[0].clut
                };
                (u, v, clut)
            } else {
                (0, 0, 0)
            };

            vertices.push(Vertex {
                x: x + self.drawing_offset.0,
                y: y + self.drawing_offset.1,
                color,
                u,
                v,
                clut,
            });
        }

        // Execute drawing
        if is_quad {
            // Split quad into 2 triangles
            self.renderer.draw_triangle(&vertices[0], &vertices[1], &vertices[2], has_texture)?;
            self.renderer.draw_triangle(&vertices[1], &vertices[2], &vertices[3], has_texture)?;
        } else {
            self.renderer.draw_triangle(&vertices[0], &vertices[1], &vertices[2], has_texture)?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct Vertex {
    pub x: i16,
    pub y: i16,
    pub color: Color,
    pub u: u8,
    pub v: u8,
    pub clut: u16,
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    fn from_word(word: u32) -> Self {
        Self {
            r: (word & 0xff) as u8,
            g: ((word >> 8) & 0xff) as u8,
            b: ((word >> 16) & 0xff) as u8,
        }
    }

    fn to_rgb555(self) -> u16 {
        let r = (self.r >> 3) as u16;
        let g = (self.g >> 3) as u16;
        let b = (self.b >> 3) as u16;
        (b << 10) | (g << 5) | r
    }
}
```

### Line Drawing

```rust
impl Gpu {
    fn draw_line(&mut self, opcode: u8) -> Result<(), GpuError> {
        let is_polyline = (opcode & 0x08) != 0;
        let is_gouraud = (opcode & 0x10) != 0;

        if is_polyline {
            self.draw_polyline(is_gouraud)
        } else {
            self.draw_single_line(is_gouraud)
        }
    }

    fn draw_single_line(&mut self, is_gouraud: bool) -> Result<(), GpuError> {
        let color0 = Color::from_word(self.command_buffer[0]);
        let pos0 = self.command_buffer[1];
        let x0 = (pos0 & 0xffff) as i16 + self.drawing_offset.0;
        let y0 = (pos0 >> 16) as i16 + self.drawing_offset.1;

        let (color1, pos1_idx) = if is_gouraud {
            (Color::from_word(self.command_buffer[2]), 3)
        } else {
            (color0, 2)
        };

        let pos1 = self.command_buffer[pos1_idx];
        let x1 = (pos1 & 0xffff) as i16 + self.drawing_offset.0;
        let y1 = (pos1 >> 16) as i16 + self.drawing_offset.1;

        self.renderer.draw_line(x0, y0, x1, y1, color0, color1, is_gouraud)?;

        Ok(())
    }

    fn draw_polyline(&mut self, is_gouraud: bool) -> Result<(), GpuError> {
        let mut i = 0;
        let mut prev_color = Color::from_word(self.command_buffer[i]);
        i += 1;

        let pos = self.command_buffer[i];
        let mut prev_x = (pos & 0xffff) as i16 + self.drawing_offset.0;
        let mut prev_y = (pos >> 16) as i16 + self.drawing_offset.1;
        i += 1;

        while i < self.command_buffer.len() {
            let word = self.command_buffer[i];

            // Check for terminator
            if word == 0x55555555 {
                break;
            }

            let color = if is_gouraud {
                i += 1;
                Color::from_word(self.command_buffer[i - 1])
            } else {
                prev_color
            };

            let x = (word & 0xffff) as i16 + self.drawing_offset.0;
            let y = (word >> 16) as i16 + self.drawing_offset.1;

            self.renderer.draw_line(prev_x, prev_y, x, y, prev_color, color, is_gouraud)?;

            prev_x = x;
            prev_y = y;
            prev_color = color;
            i += 1;
        }

        Ok(())
    }
}
```

### Rectangle Drawing

```rust
impl Gpu {
    fn draw_rectangle(&mut self, opcode: u8) -> Result<(), GpuError> {
        let has_texture = (opcode & 0x04) != 0;
        let size_type = opcode & 0x18;

        let color = Color::from_word(self.command_buffer[0]);

        let pos = self.command_buffer[1];
        let x = (pos & 0xffff) as i16 + self.drawing_offset.0;
        let y = (pos >> 16) as i16 + self.drawing_offset.1;

        let (u, v) = if has_texture {
            let tex = self.command_buffer[2];
            ((tex & 0xff) as u8, ((tex >> 8) & 0xff) as u8)
        } else {
            (0, 0)
        };

        let (width, height) = match size_type {
            0x00 => {
                // Variable size
                let size_idx = if has_texture { 3 } else { 2 };
                let size = self.command_buffer[size_idx];
                ((size & 0xffff) as u16, (size >> 16) as u16)
            }
            0x08 => (1, 1),       // 1x1
            0x10 => (8, 8),       // 8x8
            0x18 => (16, 16),     // 16x16
            _ => unreachable!(),
        };

        self.renderer.draw_rectangle(x, y, width, height, color, u, v, has_texture)?;

        Ok(())
    }

    fn fill_rectangle(&mut self) -> Result<(), GpuError> {
        let color = Color::from_word(self.command_buffer[0]);

        let top_left = self.command_buffer[1];
        let x = (top_left & 0xffff) as u16;
        let y = (top_left >> 16) as u16;

        let size = self.command_buffer[2];
        let width = (size & 0xffff) as u16;
        let height = (size >> 16) as u16;

        // Direct VRAM write (ignores drawing offset)
        for dy in 0..height {
            for dx in 0..width {
                let vram_x = (x + dx) & 0x3ff;
                let vram_y = (y + dy) & 0x1ff;
                let offset = (vram_y as usize) * 1024 + (vram_x as usize);
                self.vram[offset] = color.to_rgb555();
            }
        }

        Ok(())
    }
}
```

## VRAM Transfers

```rust
impl Gpu {
    fn cpu_to_vram(&mut self) -> Result<(), GpuError> {
        let dest = self.command_buffer[1];
        let x = (dest & 0xffff) as u16;
        let y = (dest >> 16) as u16;

        let size = self.command_buffer[2];
        let width = (size & 0xffff) as u16;
        let height = (size >> 16) as u16;

        // Prepare to receive data via DMA or FIFO
        self.status.insert(GpuStatus::READY_RECEIVE_DMA);

        // Actual transfer is done via DMA
        // Only preparation here

        Ok(())
    }

    pub fn write_vram_data(&mut self, data: &[u32]) -> Result<(), GpuError> {
        // Called only during CPU-to-VRAM command execution
        let dest = self.command_buffer[1];
        let mut x = (dest & 0xffff) as u16;
        let mut y = (dest >> 16) as u16;

        let size = self.command_buffer[2];
        let width = (size & 0xffff) as u16;

        for &word in data {
            // Extract 2×16-bit from 32-bit word
            let pixel0 = (word & 0xffff) as u16;
            let pixel1 = (word >> 16) as u16;

            self.write_vram_pixel(x, y, pixel0);
            x += 1;
            if x >= width {
                x = 0;
                y += 1;
            }

            self.write_vram_pixel(x, y, pixel1);
            x += 1;
            if x >= width {
                x = 0;
                y += 1;
            }
        }

        Ok(())
    }

    fn write_vram_pixel(&mut self, x: u16, y: u16, pixel: u16) {
        let vram_x = x & 0x3ff;
        let vram_y = y & 0x1ff;
        let offset = (vram_y as usize) * 1024 + (vram_x as usize);
        self.vram[offset] = pixel;
    }

    fn vram_to_cpu(&mut self) -> Result<(), GpuError> {
        let src = self.command_buffer[1];
        let x = (src & 0xffff) as u16;
        let y = (src >> 16) as u16;

        let size = self.command_buffer[2];
        let width = (size & 0xffff) as u16;
        let height = (size >> 16) as u16;

        // Prepare for VRAM read
        self.status.insert(GpuStatus::READY_SEND_VRAM);

        // Prepare read buffer
        // Actual reading is done via GPUREAD

        Ok(())
    }

    fn vram_to_vram(&mut self) -> Result<(), GpuError> {
        let src = self.command_buffer[1];
        let src_x = (src & 0xffff) as u16;
        let src_y = (src >> 16) as u16;

        let dest = self.command_buffer[2];
        let dest_x = (dest & 0xffff) as u16;
        let dest_y = (dest >> 16) as u16;

        let size = self.command_buffer[3];
        let width = (size & 0xffff) as u16;
        let height = (size >> 16) as u16;

        // VRAM copy
        for dy in 0..height {
            for dx in 0..width {
                let src_vx = (src_x + dx) & 0x3ff;
                let src_vy = (src_y + dy) & 0x1ff;
                let src_offset = (src_vy as usize) * 1024 + (src_vx as usize);

                let dest_vx = (dest_x + dx) & 0x3ff;
                let dest_vy = (dest_y + dy) & 0x1ff;
                let dest_offset = (dest_vy as usize) * 1024 + (dest_vx as usize);

                self.vram[dest_offset] = self.vram[src_offset];
            }
        }

        Ok(())
    }
}
```

## GP1 Command Processing

```rust
impl Gpu {
    pub fn gp1(&mut self, value: u32) -> Result<(), GpuError> {
        let command = (value >> 24) as u8;
        let parameter = value & 0xffffff;

        match command {
            0x00 => self.gp1_reset(),
            0x01 => self.gp1_reset_command_buffer(),
            0x02 => self.gp1_acknowledge_irq(),
            0x03 => self.gp1_display_enable(parameter),
            0x04 => self.gp1_dma_direction(parameter),
            0x05 => self.gp1_display_vram_start(parameter),
            0x06 => self.gp1_display_horizontal_range(parameter),
            0x07 => self.gp1_display_vertical_range(parameter),
            0x08 => self.gp1_display_mode(parameter),
            0x10..=0x1f => self.gp1_get_gpu_info(command & 0x0f),
            _ => {
                warn!("Unknown GP1 command: {:#04x}", command);
                Ok(())
            }
        }
    }

    fn gp1_reset(&mut self) -> Result<(), GpuError> {
        // Complete GPU reset
        self.status = GpuStatus::empty();
        self.command_buffer.clear();
        self.remaining_words = 0;

        // Default settings
        self.display_area = DisplayArea { x: 0, y: 0, width: 256, height: 240 };
        self.drawing_area = DrawingArea { left: 0, top: 0, right: 1023, bottom: 511 };
        self.drawing_offset = (0, 0);
        self.texture_window = TextureWindow { mask_x: 0, mask_y: 0, offset_x: 0, offset_y: 0 };

        Ok(())
    }

    fn gp1_reset_command_buffer(&mut self) -> Result<(), GpuError> {
        self.command_buffer.clear();
        self.remaining_words = 0;
        Ok(())
    }

    fn gp1_acknowledge_irq(&mut self) -> Result<(), GpuError> {
        self.status.remove(GpuStatus::INTERRUPT_REQUEST);
        Ok(())
    }

    fn gp1_display_enable(&mut self, parameter: u32) -> Result<(), GpuError> {
        self.display_enabled = (parameter & 1) == 0;

        if self.display_enabled {
            self.status.remove(GpuStatus::DISPLAY_DISABLED);
        } else {
            self.status.insert(GpuStatus::DISPLAY_DISABLED);
        }

        Ok(())
    }

    fn gp1_dma_direction(&mut self, parameter: u32) -> Result<(), GpuError> {
        self.dma_direction = match parameter & 3 {
            0 => DmaDirection::Off,
            1 => DmaDirection::Fifo,
            2 => DmaDirection::CpuToGp0,
            3 => DmaDirection::VramToCpu,
            _ => unreachable!(),
        };

        Ok(())
    }

    fn gp1_display_vram_start(&mut self, parameter: u32) -> Result<(), GpuError> {
        self.display_area.x = (parameter & 0x3ff) as u16;
        self.display_area.y = ((parameter >> 10) & 0x1ff) as u16;
        Ok(())
    }

    fn gp1_display_horizontal_range(&mut self, parameter: u32) -> Result<(), GpuError> {
        let x1 = parameter & 0xfff;
        let x2 = (parameter >> 12) & 0xfff;
        self.display_area.width = ((x2 - x1) / 8) as u16;
        Ok(())
    }

    fn gp1_display_vertical_range(&mut self, parameter: u32) -> Result<(), GpuError> {
        let y1 = parameter & 0x3ff;
        let y2 = (parameter >> 10) & 0x3ff;
        self.display_area.height = (y2 - y1) as u16;
        Ok(())
    }

    fn gp1_display_mode(&mut self, parameter: u32) -> Result<(), GpuError> {
        // Bit 0-1: Horizontal resolution 1
        // Bit 2: Vertical resolution
        // Bit 3: Video mode
        // Bit 4: Color depth
        // Bit 5: Vertical interlace
        // Bit 6: Horizontal resolution 2
        // Bit 7: Reverse flag

        self.display_mode.horizontal_res = match ((parameter >> 6) & 1, parameter & 3) {
            (0, 0) => HorizontalResolution::R256,
            (0, 1) => HorizontalResolution::R320,
            (0, 2) => HorizontalResolution::R512,
            (0, 3) => HorizontalResolution::R640,
            (1, 0) => HorizontalResolution::R368,
            _ => HorizontalResolution::R256,
        };

        self.display_mode.vertical_res = if (parameter >> 2) & 1 != 0 {
            VerticalResolution::R480
        } else {
            VerticalResolution::R240
        };

        self.display_mode.video_mode = if (parameter >> 3) & 1 != 0 {
            VideoMode::Pal
        } else {
            VideoMode::Ntsc
        };

        self.display_mode.color_depth = if (parameter >> 4) & 1 != 0 {
            ColorDepth::C24Bit
        } else {
            ColorDepth::C15Bit
        };

        self.display_mode.vertical_interlace = (parameter >> 5) & 1 != 0;

        Ok(())
    }

    fn gp1_get_gpu_info(&mut self, info_type: u32) -> Result<(), GpuError> {
        // Get GPU information (returned via GPUREAD)
        // Returns different values based on info type
        Ok(())
    }
}
```

## Renderer Trait

```rust
pub trait Renderer {
    fn draw_triangle(
        &mut self,
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        textured: bool,
    ) -> Result<(), GpuError>;

    fn draw_line(
        &mut self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        color0: Color,
        color1: Color,
        shaded: bool,
    ) -> Result<(), GpuError>;

    fn draw_rectangle(
        &mut self,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        color: Color,
        u: u8,
        v: u8,
        textured: bool,
    ) -> Result<(), GpuError>;

    fn present(&mut self, vram: &[u16; 1024 * 512], display_area: &DisplayArea) -> Result<(), GpuError>;
}
```

## Summary

- **GP0/GP1**: Two independent command systems
- **VRAM Management**: Efficient management of 1MB framebuffer
- **Drawing Primitives**: Supports polygons, lines, and rectangles
- **Texture Mapping**: 4-bit/8-bit/16-bit support
- **DMA Transfers**: High-speed VRAM access
