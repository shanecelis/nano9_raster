use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nano9_raster::celis::{ThickLine as CelisOutline, ThickLineFill as Celis};
use nano9_raster::murphy::{ThickLine as MurphyOutline, ThickLineFill as Murphy};
use nano9_raster::ThickLineFill as Zingl;

fn fold_zingl(start: (isize, isize), end: (isize, isize), wd: f32) -> u64 {
    let mut acc = 0u64;
    for (x, y) in Zingl::new(start, end, wd) {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
    }
    acc
}

fn fold_murphy(start: (isize, isize), end: (isize, isize), wd: f32) -> u64 {
    let mut acc = 0u64;
    for (x, y) in Murphy::new(start, end, wd) {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
    }
    acc
}

fn fold_celis(start: (isize, isize), end: (isize, isize), wd: f32) -> u64 {
    let mut acc = 0u64;
    for s in Celis::new(start, end, wd) {
        for x in s.x0..=s.x1 {
            acc ^= x as u64;
            acc ^= (s.y as u64).wrapping_mul(0x9e37_79b9);
        }
    }
    acc
}

fn fold_celis_outline(start: (isize, isize), end: (isize, isize), wd: f32) -> u64 {
    let mut acc = 0u64;
    for (x, y) in CelisOutline::new(start, end, wd) {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
    }
    acc
}

fn fold_murphy_outline(start: (isize, isize), end: (isize, isize), wd: f32) -> u64 {
    let mut acc = 0u64;
    for (x, y) in MurphyOutline::new(start, end, wd) {
        acc ^= x as u64;
        acc ^= (y as u64).wrapping_mul(0x9e37_79b9);
    }
    acc
}

fn bench_thick_line_fill(c: &mut Criterion) {
    let cases = [
        ("horizontal", (0, 0), (1000, 0)),
        ("shallow", (0, 0), (1000, 200)),
        ("diagonal", (0, 0), (1000, 1000)),
        ("steep", (0, 0), (200, 1000)),
    ];
    let widths = [1.0f32, 3.0];

    let mut group = c.benchmark_group("thick_line_fill");
    for (name, start, end) in cases {
        for wd in widths {
            let w = wd as u32;
            group.bench_function(format!("zingl/{name}/{w}"), |b| {
                b.iter(|| fold_zingl(black_box(start), black_box(end), black_box(wd)))
            });
            group.bench_function(format!("murphy/{name}/{w}"), |b| {
                b.iter(|| fold_murphy(black_box(start), black_box(end), black_box(wd)))
            });
            group.bench_function(format!("celis/{name}/{w}"), |b| {
                b.iter(|| fold_celis(black_box(start), black_box(end), black_box(wd)))
            });
        }
    }
    group.finish();
}

fn bench_thick_line(c: &mut Criterion) {
    let cases = [
        ("horizontal", (0, 0), (1000, 0)),
        ("shallow", (0, 0), (1000, 200)),
        ("diagonal", (0, 0), (1000, 1000)),
        ("steep", (0, 0), (200, 1000)),
    ];
    let widths = [1.0f32, 3.0];

    let mut group = c.benchmark_group("thick_line");
    for (name, start, end) in cases {
        for wd in widths {
            let w = wd as u32;
            group.bench_function(format!("murphy/{name}/{w}"), |b| {
                b.iter(|| fold_murphy_outline(black_box(start), black_box(end), black_box(wd)))
            });
            group.bench_function(format!("celis/{name}/{w}"), |b| {
                b.iter(|| fold_celis_outline(black_box(start), black_box(end), black_box(wd)))
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_thick_line_fill, bench_thick_line);
criterion_main!(benches);
