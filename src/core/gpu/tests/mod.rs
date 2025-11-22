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

//! GPU module tests
//!
//! Tests are organized into the following modules:
//! - `basic`: Basic GPU functionality (initialization, reset, register access)
//! - `vram`: VRAM operations (read, write, transfers, addressing)
//! - `gp0_commands`: GP0 drawing commands and command buffering
//! - `gp1_commands`: GP1 control commands (display control, DMA, etc.)
//! - `rendering`: Rendering primitives (triangles, lines, gradients)
//! - `timing`: GPU timing and synchronization (VBlank, HBlank, scanlines)

mod basic;
mod gp0_commands;
mod gp1_commands;
mod rendering;
mod timing;
mod vram;
