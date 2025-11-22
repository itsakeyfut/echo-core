// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut

//! Memory Bus Tests
//!
//! This module contains comprehensive tests for the PlayStation memory bus,
//! organized into logical categories:
//!
//! - `basic`: Basic memory functionality and region identification
//! - `bus`: Memory read/write operations with various data sizes
//! - `regions`: Address translation and segment mirroring
//! - `helpers`: Common test utilities
//!
//! Tests cover:
//! - Address translation and segment mirroring (KUSEG, KSEG0, KSEG1)
//! - Memory region identification
//! - Read/write operations with various data sizes (8-bit, 16-bit, 32-bit)
//! - Alignment requirements
//! - Boundary conditions
//! - Endianness verification
//! - Expansion region behavior (ROM header and open bus)

use super::*;
use crate::core::memory::MemoryRegion;

mod basic;
mod bus;
mod helpers;
mod regions;
