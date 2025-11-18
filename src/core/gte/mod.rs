// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! GTE (Geometry Transformation Engine) implementation
//!
//! The GTE is Coprocessor 2 (COP2) for the PlayStation, responsible for
//! 3D geometry transformations and lighting calculations. It's essential
//! for 3D games performance.
//!
//! # Features
//!
//! - Matrix and vector operations
//! - Perspective transformation (RTPS, RTPT)
//! - Normal clipping (NCLIP)
//! - Color depth cueing
//! - Outer product calculations
//!
//! # Hardware Details
//!
//! - 32 data registers (32-bit each)
//! - 32 control registers (32-bit each)
//! - FLAG register for overflow/underflow tracking
//! - Fixed-point arithmetic (12.4 format for most operations)
//!
//! # References
//!
//! - [PSX-SPX: GTE](http://problemkaputt.de/psx-spx.htm#geometrytransformationenginegte)

/// GTE (Geometry Transformation Engine) - COP2
///
/// The GTE performs 3D geometry transformations and lighting calculations
/// for the PlayStation. It uses fixed-point arithmetic for performance.
///
/// # Example
///
/// ```
/// use psrx::core::gte::GTE;
///
/// let mut gte = GTE::new();
/// // Set up rotation matrix (identity)
/// gte.write_control(0, 0x1000);  // R11 = 1.0 in fixed-point
/// ```
pub struct GTE {
    /// Data registers (32 x 32-bit)
    ///
    /// These hold input vectors, intermediate results, and output values.
    /// Common registers:
    /// - VXY0-2: Input vectors (X,Y components)
    /// - VZ0-2: Input vectors (Z component)
    /// - SXY0-2: Screen coordinates (projection results)
    /// - MAC0-3: Multiply-accumulate registers
    data: [i32; 32],

    /// Control registers (32 x 32-bit)
    ///
    /// These hold transformation matrices, translation vectors,
    /// and projection parameters.
    /// Common registers:
    /// - RT (0-4): Rotation matrix (3x3)
    /// - TRX/TRY/TRZ: Translation vector
    /// - H: Projection plane distance
    control: [i32; 32],

    /// FLAGS register (read from data register 31)
    ///
    /// Tracks overflow, underflow, and other calculation flags.
    /// Bit layout:
    /// - Bits 0-11: Not used (always 0)
    /// - Bit 12: MAC0 overflow (positive)
    /// - Bit 13: MAC0 overflow (negative)
    /// - Bit 14: Divide overflow
    /// - Bit 15: SX2 saturated
    /// - Bit 16: SY2 saturated
    /// - Bit 17: IR0 saturated
    /// - Bit 18-29: Various saturation flags
    /// - Bit 30: Error flag (any error)
    /// - Bit 31: Flag bit (calculation overflow)
    flags: u32,
}

// Allow dead code for GTE register constants that will be used in future commands
#[allow(dead_code)]
impl GTE {
    // Data register indices (commonly used)
    // Note: Some constants are not yet used but will be needed for future GTE commands
    const VXY0: usize = 0; // Vector 0 X,Y components (s16, s16)
    const VZ0: usize = 1; // Vector 0 Z component (s16)
    const VXY1: usize = 2; // Vector 1 X,Y components
    const VZ1: usize = 3; // Vector 1 Z component
    const VXY2: usize = 4; // Vector 2 X,Y components
    const VZ2: usize = 5; // Vector 2 Z component
    const RGB: usize = 6; // Color/code value (RGBC)
    const OTZ: usize = 7; // Average Z value (for ordering table)
    const IR0: usize = 8; // Intermediate value 0
    const IR1: usize = 9; // Intermediate value 1
    const IR2: usize = 10; // Intermediate value 2
    const IR3: usize = 11; // Intermediate value 3
    const SXY0: usize = 12; // Screen XY coordinate FIFO (oldest)
    const SXY1: usize = 13; // Screen XY coordinate FIFO
    const SXY2: usize = 14; // Screen XY coordinate FIFO (newest)
    const SXYP: usize = 15; // Screen XY coordinate (latest)
    const SZ0: usize = 16; // Screen Z FIFO (oldest)
    const SZ1: usize = 17; // Screen Z FIFO
    const SZ2: usize = 18; // Screen Z FIFO
    const SZ3: usize = 19; // Screen Z FIFO (newest)
    const RGB0: usize = 20; // Color FIFO (oldest)
    const RGB1: usize = 21; // Color FIFO
    const RGB2: usize = 22; // Color FIFO (newest)
    const RES1: usize = 23; // Reserved
    const MAC0: usize = 24; // Multiply-accumulate register 0
    const MAC1: usize = 25; // Multiply-accumulate register 1
    const MAC2: usize = 26; // Multiply-accumulate register 2
    const MAC3: usize = 27; // Multiply-accumulate register 3
    const IRGB: usize = 28; // Color conversion input
    const ORGB: usize = 29; // Color conversion output
    const LZCS: usize = 30; // Leading zero count source
    const LZCR: usize = 31; // Leading zero count result (also FLAGS)

    // Control register indices
    const RT11_RT12: usize = 0; // Rotation matrix R11,R12
    const RT13_RT21: usize = 1; // Rotation matrix R13,R21
    const RT22_RT23: usize = 2; // Rotation matrix R22,R23
    const RT31_RT32: usize = 3; // Rotation matrix R31,R32
    const RT33: usize = 4; // Rotation matrix R33
    const TRX: usize = 5; // Translation vector X
    const TRY: usize = 6; // Translation vector Y
    const TRZ: usize = 7; // Translation vector Z
    const L11_L12: usize = 8; // Light matrix L11,L12
    const L13_L21: usize = 9; // Light matrix L13,L21
    const L22_L23: usize = 10; // Light matrix L22,L23
    const L31_L32: usize = 11; // Light matrix L31,L32
    const L33: usize = 12; // Light matrix L33
    const RBK: usize = 13; // Background color R
    const GBK: usize = 14; // Background color G
    const BBK: usize = 15; // Background color B
    const LR1_LR2: usize = 16; // Light color matrix LR1,LR2
    const LR3_LG1: usize = 17; // Light color matrix LR3,LG1
    const LG2_LG3: usize = 18; // Light color matrix LG2,LG3
    const LB1_LB2: usize = 19; // Light color matrix LB1,LB2
    const LB3: usize = 20; // Light color matrix LB3
    const RFC: usize = 21; // Far color R
    const GFC: usize = 22; // Far color G
    const BFC: usize = 23; // Far color B
    const OFX: usize = 24; // Screen offset X
    const OFY: usize = 25; // Screen offset Y
    const H: usize = 26; // Projection plane distance
    const DQA: usize = 27; // Depth queue parameter A
    const DQB: usize = 28; // Depth queue parameter B
    const ZSF3: usize = 29; // Z scale factor (1/3)
    const ZSF4: usize = 30; // Z scale factor (1/4)
    const FLAG: usize = 31; // FLAG register (same as data[31])

    /// Create a new GTE instance
    ///
    /// Initializes all registers to 0. In real hardware, registers
    /// would contain undefined values at power-on.
    ///
    /// # Returns
    ///
    /// A new GTE instance with all registers cleared
    ///
    /// # Example
    ///
    /// ```
    /// use psrx::core::gte::GTE;
    ///
    /// let gte = GTE::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: [0; 32],
            control: [0; 32],
            flags: 0,
        }
    }

    /// Reset GTE to initial state
    ///
    /// Clears all data and control registers.
    pub fn reset(&mut self) {
        self.data = [0; 32];
        self.control = [0; 32];
        self.flags = 0;
    }

    /// Read from data register
    ///
    /// # Arguments
    ///
    /// * `index` - Register index (0-31)
    ///
    /// # Returns
    ///
    /// Register value as i32
    ///
    /// # Note
    ///
    /// Register 31 (LZCR) also serves as the FLAGS register.
    #[inline(always)]
    pub fn read_data(&self, index: usize) -> i32 {
        self.data[index]
    }

    /// Write to data register
    ///
    /// # Arguments
    ///
    /// * `index` - Register index (0-31)
    /// * `value` - Value to write
    ///
    /// # Note
    ///
    /// Writing to register 15 (SXYP) also updates the FIFO.
    /// Writing to register 28 (IRGB) triggers color conversion.
    /// Writing to register 30 (LZCS) triggers leading zero count.
    #[inline(always)]
    pub fn write_data(&mut self, index: usize, value: i32) {
        match index {
            Self::SXYP => {
                // Writing to SXYP pushes to FIFO
                self.data[Self::SXY0] = self.data[Self::SXY1];
                self.data[Self::SXY1] = self.data[Self::SXY2];
                self.data[Self::SXY2] = value;
                self.data[Self::SXYP] = value;
            }
            Self::LZCS => {
                // Writing to LZCS triggers leading zero count
                self.data[Self::LZCS] = value;
                // Count leading zeros of the value (treating as unsigned)
                self.data[Self::LZCR] = (value as u32).leading_zeros() as i32;
            }
            _ => {
                self.data[index] = value;
            }
        }
    }

    /// Read from control register
    ///
    /// # Arguments
    ///
    /// * `index` - Register index (0-31)
    ///
    /// # Returns
    ///
    /// Register value as i32
    #[inline(always)]
    pub fn read_control(&self, index: usize) -> i32 {
        self.control[index]
    }

    /// Write to control register
    ///
    /// # Arguments
    ///
    /// * `index` - Register index (0-31)
    /// * `value` - Value to write
    #[inline(always)]
    pub fn write_control(&mut self, index: usize, value: i32) {
        self.control[index] = value;
    }

    /// Get rotation matrix from control registers
    ///
    /// The rotation matrix is stored across 5 control registers (0-4)
    /// in a packed format with 16-bit signed values.
    ///
    /// # Returns
    ///
    /// 3x3 rotation matrix as [[i32; 3]; 3]
    fn get_rotation_matrix(&self) -> [[i32; 3]; 3] {
        [
            [
                (self.control[Self::RT11_RT12] & 0xFFFF) as i16 as i32,
                (self.control[Self::RT11_RT12] >> 16) as i16 as i32,
                (self.control[Self::RT13_RT21] & 0xFFFF) as i16 as i32,
            ],
            [
                (self.control[Self::RT13_RT21] >> 16) as i16 as i32,
                (self.control[Self::RT22_RT23] & 0xFFFF) as i16 as i32,
                (self.control[Self::RT22_RT23] >> 16) as i16 as i32,
            ],
            [
                (self.control[Self::RT31_RT32] & 0xFFFF) as i16 as i32,
                (self.control[Self::RT31_RT32] >> 16) as i16 as i32,
                (self.control[Self::RT33] & 0xFFFF) as i16 as i32,
            ],
        ]
    }

    /// RTPS: Rotate, Translate, Perspective Transform, Single
    ///
    /// This is the most commonly used GTE command. It transforms a single
    /// 3D vertex from object space to screen space.
    ///
    /// Steps:
    /// 1. Load input vector from V0 (VXY0, VZ0)
    /// 2. Multiply by rotation matrix (RT)
    /// 3. Add translation vector (TR)
    /// 4. Perform perspective division
    /// 5. Add screen offset and store in SXY FIFO
    ///
    /// # Arguments
    ///
    /// * `sf` - Shift flag: if true, shift right by 12 bits (fixed-point adjustment)
    ///
    /// # Formula
    ///
    /// ```text
    /// MAC = RT * V + TR
    /// SXY = (H * MAC.xy / MAC.z) + Offset
    /// ```
    pub fn rtps(&mut self, sf: bool) {
        let shift = if sf { 12 } else { 0 };

        // Load input vector V0
        let vx = (self.data[Self::VXY0] & 0xFFFF) as i16 as i32;
        let vy = (self.data[Self::VXY0] >> 16) as i16 as i32;
        let vz = self.data[Self::VZ0] as i16 as i32;

        // Get rotation matrix
        let rt = self.get_rotation_matrix();

        // Matrix multiplication: MAC = RT * V
        // Cast to i64 before multiplication to prevent intermediate i32 overflow
        let mac1 = (rt[0][0] as i64 * vx as i64
            + rt[0][1] as i64 * vy as i64
            + rt[0][2] as i64 * vz as i64)
            >> shift;
        let mac2 = (rt[1][0] as i64 * vx as i64
            + rt[1][1] as i64 * vy as i64
            + rt[1][2] as i64 * vz as i64)
            >> shift;
        let mac3 = (rt[2][0] as i64 * vx as i64
            + rt[2][1] as i64 * vy as i64
            + rt[2][2] as i64 * vz as i64)
            >> shift;

        // Add translation vector
        let trx = self.control[Self::TRX] as i64;
        let try_val = self.control[Self::TRY] as i64;
        let trz = self.control[Self::TRZ] as i64;

        let mac1 = mac1 + (trx << shift);
        let mac2 = mac2 + (try_val << shift);
        let mac3 = mac3 + (trz << shift);

        // Store MAC values (saturated to 32-bit)
        self.data[Self::MAC1] = mac1.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.data[Self::MAC2] = mac2.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.data[Self::MAC3] = mac3.clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        // Perspective transformation
        let h = self.control[Self::H] as i64;
        let z = mac3;

        // Handle division with PlayStation hardware behavior:
        // - Negative Z is saturated to 0
        // - Z <= 0 triggers divide overflow (FLAG bit 14)
        // - Scale is clamped to 0x1FFFF (not 1!)
        let (sx, sy) = if z <= 0 {
            // Divide overflow case: set FLAG bit 14 and use maximum scale
            self.flags |= 1 << 14; // Bit 14: divide overflow

            // Use maximum scale (0x1FFFF) for overflow case
            let scale = 0x1FFFF_i64;
            let sx = (scale * mac1) as i32;
            let sy = (scale * mac2) as i32;
            (sx, sy)
        } else {
            // Normal division with scale clamped to 0x1FFFF
            let scale = (h / z).min(0x1FFFF);
            let sx = (scale * mac1) as i32;
            let sy = (scale * mac2) as i32;
            (sx, sy)
        };

        // Apply screen offset
        let ofx = self.control[Self::OFX];
        let ofy = self.control[Self::OFY];

        let sx_screen = sx + ofx;
        let sy_screen = sy + ofy;

        // Update screen coordinate FIFO
        self.data[Self::SXY0] = self.data[Self::SXY1];
        self.data[Self::SXY1] = self.data[Self::SXY2];
        self.data[Self::SXY2] =
            (sy_screen.clamp(-1024, 1023) << 16) | (sx_screen.clamp(-1024, 1023) & 0xFFFF);
        self.data[Self::SXYP] = self.data[Self::SXY2];

        // Update screen Z FIFO
        self.data[Self::SZ0] = self.data[Self::SZ1];
        self.data[Self::SZ1] = self.data[Self::SZ2];
        self.data[Self::SZ2] = self.data[Self::SZ3];
        self.data[Self::SZ3] = (z.clamp(0, 0xFFFF) as u16) as i32;

        // Calculate OTZ (average Z for ordering table)
        let sz_avg = (self.data[Self::SZ1] + self.data[Self::SZ2] + self.data[Self::SZ3]) / 3;
        self.data[Self::OTZ] = sz_avg.clamp(0, 0xFFFF);

        // Set IR registers (intermediate results)
        self.data[Self::IR1] = mac1.clamp(-32768, 32767) as i32;
        self.data[Self::IR2] = mac2.clamp(-32768, 32767) as i32;
        self.data[Self::IR3] = mac3.clamp(0, 65535) as i32;

        // Clear flags for simplicity (proper implementation would track all flags)
        self.flags = 0;
        self.data[Self::LZCR] = 0;
    }

    /// RTPT: Rotate, Translate, Perspective Transform, Triple
    ///
    /// Similar to RTPS but processes three vertices (V0, V1, V2) in sequence.
    /// This is more efficient than calling RTPS three times.
    ///
    /// # Arguments
    ///
    /// * `sf` - Shift flag: if true, shift right by 12 bits
    pub fn rtpt(&mut self, sf: bool) {
        // Process V0
        self.rtps(sf);

        // Process V1 - swap V0 with V1 temporarily
        let v0_xy = self.data[Self::VXY0];
        let v0_z = self.data[Self::VZ0];
        self.data[Self::VXY0] = self.data[Self::VXY1];
        self.data[Self::VZ0] = self.data[Self::VZ1];
        self.rtps(sf);

        // Process V2 - swap with V2
        self.data[Self::VXY0] = self.data[Self::VXY2];
        self.data[Self::VZ0] = self.data[Self::VZ2];
        self.rtps(sf);

        // Restore V0
        self.data[Self::VXY0] = v0_xy;
        self.data[Self::VZ0] = v0_z;
    }

    /// NCLIP: Normal Clipping
    ///
    /// Calculates the cross product of screen coordinates to determine
    /// if a triangle is front-facing or back-facing (for backface culling).
    ///
    /// # Formula
    ///
    /// ```text
    /// MAC0 = SX0*SY1 + SX1*SY2 + SX2*SY0 - SX0*SY2 - SX1*SY0 - SX2*SY1
    /// ```
    ///
    /// # Result
    ///
    /// - MAC0 > 0: Front-facing (clockwise)
    /// - MAC0 < 0: Back-facing (counter-clockwise)
    /// - MAC0 = 0: Edge-on
    pub fn nclip(&mut self) {
        // Extract screen coordinates from FIFO
        let sx0 = (self.data[Self::SXY0] & 0xFFFF) as i16 as i32;
        let sy0 = (self.data[Self::SXY0] >> 16) as i16 as i32;
        let sx1 = (self.data[Self::SXY1] & 0xFFFF) as i16 as i32;
        let sy1 = (self.data[Self::SXY1] >> 16) as i16 as i32;
        let sx2 = (self.data[Self::SXY2] & 0xFFFF) as i16 as i32;
        let sy2 = (self.data[Self::SXY2] >> 16) as i16 as i32;

        // Calculate cross product (determinant)
        let result =
            (sx0 * sy1) + (sx1 * sy2) + (sx2 * sy0) - (sx0 * sy2) - (sx1 * sy0) - (sx2 * sy1);

        self.data[Self::MAC0] = result;

        // Clear flags
        self.flags = 0;
        self.data[Self::LZCR] = 0;
    }

    /// MVMVA: Multiply Vector by Matrix and Vector Addition
    ///
    /// General-purpose matrix-vector multiplication with vector addition.
    /// This is a flexible command that can use different matrix and vector sources.
    ///
    /// # Arguments
    ///
    /// * `command` - Full GTE command word containing operation parameters
    ///
    /// # Format
    ///
    /// The command word specifies:
    /// - Bits [20:19]: Translation vector selection
    /// - Bits [18:17]: Multiply vector selection
    /// - Bits [16:15]: Multiply matrix selection
    /// - Bit 10: lm flag (limit negative to 0)
    /// - Bit 19: sf flag (shift fraction)
    pub fn mvmva(&mut self, command: u32) {
        let sf = ((command >> 19) & 1) != 0;
        let mx = (command >> 17) & 0x3; // Matrix selection
        let v = (command >> 15) & 0x3; // Vector selection
        let cv = (command >> 13) & 0x3; // Translation vector selection
        let lm = ((command >> 10) & 1) != 0; // Limit negative values

        let shift = if sf { 12 } else { 0 };

        // Select input vector
        let (vx, vy, vz) = match v {
            0 => {
                // V0
                let vx = (self.data[Self::VXY0] & 0xFFFF) as i16 as i32;
                let vy = (self.data[Self::VXY0] >> 16) as i16 as i32;
                let vz = self.data[Self::VZ0] as i16 as i32;
                (vx, vy, vz)
            }
            1 => {
                // V1
                let vx = (self.data[Self::VXY1] & 0xFFFF) as i16 as i32;
                let vy = (self.data[Self::VXY1] >> 16) as i16 as i32;
                let vz = self.data[Self::VZ1] as i16 as i32;
                (vx, vy, vz)
            }
            2 => {
                // V2
                let vx = (self.data[Self::VXY2] & 0xFFFF) as i16 as i32;
                let vy = (self.data[Self::VXY2] >> 16) as i16 as i32;
                let vz = self.data[Self::VZ2] as i16 as i32;
                (vx, vy, vz)
            }
            3 => {
                // IR (short vector)
                (
                    self.data[Self::IR1] as i16 as i32,
                    self.data[Self::IR2] as i16 as i32,
                    self.data[Self::IR3] as i16 as i32,
                )
            }
            _ => unreachable!(),
        };

        // Select matrix (simplified - only rotation matrix for now)
        let matrix = match mx {
            0..=3 => self.get_rotation_matrix(),
            _ => [[0; 3]; 3],
        };

        // Matrix multiplication
        // Cast to i64 before multiplication to prevent intermediate i32 overflow
        let mac1 = (matrix[0][0] as i64 * vx as i64
            + matrix[0][1] as i64 * vy as i64
            + matrix[0][2] as i64 * vz as i64)
            >> shift;
        let mac2 = (matrix[1][0] as i64 * vx as i64
            + matrix[1][1] as i64 * vy as i64
            + matrix[1][2] as i64 * vz as i64)
            >> shift;
        let mac3 = (matrix[2][0] as i64 * vx as i64
            + matrix[2][1] as i64 * vy as i64
            + matrix[2][2] as i64 * vz as i64)
            >> shift;

        // Add translation vector (simplified - using TR for now)
        let (tx, ty, tz) = match cv {
            0 | 1 => (
                self.control[Self::TRX] as i64,
                self.control[Self::TRY] as i64,
                self.control[Self::TRZ] as i64,
            ),
            _ => (0, 0, 0),
        };

        let mac1 = mac1 + (tx << shift);
        let mac2 = mac2 + (ty << shift);
        let mac3 = mac3 + (tz << shift);

        // Store results
        self.data[Self::MAC1] = mac1.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.data[Self::MAC2] = mac2.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.data[Self::MAC3] = mac3.clamp(i32::MIN as i64, i32::MAX as i64) as i32;

        // Update IR registers with limiting
        let min_val = if lm { 0 } else { -32768 };
        self.data[Self::IR1] = mac1.clamp(min_val, 32767) as i32;
        self.data[Self::IR2] = mac2.clamp(min_val, 32767) as i32;
        self.data[Self::IR3] = mac3.clamp(min_val, 32767) as i32;

        self.flags = 0;
        self.data[Self::LZCR] = 0;
    }

    /// Execute GTE command
    ///
    /// Dispatches a GTE command to the appropriate handler based on the opcode.
    ///
    /// # Arguments
    ///
    /// * `command` - 32-bit GTE command word
    ///
    /// # Format
    ///
    /// - Bits [5:0]: Opcode
    /// - Bit 19: sf (shift fraction)
    /// - Bit 10: lm (limit negative values)
    /// - Other bits: Command-specific parameters
    ///
    /// # Common Commands
    ///
    /// - 0x01: RTPS (Perspective transform single)
    /// - 0x06: NCLIP (Normal clipping)
    /// - 0x12: MVMVA (Matrix-vector multiply)
    /// - 0x30: RTPT (Perspective transform triple)
    pub fn execute(&mut self, command: u32) {
        let opcode = command & 0x3F;
        let sf = (command & 0x80000) != 0; // Shift flag (bit 19)

        match opcode {
            0x01 => self.rtps(sf),
            0x06 => self.nclip(),
            0x12 => self.mvmva(command),
            0x30 => self.rtpt(sf),
            // TODO: Implement remaining GTE commands as needed
            _ => {
                log::warn!("Unknown GTE command: 0x{:02X}", opcode);
                // Set error flag for unknown commands
                self.flags = 0x80000000;
                self.data[Self::LZCR] = 0x80000000u32 as i32;
            }
        }
    }
}

impl Default for GTE {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gte_initialization() {
        let gte = GTE::new();
        assert_eq!(gte.read_data(0), 0);
        assert_eq!(gte.read_control(0), 0);
        assert_eq!(gte.flags, 0);
    }

    #[test]
    fn test_data_register_read_write() {
        let mut gte = GTE::new();

        // Write and read data register
        gte.write_data(GTE::MAC1, 0x12345678);
        assert_eq!(gte.read_data(GTE::MAC1), 0x12345678);

        // Test register 31 (LZCR/FLAGS) can be written and read
        gte.write_data(GTE::LZCR, 0xABCDEF00u32 as i32);
        assert_eq!(gte.read_data(GTE::LZCR) as u32, 0xABCDEF00);
    }

    #[test]
    fn test_control_register_read_write() {
        let mut gte = GTE::new();

        // Write and read control register
        gte.write_control(GTE::TRX, 1000);
        assert_eq!(gte.read_control(GTE::TRX), 1000);

        gte.write_control(GTE::H, 2000);
        assert_eq!(gte.read_control(GTE::H), 2000);
    }

    #[test]
    fn test_rtps_identity_matrix() {
        let mut gte = GTE::new();

        // Setup identity rotation matrix (fixed-point 1.0 = 0x1000)
        gte.write_control(GTE::RT11_RT12, 0x00001000); // R11=1.0, R12=0
        gte.write_control(GTE::RT13_RT21, 0x10000000); // R13=0, R21=0
        gte.write_control(GTE::RT22_RT23, 0x00001000); // R22=1.0, R23=0
        gte.write_control(GTE::RT31_RT32, 0x00000000); // R31=0, R32=0
        gte.write_control(GTE::RT33, 0x00001000); // R33=1.0

        // Set translation to zero
        gte.write_control(GTE::TRX, 0);
        gte.write_control(GTE::TRY, 0);
        gte.write_control(GTE::TRZ, 1000);

        // Set projection parameters
        gte.write_control(GTE::H, 1000);
        gte.write_control(GTE::OFX, 0);
        gte.write_control(GTE::OFY, 0);

        // Input vector (10, 20, 30)
        gte.write_data(GTE::VXY0, (20 << 16) | (10 & 0xFFFF));
        gte.write_data(GTE::VZ0, 30);

        // Execute RTPS
        gte.rtps(false);

        // Verify MAC values are set (should be transformed coordinates)
        // With identity matrix and zero translation:
        // MAC1 should be proportional to input
        assert_ne!(gte.read_data(GTE::MAC1), 0);
        assert_ne!(gte.read_data(GTE::MAC2), 0);
        assert_ne!(gte.read_data(GTE::MAC3), 0);

        // Screen coordinates should be calculated
        assert_ne!(gte.read_data(GTE::SXY2), 0);
    }

    #[test]
    fn test_nclip_clockwise() {
        let mut gte = GTE::new();

        // Set up clockwise triangle
        // (0,0), (10,0), (5,10)
        gte.write_data(GTE::SXY0, 0); // (0, 0)
        gte.write_data(GTE::SXY1, 10); // (10, 0)
        gte.write_data(GTE::SXY2, (10 << 16) | (5 & 0xFFFF)); // (5, 10)

        gte.nclip();

        let mac0 = gte.read_data(GTE::MAC0);
        // For clockwise winding, MAC0 should be positive
        assert!(mac0 > 0, "MAC0 should be positive for clockwise triangle");
    }

    #[test]
    fn test_nclip_counter_clockwise() {
        let mut gte = GTE::new();

        // Set up counter-clockwise triangle (reversed winding)
        // (0,0), (5,10), (10,0)
        gte.write_data(GTE::SXY0, 0); // (0, 0)
        gte.write_data(GTE::SXY1, (10 << 16) | (5 & 0xFFFF)); // (5, 10)
        gte.write_data(GTE::SXY2, 10); // (10, 0)

        gte.nclip();

        let mac0 = gte.read_data(GTE::MAC0);
        // For counter-clockwise winding, MAC0 should be negative
        assert!(
            mac0 < 0,
            "MAC0 should be negative for counter-clockwise triangle"
        );
    }

    #[test]
    fn test_leading_zero_count() {
        let mut gte = GTE::new();

        // Test positive value
        gte.write_data(GTE::LZCS, 0x00000001);
        assert_eq!(gte.read_data(GTE::LZCR), 31);

        // Test negative value
        gte.write_data(GTE::LZCS, -1);
        assert_eq!(gte.read_data(GTE::LZCR), 0);

        // Test zero
        gte.write_data(GTE::LZCS, 0);
        assert_eq!(gte.read_data(GTE::LZCR), 32);
    }

    #[test]
    fn test_sxy_fifo() {
        let mut gte = GTE::new();

        // Write to SXYP should push to FIFO
        gte.write_data(GTE::SXYP, 100);
        assert_eq!(gte.read_data(GTE::SXY2), 100);
        assert_eq!(gte.read_data(GTE::SXY1), 0);
        assert_eq!(gte.read_data(GTE::SXY0), 0);

        gte.write_data(GTE::SXYP, 200);
        assert_eq!(gte.read_data(GTE::SXY2), 200);
        assert_eq!(gte.read_data(GTE::SXY1), 100);
        assert_eq!(gte.read_data(GTE::SXY0), 0);

        gte.write_data(GTE::SXYP, 300);
        assert_eq!(gte.read_data(GTE::SXY2), 300);
        assert_eq!(gte.read_data(GTE::SXY1), 200);
        assert_eq!(gte.read_data(GTE::SXY0), 100);
    }

    #[test]
    fn test_execute_rtps_command() {
        let mut gte = GTE::new();

        // Setup basic parameters
        gte.write_control(GTE::RT11_RT12, 0x00001000);
        gte.write_control(GTE::RT22_RT23, 0x00001000);
        gte.write_control(GTE::RT33, 0x00001000);
        gte.write_control(GTE::H, 1000);

        // Input vector
        gte.write_data(GTE::VXY0, (100 << 16) | (50 & 0xFFFF));
        gte.write_data(GTE::VZ0, 200);

        // Execute RTPS via command (opcode 0x01)
        gte.execute(0x01);

        // Verify execution completed (screen coordinates updated)
        assert_ne!(gte.read_data(GTE::SXY2), 0);
    }

    #[test]
    fn test_execute_nclip_command() {
        let mut gte = GTE::new();

        // Setup triangle
        gte.write_data(GTE::SXY0, 0);
        gte.write_data(GTE::SXY1, 10);
        gte.write_data(GTE::SXY2, (10 << 16) | (5 & 0xFFFF));

        // Execute NCLIP via command (opcode 0x06)
        gte.execute(0x06);

        // Verify MAC0 was calculated
        assert_ne!(gte.read_data(GTE::MAC0), 0);
    }

    #[test]
    fn test_execute_unknown_command() {
        let mut gte = GTE::new();

        // Execute unknown command
        gte.execute(0xFF);

        // Should set error flag
        assert_ne!(gte.flags & 0x80000000, 0);
    }
}
