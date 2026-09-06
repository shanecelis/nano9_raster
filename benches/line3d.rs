use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nano9_raster::{Inclusive, Line3d};

fn fold_half_open(start: (isize, isize, isize), end: (isize, isize, isize)) -> u64 {
    let mut acc = 0u64;
    for (x, y, z) in Line3d::new(start, end) {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
        acc ^= (z as u64).wrapping_mul(0x85eb_ca77);
    }
    acc
}

fn fold_inclusive(start: (isize, isize, isize), end: (isize, isize, isize)) -> u64 {
    let mut acc = 0u64;
    for (x, y, z) in Line3d::new(start, end).inclusive() {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
        acc ^= (z as u64).wrapping_mul(0x85eb_ca77);
    }
    acc
}

fn bench_line3d(c: &mut Criterion) {
    let cases = [
        ("axis", (0, 0, 0), (1000, 0, 0)),
        ("shallow", (0, 0, 0), (1000, 200, 50)),
        ("diagonal", (0, 0, 0), (1000, 1000, 1000)),
        ("steep", (0, 0, 0), (200, 400, 1000)),
    ];

    let mut group = c.benchmark_group("line3d");
    for (name, start, end) in cases {
        group.bench_function(format!("half_open/{name}"), |b| {
            b.iter(|| fold_half_open(black_box(start), black_box(end)))
        });
        group.bench_function(format!("inclusive/{name}"), |b| {
            b.iter(|| fold_inclusive(black_box(start), black_box(end)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_line3d);
criterion_main!(benches);
