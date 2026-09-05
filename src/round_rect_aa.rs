//! Anti-aliased rounded rectangle: [`CircleAa`] quarter-arcs plus solid edges.
//!
//! Coverage is the crate convention: `255` is fully on the curve, `0` is
//! fully off. Corner pixels are the matching quadrant of [`CircleAa`] at
//! each corner center. Straight edges between those arcs are fully on.

use crate::circle_aa::CircleAa;
use crate::round_rect::normalize_rect;
#[cfg(feature = "fill")]
use crate::round_rect::RoundRect;
use crate::{Point, PointAa};

#[cfg(feature = "fill")]
use crate::fill::{Fill, Plot, Span};

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn index(self) -> u8 {
        match self {
            Corner::TopLeft => 0,
            Corner::TopRight => 1,
            Corner::BottomLeft => 2,
            Corner::BottomRight => 3,
        }
    }

    fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Corner::TopLeft),
            1 => Some(Corner::TopRight),
            2 => Some(Corner::BottomLeft),
            3 => Some(Corner::BottomRight),
            _ => None,
        }
    }

    fn contains(self, center: Point, p: Point) -> bool {
        let (dx, dy) = (p.0 - center.0, p.1 - center.1);
        match self {
            Corner::TopLeft => dx <= 0 && dy <= 0,
            Corner::TopRight => dx >= 0 && dy <= 0,
            Corner::BottomLeft => dx <= 0 && dy >= 0,
            Corner::BottomRight => dx >= 0 && dy >= 0,
        }
    }
}

enum Phase {
    Corner(Corner),
    Edge(u8),
    Done,
}

/// Anti-aliased inclusive rounded rectangle
pub struct RoundRectAa {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    r: isize,
    phase: Phase,
    circle: CircleAa,
    edge_i: isize,
}

impl RoundRectAa {
    /// Inclusive anti-aliased rounded rect with opposite corners `p0` and
    /// `p1`. Radius clamping matches [`RoundRect`].
    pub fn new(p0: Point, p1: Point, r: isize) -> Self {
        let (x0, y0, x1, y1, r) = normalize_rect(p0, p1, r);
        let mut rr = RoundRectAa {
            x0,
            y0,
            x1,
            y1,
            r,
            phase: Phase::Done,
            circle: CircleAa::new((0, 0), 0),
            edge_i: 0,
        };
        if r > 0 {
            rr.start_corner(Corner::TopLeft);
        } else {
            rr.start_edge(0);
        }
        rr
    }

    fn corner_center(&self, corner: Corner) -> Point {
        match corner {
            Corner::TopLeft => (self.x0 + self.r, self.y0 + self.r),
            Corner::TopRight => (self.x1 - self.r, self.y0 + self.r),
            Corner::BottomLeft => (self.x0 + self.r, self.y1 - self.r),
            Corner::BottomRight => (self.x1 - self.r, self.y1 - self.r),
        }
    }

    fn start_corner(&mut self, corner: Corner) {
        self.phase = Phase::Corner(corner);
        self.circle = CircleAa::new(self.corner_center(corner), self.r);
    }

    fn start_edge(&mut self, edge: u8) {
        self.phase = Phase::Edge(edge);
        self.edge_i = 0;
    }

    fn edge_pixels(&self, edge: u8) -> (Point, isize) {
        let (x0, y0, x1, y1, r) = (self.x0, self.y0, self.x1, self.y1, self.r);
        if r == 0 {
            let w = x1 - x0 + 1;
            let h = y1 - y0 + 1;
            return match edge {
                0 => ((x0, y0), w),
                1 if h > 2 => ((x1, y0 + 1), h - 2),
                2 if h > 1 => ((x0, y1), w),
                3 if h > 2 && w > 1 => ((x0, y0 + 1), h - 2),
                _ => ((x0, y0), 0),
            };
        }
        match edge {
            0 => ((x0 + r + 1, y0), (x1 - r - 1) - (x0 + r + 1) + 1),
            1 => ((x1, y0 + r + 1), (y1 - r - 1) - (y0 + r + 1) + 1),
            2 => ((x0 + r + 1, y1), (x1 - r - 1) - (x0 + r + 1) + 1),
            _ => ((x0, y0 + r + 1), (y1 - r - 1) - (y0 + r + 1) + 1),
        }
    }

    fn edge_point(&self, edge: u8, i: isize) -> Point {
        let (start, _) = self.edge_pixels(edge);
        match edge {
            0 | 2 => (start.0 + i, start.1),
            1 => (start.0, start.1 + i),
            _ => (start.0, start.1 + i),
        }
    }
}

impl Iterator for RoundRectAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.phase {
                Phase::Corner(corner) => {
                    let center = self.corner_center(corner);
                    for (p, c) in self.circle.by_ref() {
                        if c > 0 && corner.contains(center, p) {
                            return Some((p, c));
                        }
                    }
                    if let Some(next) = Corner::from_index(corner.index() + 1) {
                        self.start_corner(next);
                    } else {
                        self.start_edge(0);
                    }
                }
                Phase::Edge(edge) => {
                    let (_, len) = self.edge_pixels(edge);
                    if self.edge_i < len {
                        let p = self.edge_point(edge, self.edge_i);
                        self.edge_i += 1;
                        return Some((p, 255));
                    }
                    if edge < 3 {
                        self.start_edge(edge + 1);
                    } else {
                        self.phase = Phase::Done;
                    }
                }
                Phase::Done => return None,
            }
        }
    }
}

#[cfg(feature = "fill")]
#[cfg_attr(
    docsrs,
    doc(cfg(all(feature = "aa", feature = "round-rect", feature = "fill")))
)]
impl Fill<Plot> for RoundRectAa {
    /// Filled anti-aliased rounded rect: hard [`RoundRect`] interior spans
    /// plus the outside fringe of the [`RoundRectAa`] outline.
    fn fill(self) -> impl Iterator<Item = Plot> {
        let hard = RoundRect::new((self.x0, self.y0), (self.x1, self.y1), self.r);
        RoundRectAaFill {
            spans: Some(hard.fill()),
            fringe: RoundRectAa::new((self.x0, self.y0), (self.x1, self.y1), self.r),
            hard: RoundRect::new((self.x0, self.y0), (self.x1, self.y1), self.r),
        }
    }
}

#[cfg(feature = "fill")]
struct RoundRectAaFill<S> {
    spans: Option<S>,
    fringe: RoundRectAa,
    hard: RoundRect,
}

#[cfg(feature = "fill")]
impl<S: Iterator<Item = Span>> Iterator for RoundRectAaFill<S> {
    type Item = Plot;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(spans) = self.spans.as_mut() {
            if let Some(span) = spans.next() {
                return Some(Plot::Span(span));
            }
            self.spans = None;
        }
        loop {
            let (p, c) = self.fringe.next()?;
            if c == 0 {
                continue;
            }
            let inside = self
                .hard
                .row_span(p.1)
                .is_some_and(|(x0, x1)| p.0 >= x0 && p.0 <= x1);
            if !inside {
                return Some(Plot::Point((p, c)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Corner, RoundRectAa};
    use crate::circle_aa::CircleAa;
    use crate::round_rect::RoundRect;
    use crate::PointAa;
    use std::collections::BTreeMap;

    fn blend(iter: impl Iterator<Item = PointAa>) -> BTreeMap<(isize, isize), u8> {
        let mut pixels: BTreeMap<(isize, isize), u8> = BTreeMap::new();
        for (p, c) in iter {
            if c == 0 {
                continue;
            }
            pixels
                .entry(p)
                .and_modify(|old| *old = (*old).max(c))
                .or_insert(c);
        }
        pixels
    }

    fn quadrant(center: (isize, isize), corner: Corner, r: isize) -> BTreeMap<(isize, isize), u8> {
        blend(
            CircleAa::new(center, r)
                .filter(|(p, _)| corner.contains(center, *p))
                .map(|(p, c)| (p, c)),
        )
    }

    #[test]
    fn corners_match_circle_aa_quadrants() {
        let cases = [
            ((0, 0), (20, 14), 5),
            ((-3, 2), (11, 18), 4),
            ((0, 0), (7, 7), 3),
            ((0, 0), (7, 7), 2),
            ((1, 1), (16, 9), 1),
        ];
        for (p0, p1, r) in cases {
            let rr = blend(RoundRectAa::new(p0, p1, r));
            let (x0, y0, x1, y1, r) = crate::round_rect::normalize_rect(p0, p1, r);
            if r == 0 {
                continue;
            }
            let corners = [
                (Corner::TopLeft, (x0 + r, y0 + r)),
                (Corner::TopRight, (x1 - r, y0 + r)),
                (Corner::BottomLeft, (x0 + r, y1 - r)),
                (Corner::BottomRight, (x1 - r, y1 - r)),
            ];
            for (corner, center) in corners {
                let from_circle = quadrant(center, corner, r);
                let from_rect: BTreeMap<_, _> = rr
                    .iter()
                    .filter(|(p, _)| corner.contains(center, **p))
                    .map(|(p, c)| (*p, *c))
                    .collect();
                assert_eq!(from_rect, from_circle, "{p0:?}->{p1:?} r={r} {center:?}");
            }
        }
    }

    #[test]
    fn reversed_corners_match() {
        let a = blend(RoundRectAa::new((0, 0), (12, 8), 3));
        let b = blend(RoundRectAa::new((12, 8), (0, 0), 3));
        assert_eq!(a, b);
    }

    #[test]
    fn negative_radius_matches_abs() {
        let pos = blend(RoundRectAa::new((0, 0), (12, 8), 3));
        let neg = blend(RoundRectAa::new((0, 0), (12, 8), -3));
        assert_eq!(pos, neg);
    }

    #[test]
    fn zero_radius_is_hard_rect() {
        let aa = blend(RoundRectAa::new((0, 0), (7, 7), 0));
        let hard: BTreeMap<_, _> = RoundRect::new((0, 0), (7, 7), 0)
            .map(|p| (p, 255u8))
            .collect();
        assert_eq!(aa, hard);
    }

    #[test]
    fn zero_radius_degenerate_rects() {
        assert_eq!(
            blend(RoundRectAa::new((3, 4), (3, 4), 0)),
            BTreeMap::from([((3, 4), 255)])
        );
        let row: BTreeMap<_, _> = (0..=3).map(|x| ((x, 2), 255u8)).collect();
        assert_eq!(blend(RoundRectAa::new((0, 2), (3, 2), 0)), row);
        let col: BTreeMap<_, _> = (0..=3).map(|y| ((2, y), 255u8)).collect();
        assert_eq!(blend(RoundRectAa::new((2, 0), (2, 3), 0)), col);
    }

    #[test]
    fn straight_edges_are_fully_on() {
        let (p0, p1, r) = ((0, 0), (20, 14), 5);
        let pixels = blend(RoundRectAa::new(p0, p1, r));
        let (x0, y0, x1, y1, r) = crate::round_rect::normalize_rect(p0, p1, r);
        for x in (x0 + r + 1)..=(x1 - r - 1) {
            assert_eq!(pixels.get(&(x, y0)), Some(&255));
            assert_eq!(pixels.get(&(x, y1)), Some(&255));
        }
        for y in (y0 + r + 1)..=(y1 - r - 1) {
            assert_eq!(pixels.get(&(x0, y)), Some(&255));
            assert_eq!(pixels.get(&(x1, y)), Some(&255));
        }
    }

    /// 8×8 r=2: corners are `CircleAa` r=2, flats are fully on.
    #[test]
    fn test_round_rect_aa_shape() {
        let mut grid = [0u32; 8];
        for ((x, y), c) in RoundRectAa::new((0, 0), (7, 7), 2) {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            let shift = ((7 - x) * 4) as u32;
            let nyb = (c as u32) >> 4;
            let old = (grid[y as usize] >> shift) & 0xf;
            grid[y as usize] = (grid[y as usize] & !(0xf << shift)) | (old.max(nyb) << shift);
        }
        #[rustfmt::skip]
        assert_eq!(grid, [
            0x0cffffc0,
            0xc300003c,
            0xf000000f,
            0xf000000f,
            0xf000000f,
            0xf000000f,
            0xc300003c,
            0x0cffffc0,
        ]);
    }

    #[cfg(feature = "fill")]
    #[test]
    fn fill_is_hard_interior_plus_outside_fringe() {
        use crate::fill::{Fill, Plot};

        let hard: BTreeMap<_, _> = RoundRect::new((0, 0), (20, 14), 5)
            .fill()
            .flat_map(|s| (s.x0..=s.x1).map(move |x| ((x, s.y), 255u8)))
            .collect();
        let mut from_fill: BTreeMap<(isize, isize), u8> = BTreeMap::new();
        for plot in RoundRectAa::new((0, 0), (20, 14), 5).fill() {
            match plot {
                Plot::Span(s) => {
                    for x in s.x0..=s.x1 {
                        from_fill.insert((x, s.y), 255);
                    }
                }
                Plot::Point((p, c)) => {
                    assert!(c > 0);
                    assert!(!hard.contains_key(&p), "fringe {p:?} is inside");
                    from_fill
                        .entry(p)
                        .and_modify(|old| *old = (*old).max(c))
                        .or_insert(c);
                }
            }
        }
        for (p, c) in &hard {
            assert_eq!(from_fill.get(p), Some(c), "missing solid {p:?}");
        }
    }
}
