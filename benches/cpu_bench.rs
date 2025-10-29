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

use criterion::{criterion_group, criterion_main, Criterion};
use echo_core::core::cpu::CPU;
use echo_core::core::memory::Bus;
use std::hint::black_box;

fn cpu_step_benchmark(c: &mut Criterion) {
    c.bench_function("cpu_step", |b| {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Write a NOP instruction to the BIOS area
        // NOP = 0x00000000 (SLL r0, r0, 0)
        bus.write32(0xBFC00000, 0x00000000).unwrap();

        b.iter(|| {
            cpu.reset();
            black_box(cpu.step(&mut bus).unwrap());
        });
    });
}

fn cpu_register_access_benchmark(c: &mut Criterion) {
    c.bench_function("cpu_register_read", |b| {
        let cpu = CPU::new();
        b.iter(|| {
            for i in 0..32 {
                black_box(cpu.reg(i));
            }
        });
    });

    c.bench_function("cpu_register_write", |b| {
        let mut cpu = CPU::new();
        b.iter(|| {
            for i in 0..32 {
                cpu.set_reg(i, black_box(i as u32 * 100));
            }
        });
    });
}

criterion_group!(benches, cpu_step_benchmark, cpu_register_access_benchmark);
criterion_main!(benches);
