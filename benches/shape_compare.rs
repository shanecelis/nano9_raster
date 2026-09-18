use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use nano9_raster::{
    Circle, CircleAa, Ellipse, EllipseAa, Fill, Plot, Point, PointAa, PointIteratorExt, QuadArc,
    RoundRect, RoundRect2, Span,
};

#[inline]
fn mix(acc: usize, x: isize, y: isize, alpha: u8) -> usize {
    acc.rotate_left(7)
        ^ (x as usize).wrapping_mul(0x9e37_79b1)
        ^ (y as usize).wrapping_mul(0x85eb_ca77)
        ^ usize::from(alpha)
}

fn consume_points(points: impl Iterator<Item = Point>) -> usize {
    black_box(points.fold(0, |acc, (x, y)| mix(acc, x, y, 255)))
}

fn consume_points_aa(points: impl Iterator<Item = PointAa>) -> usize {
    black_box(points.fold(0, |acc, ((x, y), alpha)| mix(acc, x, y, alpha)))
}

fn consume_spans(spans: impl Iterator<Item = Span>) -> usize {
    black_box(spans.fold(0, |acc, span| {
        mix(mix(acc, span.x0, span.y, 255), span.x1, span.y, 255)
    }))
}

fn consume_plots(plots: impl Iterator<Item = Plot>) -> usize {
    black_box(plots.fold(0, |acc, plot| match plot {
        Plot::Point(((x, y), alpha)) => mix(acc, x, y, alpha),
        Plot::Span(span) => mix(mix(acc, span.x0, span.y, 255), span.x1, span.y, 255),
    }))
}

fn shape_compare(c: &mut Criterion) {
    for radius in [8isize, 32, 128] {
        let mut group = c.benchmark_group("outline");
        group.bench_with_input(BenchmarkId::new("circle", radius), &radius, |b, &r| {
            b.iter(|| consume_points(Circle::new((0, 0), black_box(r))))
        });
        group.bench_with_input(BenchmarkId::new("ellipse", radius), &radius, |b, &r| {
            b.iter(|| consume_points(Ellipse::new((0, 0), black_box(r), black_box(r))))
        });
        group.finish();

        let mut group = c.benchmark_group("fill");
        group.bench_with_input(BenchmarkId::new("circle", radius), &radius, |b, &r| {
            b.iter(|| consume_spans(Circle::new((0, 0), black_box(r)).fill()))
        });
        group.bench_with_input(BenchmarkId::new("ellipse", radius), &radius, |b, &r| {
            b.iter(|| consume_spans(Ellipse::new((0, 0), black_box(r), black_box(r)).fill()))
        });
        group.finish();

        let mut group = c.benchmark_group("outline_aa");
        group.bench_with_input(BenchmarkId::new("circle", radius), &radius, |b, &r| {
            b.iter(|| consume_points_aa(CircleAa::new((0, 0), black_box(r))))
        });
        group.bench_with_input(BenchmarkId::new("ellipse", radius), &radius, |b, &r| {
            b.iter(|| consume_points_aa(EllipseAa::new((0, 0), black_box(r), black_box(r))))
        });
        group.finish();

        let mut group = c.benchmark_group("fill_aa");
        group.bench_with_input(BenchmarkId::new("circle", radius), &radius, |b, &r| {
            b.iter(|| consume_plots(CircleAa::new((0, 0), black_box(r)).fill()))
        });
        group.bench_with_input(BenchmarkId::new("ellipse", radius), &radius, |b, &r| {
            b.iter(|| consume_plots(EllipseAa::new((0, 0), black_box(r), black_box(r)).fill()))
        });
        group.finish();
    }
}

fn ellipse_reflection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ellipse_reflection");
    for (a, b) in [(8isize, 4isize), (32, 16), (128, 64)] {
        let dimensions = format!("{a}x{b}");
        group.bench_with_input(
            BenchmarkId::new("outline", &dimensions),
            &(a, b),
            |bencher, &(a, b)| {
                bencher.iter(|| consume_points(Ellipse::new((0, 0), black_box(a), black_box(b))))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("fill", &dimensions),
            &(a, b),
            |bencher, &(a, b)| {
                bencher
                    .iter(|| consume_spans(Ellipse::new((0, 0), black_box(a), black_box(b)).fill()))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("outline_aa", &dimensions),
            &(a, b),
            |bencher, &(a, b)| {
                bencher
                    .iter(|| consume_points_aa(EllipseAa::new((0, 0), black_box(a), black_box(b))))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("fill_aa", &dimensions),
            &(a, b),
            |bencher, &(a, b)| {
                bencher.iter(|| {
                    consume_plots(EllipseAa::new((0, 0), black_box(a), black_box(b)).fill())
                })
            },
        );
    }
    group.finish();
}

fn circle_decomposition(c: &mut Criterion) {
    let mut group = c.benchmark_group("circle_decomposition");
    for radius in [8isize, 32, 128] {
        group.bench_with_input(BenchmarkId::new("baseline", radius), &radius, |b, &r| {
            b.iter(|| consume_points(Circle::new((13, -7), black_box(r))))
        });
        group.bench_with_input(BenchmarkId::new("composed", radius), &radius, |b, &r| {
            b.iter(|| {
                consume_points(
                    QuadArc::new(black_box(r))
                        .reflect_x()
                        .reflect_y()
                        .translate(13, -7),
                )
            })
        });
    }
    group.finish();
}

fn round_rect_decomposition(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_rect_decomposition");
    for &(width, height, radius) in &[(32isize, 24isize, 4isize), (128, 96, 16), (512, 384, 64)] {
        let dimensions = format!("{width}x{height}/r{radius}");
        group.bench_with_input(
            BenchmarkId::new("baseline", &dimensions),
            &(width, height, radius),
            |b, &(w, h, r)| {
                b.iter(|| {
                    consume_points(RoundRect::new(
                        (13, -7),
                        (13 + black_box(w) - 1, -7 + black_box(h) - 1),
                        black_box(r),
                    ))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("quad_arc", &dimensions),
            &(width, height, radius),
            |b, &(w, h, r)| {
                b.iter(|| {
                    consume_points(RoundRect2::new(
                        (13, -7),
                        (13 + black_box(w) - 1, -7 + black_box(h) - 1),
                        black_box(r),
                    ))
                })
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    targets = shape_compare, ellipse_reflection, circle_decomposition, round_rect_decomposition
}
criterion_main!(benches);
