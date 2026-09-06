//! Scanline thick line from the four corners of the perpendicular box.
//!
//! A chord `A→B` of width `wd` is the convex quad whose ends are the Murphy
//! spokes at `A` and `B`. [`ThickLineFill`] walks the two y-monotonic chains
//! from the lowest corner and emits a solid horizontal span on each row.
//! [`ThickLine`] is the four [`Line`]s around that quad.

use crate::line::Line;
use crate::inclusive::Inclusive;
use crate::AndMap;
use crate::Point;

fn offset(p: Point, dir: Point, steps: isize) -> Point {
    if steps <= 0 || (dir.0 == 0 && dir.1 == 0) {
        return p;
    }
    let target = (p.0 + dir.0, p.1 + dir.1);
    Line::new(p, target).nth(steps as usize).unwrap_or(target)
}

/// Corners `a, b, c, d` in cycle order: spoke at `start`, then spoke at `end`.
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

struct Chain {
    line: Line,
    end: Point,
    pending_end: bool,
    next: Option<(Point, Point)>,
    peek: Option<Point>,
}

impl Chain {
    fn empty() -> Self {
        Chain {
            line: Line::new((0, 0), (0, 0)),
            end: (0, 0),
            pending_end: false,
            next: None,
            peek: None,
        }
    }

    fn new(first: Option<(Point, Point)>, second: Option<(Point, Point)>) -> Self {
        let Some((s, e)) = first else {
            return Chain::empty();
        };
        let mut chain = Chain {
            line: Line::new(s, e),
            end: e,
            pending_end: true,
            next: second,
            peek: None,
        };
        chain.pull();
        chain
    }

    fn pull(&mut self) {
        if self.peek.is_some() {
            return;
        }
        if let Some(p) = self.line.next() {
            self.peek = Some(p);
            return;
        }
        if self.pending_end {
            self.pending_end = false;
            self.peek = Some(self.end);
            return;
        }
        if let Some((s, e)) = self.next.take() {
            self.line = Line::new(s, e);
            self.end = e;
            self.pending_end = true;
            let _ = self.line.next();
            self.pull();
        }
    }

    fn peek(&mut self) -> Option<Point> {
        self.pull();
        self.peek
    }

    fn pop(&mut self) -> Option<Point> {
        self.pull();
        self.peek.take()
    }

    fn absorb_row(&mut self, y: isize, lo: &mut isize, hi: &mut isize) {
        while let Some((x, py)) = self.peek() {
            if py != y {
                break;
            }
            let _ = self.pop();
            if x < *lo {
                *lo = x;
            }
            if x > *hi {
                *hi = x;
            }
        }
    }
}

/// Inclusive filled thick line (`[start, end]`) of width `wd`.
///
/// Solid horizontal spans of the perpendicular box.
pub struct ThickLineFill {
    left: Chain,
    right: Chain,
    y: isize,
    y_max: isize,
    x: isize,
    x_hi: isize,
    started: bool,
    done: bool,
}

impl ThickLineFill {
    /// Inclusive thick line (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> Self {
        let v = corners(start, end, wd);
        let imin = idx_min(&v);
        let imax = idx_max(&v);
        let (l0, l1) = chain_edges(&v, imin, imax, 1);
        let (r0, r1) = chain_edges(&v, imin, imax, -1);
        let y = v[imin].1;
        let y_max = v[imax].1;
        let single = l0.is_none() && r0.is_none();
        ThickLineFill {
            left: Chain::new(l0, l1),
            right: Chain::new(r0, r1),
            y,
            y_max,
            x: if single { v[imin].0 } else { 0 },
            x_hi: if single { v[imin].0 } else { -1 },
            started: single,
            done: false,
        }
    }

    fn enter_row(&mut self) -> bool {
        if self.started {
            if self.y == self.y_max {
                return false;
            }
            self.y += 1;
        } else {
            self.started = true;
        }
        let mut lo = isize::MAX;
        let mut hi = isize::MIN;
        self.left.absorb_row(self.y, &mut lo, &mut hi);
        self.right.absorb_row(self.y, &mut lo, &mut hi);
        if lo > hi {
            self.x = 0;
            self.x_hi = -1;
        } else {
            self.x = lo;
            self.x_hi = hi;
        }
        true
    }
}

impl Iterator for ThickLineFill {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            if self.x <= self.x_hi {
                let p = (self.x, self.y);
                self.x += 1;
                return Some(p);
            }
            if !self.enter_row() {
                self.done = true;
                return None;
            }
        }
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
        Line::new(a, b)
            .and_map(move |p| (p.0 + da.0, p.1 + da.1))
            .chain(
                Line::new(b, c)
                    .inclusive()
                    .and_map(move |p| (p.0 + ab.0, p.1 + ab.1)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{corners, ThickLine, ThickLineFill};
    use crate::line::Line;
    use crate::Point;
    use std::collections::BTreeSet;
    use std::vec::Vec;

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
            let fill: BTreeSet<_> = ThickLineFill::new(start, end, 1.0).collect();
            let mut line: BTreeSet<_> = Line::new(start, end).collect();
            line.insert(end);
            assert_eq!(fill, line, "{start:?}->{end:?}");
        }
    }

    #[test]
    fn degenerate_is_the_point() {
        let fill: Vec<_> = ThickLineFill::new((3, 4), (3, 4), 3.0).collect();
        assert_eq!(fill, [(3, 4)]);
    }

    #[test]
    fn test_thick_line_fill_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 3), (7, 3), 3.0)), [
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
        assert_eq!(plot_binary(ThickLineFill::new((3, 0), (3, 7), 3.0)), [
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
        assert_eq!(plot_binary(ThickLineFill::new((0, 0), (5, 2), 3.0)), [
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
        assert_eq!(plot_binary(ThickLineFill::new((0, 0), (7, 7), 3.0)), [
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
        assert_eq!(plot_binary(ThickLineFill::new((0, 7), (7, 0), 3.0)), [
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
            let fill: BTreeSet<_> = ThickLineFill::new(start, end, wd).collect();
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
            let fill: BTreeSet<_> = ThickLineFill::new(start, end, wd).collect();
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
            let celis: BTreeSet<_> = ThickLineFill::new(start, end, wd).collect();
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
            let celis: BTreeSet<_> = ThickLineFill::new(start, end, 3.0).collect();
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
}
