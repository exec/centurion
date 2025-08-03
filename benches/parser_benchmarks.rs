use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_parser(c: &mut Criterion) {
    c.bench_function("parse message", |b| {
        b.iter(|| {
            // TODO: Add actual benchmarks
        })
    });
}

criterion_group!(benches, benchmark_parser);
criterion_main!(benches);