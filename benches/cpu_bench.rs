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

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use psrx::core::cpu::icache::InstructionCache;
use psrx::core::cpu::CPU;
use psrx::core::memory::Bus;
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

fn icache_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_cache");

    // Benchmark cache hit
    group.bench_function("cache_hit", |b| {
        let mut cache = InstructionCache::new();
        cache.store(0x80000000, 0x3C080000);

        b.iter(|| {
            black_box(cache.fetch(black_box(0x80000000)));
        });
    });

    // Benchmark cache miss
    group.bench_function("cache_miss", |b| {
        let cache = InstructionCache::new();

        b.iter(|| {
            black_box(cache.fetch(black_box(0x80000000)));
        });
    });

    // Benchmark cache store
    group.bench_function("cache_store", |b| {
        let mut cache = InstructionCache::new();

        b.iter(|| {
            cache.store(black_box(0x80000000), black_box(0x3C080000));
        });
    });

    // Benchmark sequential access (high hit rate)
    group.bench_function("sequential_access", |b| {
        let mut cache = InstructionCache::new();

        // Prefill with sequential instructions
        for i in 0..100 {
            cache.store(0x80000000 + i * 4, 0x00000000);
        }

        b.iter(|| {
            for i in 0..100 {
                black_box(cache.fetch(black_box(0x80000000 + i * 4)));
            }
        });
    });

    // Benchmark cache eviction (conflicting addresses)
    group.bench_function("eviction_pattern", |b| {
        let mut cache = InstructionCache::new();

        b.iter(|| {
            // Two addresses that map to same cache line
            cache.store(black_box(0x80000000), black_box(0x11111111));
            cache.store(black_box(0x80001000), black_box(0x22222222));
        });
    });

    // Benchmark invalidation
    group.bench_function("invalidate", |b| {
        let mut cache = InstructionCache::new();
        cache.store(0x80000000, 0x3C080000);

        b.iter(|| {
            cache.invalidate(black_box(0x80000000));
        });
    });

    // Benchmark range invalidation
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("invalidate_range", size),
            size,
            |b, &size| {
                let mut cache = InstructionCache::new();

                // Fill cache
                for i in 0..size {
                    cache.store(0x80000000 + i * 4, 0x00000000);
                }

                b.iter(|| {
                    cache.invalidate_range(black_box(0x80000000), black_box(0x80000000 + size * 4));
                });
            },
        );
    }

    group.finish();
}

fn cpu_with_icache_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_with_icache");

    // Benchmark CPU step with cache hits
    group.bench_function("cpu_step_with_cache", |b| {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Write some instructions to BIOS area
        for i in 0..100 {
            bus.write32(0xBFC00000 + i * 4, 0x00000000).unwrap(); // NOPs
        }

        // Prefill cache
        for i in 0..100 {
            cpu.prefill_icache(0xBFC00000 + i * 4, 0x00000000);
        }

        b.iter(|| {
            cpu.reset();
            // Execute a few instructions to measure cache hit performance
            for _ in 0..10 {
                black_box(cpu.step(&mut bus).unwrap());
            }
        });
    });

    // Benchmark CPU step with cache misses
    group.bench_function("cpu_step_without_cache", |b| {
        let mut cpu = CPU::new();
        let mut bus = Bus::new();

        // Write some instructions to BIOS area
        for i in 0..100 {
            bus.write32(0xBFC00000 + i * 4, 0x00000000).unwrap(); // NOPs
        }

        b.iter(|| {
            cpu.reset();
            // Clear cache to force misses
            cpu.invalidate_icache_range(0xBFC00000, 0xBFC00400);

            // Execute a few instructions to measure cache miss performance
            for _ in 0..10 {
                black_box(cpu.step(&mut bus).unwrap());
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    cpu_step_benchmark,
    cpu_register_access_benchmark,
    icache_benchmark,
    cpu_with_icache_benchmark
);
criterion_main!(benches);
