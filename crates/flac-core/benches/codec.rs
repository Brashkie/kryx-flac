//! Throughput benchmarks for decode and encode on a representative signal.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use kryx_flac::{decode, encode, EncodeOptions};

/// A deterministic ~1 s 44.1 kHz stereo signal: a tone plus a decorrelated
/// ramp, so both LPC and FIXED paths and stereo decorrelation get exercised.
fn make_signal(n: usize) -> Vec<Vec<i32>> {
    let l: Vec<i32> = (0..n)
        .map(|i| {
            let tone = (i as f64 * 0.06).sin() * 9000.0;
            let ramp = ((i as i32 * 5) % 1500) - 750;
            tone as i32 + ramp
        })
        .collect();
    let r: Vec<i32> = (0..n)
        .map(|i| ((i as f64 * 0.045).sin() * 7000.0) as i32)
        .collect();
    vec![l, r]
}

fn bench(c: &mut Criterion) {
    let n = 44_100;
    let channels = make_signal(n);
    let opts = EncodeOptions::new(44_100, 16);
    let flac = encode(&channels, &opts).unwrap();
    let pcm_bytes = (n * 2 * 2) as u64; // samples * channels * 2 bytes

    let mut g = c.benchmark_group("flac");
    g.throughput(Throughput::Bytes(pcm_bytes));
    g.bench_function("encode", |b| b.iter(|| encode(&channels, &opts).unwrap()));
    g.bench_function("decode", |b| b.iter(|| decode(&flac).unwrap()));
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
