//! Scanline thick line from the four corners of the perpendicular box.
//!
//! A chord `A→B` of width `wd` is the convex quad whose ends are the Murphy
//! spokes at `A` and `B`. [`ThickLineFill`] walks the two y-monotonic chains
//! from the lowest corner and feeds them to [`crate::Spanner`].
//! [`ThickLine`] is the four [`Line`]s around that quad. [`ThickLineAa`] is the
//! same walk with [`crate::LineAa`].

use crate::inclusive::Inclusive;
use crate::line::Line;
#[cfg(feature = "aa")]
use crate::line_aa::LineAa;
use crate::AndMap;
use crate::Point;
#[cfg(feature = "aa")]
use crate::PointAa;
use crate::Span;
use crate::Spanner;

fn offset(p: Point, dir: Point, steps: isize) -> Point {
    if steps <= 0 || (dir.0 == 0 && dir.1 == 0) {
        return p;
    }
    let target = (p.0 + dir.0, p.1 + dir.1);
    Line::new(p, target).nth(steps as usize).unwrap_or(target)
}

/// Corners `a, b, c, d` in cycle order: spoke at `start`, then spoke at `end`.
///
/// ```text
///            a                 d
///             +---------------+
///            /                 \
///      start●-------------------●end
///            \                 /
///             +---------------+
///            b                 c
/// ```
///
/// `a` and `d` are the `(dy, -dx)` offsets; `b` and `c` are `(-dy, dx)`.
fn corners(start: Point, end: Point, wd: f32) -> [Point; 4] {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let half = (wd.abs() / 2.0) as isize;
    [
        offset(start, (dy, -dx), half),
        offset(start, (-dy, dx), half),
        offset(end, (-dy, dx), half),
        offset(end, (dy, -dx), half),
    ]
}

fn idx_min(v: &[Point; 4]) -> usize {
    let mut i = 0;
    for j in 1..4 {
        if (v[j].1, v[j].0) < (v[i].1, v[i].0) {
            i = j;
        }
    }
    i
}

fn idx_max(v: &[Point; 4]) -> usize {
    let mut i = 0;
    for j in 1..4 {
        if (v[j].1, v[j].0) > (v[i].1, v[i].0) {
            i = j;
        }
    }
    i
}

/// Corners `a, b, c, d` in cycle order, rotated so `a` is min and `c` is max.
///
/// `a` is the least `(y, x)` (bottom-left); `c` is the greatest (top-right).
///
/// ```text
///            d                 c
///             +---------------+
///            /           max /
///           /               /
///          +---------------+
///         a                 b
///       min
/// ```
fn corners_min_max(start: Point, end: Point, wd: f32) -> [Point; 4] {
    let v = corners(start, end, wd);
    let i = idx_min(&v);
    [v[i], v[(i + 1) % 4], v[(i + 2) % 4], v[(i + 3) % 4]]
}

/// Up to two edges from `from` to `to` walking `dir` (±1) around the cycle.
fn chain_edges(
    v: &[Point; 4],
    from: usize,
    to: usize,
    dir: isize,
) -> (Option<(Point, Point)>, Option<(Point, Point)>) {
    if from == to {
        return (None, None);
    }
    let mut e0 = None;
    let mut e1 = None;
    let mut i = from;
    loop {
        let j = (i as isize + dir).rem_euclid(4) as usize;
        if v[i] != v[j] {
            let edge = (v[i], v[j]);
            if e0.is_none() {
                e0 = Some(edge);
            } else {
                e1 = Some(edge);
            }
        }
        i = j;
        if i == to {
            break;
        }
    }
    (e0, e1)
}

fn walk(
    e0: Option<(Point, Point)>,
    e1: Option<(Point, Point)>,
    fallback: Point,
) -> impl Iterator<Item = Point> {
    let fallback = (e0.is_none() && e1.is_none()).then_some(fallback);
    e0.into_iter()
        .chain(e1)
        .enumerate()
        .flat_map(|(i, (s, e))| Line::new(s, e).inclusive().skip((i != 0) as usize))
        .chain(fallback)
}

/// Inclusive filled thick line (`[start, end]`) of width `wd`.
///
/// Solid horizontal spans of the perpendicular box, via [`Spanner`].
pub struct ThickLineFill;

impl ThickLineFill {
    /// Inclusive thick line (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> impl Iterator<Item = Span> {
        let [a, b, c, d] = corners_min_max(start, end, wd);
        Spanner::new(
            Line::new(a, b).chain(Line::new(b, c).inclusive()),
            Line::new(a, d).chain(Line::new(d, c).inclusive()),
        )
    }
}

/// Outline of the perpendicular parallelogram.
///
/// Walks `a→b` and emits each point plus `(d - a)` (the parallel `d→c`).
/// Then walks `a→d` and emits each point plus `(b - a)` (the parallel `b→c`).
pub struct ThickLine;

impl ThickLine {
    /// Thick-line outline (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> impl Iterator<Item = Point> {
        let [a, b, c, d] = corners(start, end, wd);
        let da = (d.0 - a.0, d.1 - a.1);
        let ab = (a.0 - b.0, a.1 - b.1);
        // To avoid any overdraw, a-b is inclusive...
        Line::new(a, b)
            .inclusive()
            .and_map(move |p| (p.0 + da.0, p.1 + da.1))
            .chain(
                // ...and b-c is exclusive of its endpoints.
                Line::new(b, c)
                    .skip(1)
                    .and_map(move |p| (p.0 + ab.0, p.1 + ab.1)),
            )
    }
}

/// Anti-aliased outline of the perpendicular parallelogram.
///
/// Same two-edge walk as [`ThickLine`], using [`LineAa`].
#[cfg(feature = "aa")]
#[cfg_attr(docsrs, doc(cfg(feature = "aa")))]
pub struct ThickLineAa;

#[cfg(feature = "aa")]
impl ThickLineAa {
    /// Inclusive anti-aliased thick-line outline (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> impl Iterator<Item = PointAa> {
        let [a, b, c, d] = corners(start, end, wd);
        let da = (d.0 - a.0, d.1 - a.1);
        let ab = (a.0 - b.0, a.1 - b.1);
        LineAa::new(a, b)
            .and_map(move |(p, cov)| ((p.0 + da.0, p.1 + da.1), cov))
            .chain(
                // This overdraws on point C and D unfortunately.
                LineAa::new(b, c)
                    .skip(1)
                    .and_map(move |(p, cov)| ((p.0 + ab.0, p.1 + ab.1), cov)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{corners, corners_min_max, idx_max, idx_min, ThickLine, ThickLineFill};
    use crate::line::Line;
    use crate::Point;
    use std::collections::BTreeSet;
    use std::vec::Vec;

    #[test]
    fn corners_min_max_puts_extrema_at_a_and_c() {
        for (start, end, wd) in [
            ((0, 3), (7, 3), 3.0),
            ((3, 0), (3, 7), 3.0),
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 0), (7, 7), 1.0),
        ] {
            let v = corners(start, end, wd);
            let [a, b, c, d] = corners_min_max(start, end, wd);
            assert_eq!(a, v[idx_min(&v)], "{start:?}->{end:?}");
            assert_eq!(c, v[idx_max(&v)], "{start:?}->{end:?}");
            let mut got = [a, b, c, d];
            let mut raw = v;
            got.sort_unstable();
            raw.sort_unstable();
            assert_eq!(got, raw, "{start:?}->{end:?}");
        }
    }

    fn expand(spans: impl Iterator<Item = crate::Span>) -> impl Iterator<Item = Point> {
        spans.flat_map(|s| (s.x0..=s.x1).map(move |x| (x, s.y)))
    }

    fn plot_binary(points: impl Iterator<Item = Point>) -> [u8; 8] {
        let mut grid = [0u8; 8];
        for (x, y) in points {
            if !(0..8).contains(&x) || !(0..8).contains(&y) {
                continue;
            }
            grid[y as usize] |= 0x80 >> x;
        }
        grid
    }

    fn sandwiched_holes(fill: &BTreeSet<Point>) -> BTreeSet<Point> {
        let mut pts = fill.iter().copied();
        let Some((x0, y0)) = pts.next() else {
            return BTreeSet::new();
        };
        let (mut minx, mut maxx, mut miny, mut maxy) = (x0, x0, y0, y0);
        for (x, y) in pts {
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
        }
        let mut holes = BTreeSet::new();
        for y in miny..=maxy {
            for x in minx..=maxx {
                let p = (x, y);
                if fill.contains(&p) {
                    continue;
                }
                let lr = fill.contains(&(x - 1, y)) && fill.contains(&(x + 1, y));
                let ud = fill.contains(&(x, y - 1)) && fill.contains(&(x, y + 1));
                if lr || ud {
                    holes.insert(p);
                }
            }
        }
        holes
    }

    #[test]
    fn width_one_is_inclusive_line() {
        for (start, end) in [((0, 3), (7, 3)), ((0, 0), (7, 7)), ((0, 7), (7, 0))] {
            let fill: BTreeSet<_> = expand(ThickLineFill::new(start, end, 1.0)).collect();
            let mut line: BTreeSet<_> = Line::new(start, end).collect();
            line.insert(end);
            assert_eq!(fill, line, "{start:?}->{end:?}");
        }
    }

    #[test]
    fn degenerate_is_the_point() {
        let fill: Vec<_> = expand(ThickLineFill::new((3, 4), (3, 4), 3.0)).collect();
        assert_eq!(fill, [(3, 4)]);
    }

    #[test]
    fn test_thick_line_fill_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(expand(ThickLineFill::new((0, 3), (7, 3), 3.0))), [
            0b00000000,
            0b00000000,
            0b11111111,
            0b11111111,
            0b11111111,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_fill_shape_vertical() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(expand(ThickLineFill::new((3, 0), (3, 7), 3.0))), [
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
        ]);
    }

    #[test]
    fn test_thick_line_fill_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(expand(ThickLineFill::new((0, 0), (5, 2), 3.0))), [
            0b11110000,
            0b11111100,
            0b00111100,
            0b00001100,
            0b00000000,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_fill_shape_diagonal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(expand(ThickLineFill::new((0, 0), (7, 7), 3.0))), [
            0b11100000,
            0b11110000,
            0b11111000,
            0b01111100,
            0b00111110,
            0b00011111,
            0b00001111,
            0b00000111,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_binary(expand(ThickLineFill::new((0, 7), (7, 0), 3.0))), [
            0b00000111,
            0b00001111,
            0b00011111,
            0b00111110,
            0b01111100,
            0b11111000,
            0b11110000,
            0b11100000,
        ]);
    }

    #[test]
    fn fill_has_no_regular_holes() {
        for (start, end, wd) in [
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 3), (7, 3), 3.0),
        ] {
            let fill: BTreeSet<_> = expand(ThickLineFill::new(start, end, wd)).collect();
            assert!(
                sandwiched_holes(&fill).is_empty(),
                "{start:?}->{end:?} wd={wd}"
            );
        }
    }

    #[test]
    fn test_thick_line_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 3), (7, 3), 3.0)), [
            0b00000000,
            0b00000000,
            0b11111111,
            0b10000001,
            0b11111111,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_shape_vertical() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((3, 0), (3, 7), 3.0)), [
            0b00111000,
            0b00101000,
            0b00101000,
            0b00101000,
            0b00101000,
            0b00101000,
            0b00101000,
            0b00111000,
        ]);
    }

    #[test]
    fn test_thick_line_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 0), (5, 2), 3.0)), [
            0b10110000,
            0b11001100,
            0b00110100,
            0b00001100,
            0b00000000,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_shape_diagonal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 0), (7, 7), 3.0)), [
            0b10100000,
            0b00010000,
            0b10001000,
            0b01000100,
            0b00100010,
            0b00010001,
            0b00001000,
            0b00000101,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 7), (7, 0), 3.0)), [
            0b00000101,
            0b00001000,
            0b00010001,
            0b00100010,
            0b01000100,
            0b10001000,
            0b00010000,
            0b10100000,
        ]);
    }

    #[test]
    fn outline_is_the_four_edges() {
        for (start, end, wd) in [
            ((0, 3), (7, 3), 3.0),
            ((3, 0), (3, 7), 3.0),
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 0), (7, 7), 1.0),
        ] {
            let v = corners(start, end, wd);
            let mut edges: BTreeSet<Point> = BTreeSet::new();
            for i in 0..4 {
                let a = v[i];
                let b = v[(i + 1) % 4];
                if a == b {
                    continue;
                }
                edges.extend(Line::new(a, b));
            }
            let outline: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            assert_eq!(outline, edges, "{start:?}->{end:?} wd={wd}");
        }
    }

    #[test]
    fn outline_is_subset_of_fill() {
        for (start, end, wd) in [
            ((0, 3), (7, 3), 3.0),
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((4, 4), (4, 4), 3.0),
        ] {
            let fill: BTreeSet<_> = expand(ThickLineFill::new(start, end, wd)).collect();
            let outline: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            assert!(outline.is_subset(&fill), "{start:?}->{end:?} wd={wd}");
        }
    }

    #[cfg(feature = "murphy")]
    #[test]
    fn fill_matches_murphy() {
        use crate::murphy::ThickLineFill as Murphy;
        for (start, end, wd) in [
            ((0, 3), (7, 3), 1.0),
            ((0, 3), (7, 3), 3.0),
            ((3, 0), (3, 7), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 0), (7, 7), 1.0),
            ((4, 4), (4, 4), 3.0),
        ] {
            let celis: BTreeSet<_> = expand(ThickLineFill::new(start, end, wd)).collect();
            let murphy: BTreeSet<_> = Murphy::new(start, end, wd).collect();
            assert_eq!(celis, murphy, "{start:?}->{end:?} wd={wd}");
        }
    }

    #[cfg(feature = "murphy")]
    #[test]
    fn outline_vs_murphy() {
        use crate::murphy::ThickLine as Murphy;
        for (start, end, wd) in [
            ((0, 3), (7, 3), 1.0),
            ((0, 3), (7, 3), 3.0),
            ((3, 0), (3, 7), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 0), (7, 7), 1.0),
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
        ] {
            let celis: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            let murphy: BTreeSet<_> = Murphy::new(start, end, wd).collect();
            if wd == 1.0 || start.1 == end.1 || start.0 == end.0 {
                assert_eq!(celis, murphy, "{start:?}->{end:?} wd={wd}");
            } else {
                assert!(
                    !celis.is_empty() && !murphy.is_empty(),
                    "{start:?}->{end:?}"
                );
            }
        }
    }

    /// Murphy's double-square extras stick out of the four-corner box on 45°.
    #[cfg(feature = "murphy")]
    #[test]
    fn diagonal_fill_is_subset_of_murphy() {
        use crate::murphy::ThickLineFill as Murphy;
        for (start, end) in [((0, 0), (7, 7)), ((0, 7), (7, 0)), ((7, 7), (0, 0))] {
            let celis: BTreeSet<_> = expand(ThickLineFill::new(start, end, 3.0)).collect();
            let murphy: BTreeSet<_> = Murphy::new(start, end, 3.0).collect();
            assert!(
                celis.is_subset(&murphy),
                "{start:?}->{end:?} celis not in murphy"
            );
            assert!(
                !celis.is_empty() && celis != murphy,
                "{start:?}->{end:?} expected Murphy extras outside the box"
            );
        }
    }

    #[cfg(feature = "aa")]
    fn plot_hex(points: impl Iterator<Item = crate::PointAa>) -> [u32; 8] {
        let mut grid = [0u32; 8];
        for ((x, y), c) in points {
            if !(0..8).contains(&x) || !(0..8).contains(&y) {
                continue;
            }
            let shift = ((7 - x) * 4) as u32;
            let nyb = (c as u32) >> 4;
            let old = (grid[y as usize] >> shift) & 0xf;
            grid[y as usize] = (grid[y as usize] & !(0xf << shift)) | (old.max(nyb) << shift);
        }
        grid
    }

    #[cfg(feature = "aa")]
    #[test]
    fn aa_covers_hard_outline() {
        use super::ThickLineAa;
        for (start, end, wd) in [
            ((0, 3), (7, 3), 3.0),
            ((3, 0), (3, 7), 3.0),
            ((0, 0), (5, 2), 3.0),
            ((0, 0), (7, 7), 3.0),
            ((0, 7), (7, 0), 3.0),
            ((0, 0), (7, 7), 1.0),
            ((4, 4), (4, 4), 3.0),
        ] {
            let hard: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            let aa: BTreeSet<_> = ThickLineAa::new(start, end, wd).map(|(p, _)| p).collect();
            assert!(hard.is_subset(&aa), "{start:?}->{end:?} wd={wd}");
        }
    }

    #[cfg(feature = "aa")]
    #[test]
    fn test_thick_line_aa_shape_horizontal() {
        use super::ThickLineAa;
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 3), (7, 3), 3.0)), [
            0x00000000,
            0x00000000,
            0xffffffff,
            0xf000000f,
            0xffffffff,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 3), (7, 3), 1.0)), [
            0x00000000,
            0x00000000,
            0x00000000,
            0xffffffff,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
    }

    #[cfg(feature = "aa")]
    #[test]
    fn test_thick_line_aa_shape_vertical() {
        use super::ThickLineAa;
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((3, 0), (3, 7), 3.0)), [
            0x00fff000,
            0x00f0f000,
            0x00f0f000,
            0x00f0f000,
            0x00f0f000,
            0x00f0f000,
            0x00f0f000,
            0x00fff000,
        ]);
    }

    #[cfg(feature = "aa")]
    #[test]
    fn test_thick_line_aa_shape_shallow() {
        use super::ThickLineAa;
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 0), (5, 2), 3.0)), [
            0xf6cc6000,
            0xf9339f00,
            0x06cc6f00,
            0x00039f00,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
    }

    #[cfg(feature = "aa")]
    #[test]
    fn test_thick_line_aa_shape_diagonal() {
        use super::ThickLineAa;
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 0), (7, 7), 3.0)), [
            0xf3f30000,
            0x303f3000,
            0xf303f300,
            0x3f303f30,
            0x03f303f3,
            0x003f303f,
            0x0003f303,
            0x00003f3f,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 7), (7, 0), 3.0)), [
            0x00003f3f,
            0x0003f303,
            0x003f303f,
            0x03f303f3,
            0x3f303f30,
            0xf303f300,
            0x303f3000,
            0xf3f30000,
        ]);
    }
}
