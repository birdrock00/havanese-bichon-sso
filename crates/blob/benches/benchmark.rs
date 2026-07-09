use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::time::Duration;
use tempfile::TempDir;

use bichon_blob::{Codec, Config, Engine};

fn make_key(seed: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&seed.to_le_bytes());
    key
}

fn make_value(size: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(size);
    let pattern = b"The quick brown fox jumps over the lazy dog. ";
    while v.len() < size {
        let rem = size - v.len();
        let n = rem.min(pattern.len());
        v.extend_from_slice(&pattern[..n]);
    }
    v
}

pub fn bench_write_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let value = make_value(1024); // 1 KB
    let mut counter = 0u64;

    group.bench_function("1KB", |b| {
        b.iter_batched(
            || {
                counter += 1;
                (make_key(counter), value.clone())
            },
            |(key, val)| {
                engine.put(key, &val, Codec::Zstd).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_write_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    group.throughput(Throughput::Bytes(64 * 1024));
    group.measurement_time(Duration::from_secs(10));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let value = make_value(64 * 1024); // 64 KB
    let mut counter = 0u64;

    group.bench_function("64KB", |b| {
        b.iter_batched(
            || {
                counter += 1;
                (make_key(counter), value.clone())
            },
            |(key, val)| {
                engine.put(key, &val, Codec::Zstd).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_write_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    group.throughput(Throughput::Bytes(1024 * 1024));
    group.measurement_time(Duration::from_secs(15));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let value = make_value(1024 * 1024); // 1 MB
    let mut counter = 0u64;

    group.bench_function("1MB", |b| {
        b.iter_batched(
            || {
                counter += 1;
                (make_key(counter), value.clone())
            },
            |(key, val)| {
                engine.put(key, &val, Codec::Zstd).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_read_hot(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    // Pre-populate: 10 keys
    let value = make_value(4096);
    for i in 0..10u64 {
        engine
            .put(make_key(i), &value, Codec::Zstd)
            .unwrap();
    }

    let mut counter = 0u64;
    group.bench_function("hot", |b| {
        b.iter(|| {
            let key = make_key(counter % 10);
            counter += 1;
            std::hint::black_box(engine.get(&key).unwrap());
        })
    });
    group.finish();
}

pub fn bench_read_cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let value = make_value(4096);
    // Write 5000 keys across all 256 buckets — mmap page faults will occur
    for i in 0..5000u64 {
        engine
            .put(make_key(i), &value, Codec::Zstd)
            .unwrap();
    }

    let mut counter = 0u64;
    group.bench_function("cold", |b| {
        b.iter(|| {
            let key = make_key(counter % 5000);
            counter += 1;
            std::hint::black_box(engine.get(&key).unwrap());
        })
    });
    group.finish();
}

pub fn bench_read_large_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    group.throughput(Throughput::Bytes(1024 * 1024));
    group.measurement_time(Duration::from_secs(10));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    let value = make_value(1024 * 1024); // 1 MB
    for i in 0..5u64 {
        engine
            .put(make_key(i), &value, Codec::Zstd)
            .unwrap();
    }

    let mut counter = 0u64;
    group.bench_function("1MB", |b| {
        b.iter(|| {
            let key = make_key(counter % 5);
            counter += 1;
            std::hint::black_box(engine.get(&key).unwrap());
        })
    });
    group.finish();
}

pub fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("delete", |b| {
        let dir = TempDir::new().unwrap();
        let engine = Engine::open(dir.path(), Config::default()).unwrap();

        let value = make_value(4096);
        let mut counter = 0u64;

        b.iter_batched(
            || {
                counter += 1;
                let key = make_key(counter);
                engine.put(key, &value, Codec::Zstd).unwrap();
                key
            },
            |key| {
                engine.delete(&key).unwrap();
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

pub fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(15));

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path(), Config::default()).unwrap();

    // Pre-populate with 500 entries
    let value = make_value(8192);
    for i in 0..500u64 {
        engine
            .put(make_key(i), &value, Codec::Zstd)
            .unwrap();
    }

    let mut counter: u64 = 500;
    group.bench_function("80w_15r_5d", |b| {
        b.iter(|| {
            counter += 1;
            let op = counter % 100;
            match op {
                0..=79 => {
                    let key = make_key(counter);
                    let val = make_value(4096);
                    engine.put(key, &val, Codec::Zstd).unwrap();
                }
                80..=94 => {
                    std::hint::black_box(
                        engine.get(&make_key(counter % 500)).unwrap(),
                    );
                }
                _ => {
                    if counter % 2 == 0 {
                        let key = make_key(counter % 500);
                        let _ = engine.delete(&key);
                    }
                }
            }
        })
    });
    group.finish();
}

pub fn bench_gc(c: &mut Criterion) {
    let mut group = c.benchmark_group("gc");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    group.bench_function("gc_30pct_deleted", |b| {
        let dir = TempDir::new().unwrap();
        let engine = Engine::open(dir.path(), Config::default()).unwrap();

        let value = make_value(200_000);
        let n = 1200u64;
        for i in 0..n {
            engine
                .put(make_key(i), &value, Codec::None)
                .unwrap();
        }
        for i in (0..n).step_by(3) {
            engine.delete(&make_key(i)).unwrap();
        }

        b.iter(|| {
            engine.gc().unwrap();
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_write_small,
    bench_write_medium,
    bench_write_large,
    bench_read_hot,
    bench_read_cold,
    bench_read_large_value,
    bench_delete,
    bench_mixed_workload,
    bench_gc,
);
criterion_main!(benches);
