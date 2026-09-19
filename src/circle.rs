//! Midpoint circle from Alois Zingl's `plotCircle`.

#[cfg(feature = "fill")]
use crate::fill::Span;
use crate::{AndMap, Point};

/// Iterator over one quarter of a circle centered at the origin.
///
/// The points run from `(radius, 0)` toward `(0, radius)`. Combine this with
/// [`PointIteratorExt`] to reflect and translate the arc.
#[derive(Clone, Copy)]
pub struct QuadArc {
    x: isize,
    y: isize,
    err: isize,
}

impl QuadArc {
    /// Quarter-arc with the given radius.
    ///
    /// Negative radii are treated as their absolute value.
    #[inline]
    pub fn new(radius: isize) -> Self {
        let r = radius.abs();
        QuadArc {
            x: r,
            y: 0,
            err: 2 - 2 * r,
        }
    }

    #[inline]
    fn advance(&mut self) {
        let r = self.err;
        if r <= self.y {
            self.y += 1;
            self.err += self.y * 2 + 1;
        }
        if r > -self.x || self.err > self.y {
            self.x -= 1;
            self.err += 1 - self.x * 2;
        }
    }
}

impl Iterator for QuadArc {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.x < 0 {
            return None;
        }
        let point = (self.x, self.y);
        self.advance();
        Some(point)
    }
}

/// Reflect a point across the x-axis.
#[inline]
pub fn reflect_x((x, y): Point) -> Point {
    (x, -y)
}

/// Reflect a point across the y-axis.
#[inline]
pub fn reflect_y((x, y): Point) -> Point {
    (-x, y)
}

/// An iterator that translates every point by a fixed offset.
pub struct Translate<I> {
    iter: I,
    dx: isize,
    dy: isize,
}

impl<I: Iterator<Item = Point>> Iterator for Translate<I> {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x + self.dx, y + self.dy))
    }
}

/// Composable transforms for iterators over raster points.
pub trait PointIteratorExt: Iterator<Item = Point> + Sized {
    /// Translate every point by `(dx, dy)`.
    #[inline]
    fn translate(self, dx: isize, dy: isize) -> Translate<Self> {
        Translate { iter: self, dx, dy }
    }
}

impl<I: Iterator<Item = Point>> PointIteratorExt for I {}

/// Circle outline iterator factory.
pub struct Circle;

impl Circle {
    /// Closed circle centered at `center` with the given `radius`.
    ///
    /// Negative radii are treated as their absolute value.
    #[inline]
    pub fn new(center: Point, radius: isize) -> impl Iterator<Item = Point> {
        QuadArc::new(radius)
            .and_map(reflect_x)
            .and_map(reflect_y)
            .translate(center.0, center.1)
    }
}

/// Iterator over filled-circle scanlines built from a [`QuadArc`].
#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "circle", feature = "fill"))))]
pub struct CircleFill {
    xm: isize,
    ym: isize,
    arc: QuadArc,
    last_y: Option<isize>,
    pending: Option<Span>,
}

#[cfg(feature = "fill")]
impl CircleFill {
    /// Filled circle centered at `center` with the given radius.
    #[inline]
    pub fn new(center: Point, radius: isize) -> Self {
        CircleFill {
            xm: center.0,
            ym: center.1,
            arc: QuadArc::new(radius),
            last_y: None,
            pending: None,
        }
    }

    #[inline]
    fn span(&self, x: isize, y: isize) -> Span {
        Span {
            x0: self.xm - x,
            x1: self.xm + x,
            y: self.ym + y,
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for CircleFill {
    type Item = Span;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(span) = self.pending.take() {
            return Some(span);
        }

        loop {
            let (x, y) = self.arc.next()?;
            if self.last_y == Some(y) {
                continue;
            }
            self.last_y = Some(y);

            let span = self.span(x, y);
            if y != 0 {
                self.pending = Some(self.span(x, reflect_x((x, y)).1));
            }
            return Some(span);
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "fill")]
    use super::CircleFill;
    use super::{reflect_x, reflect_y, Circle, PointIteratorExt, QuadArc};
    use crate::{AndMap, Point};
    use std::vec::Vec;

    fn assert_point_set(actual: impl Iterator<Item = Point>, expected: &[Point]) {
        let mut actual: Vec<_> = actual.collect();
        actual.sort_unstable();
        actual.dedup();
        let mut expected = expected.to_vec();
        expected.sort_unstable();
        expected.dedup();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_circle() {
        assert_point_set(Circle::new((0, 0), 0), &[(0, 0)]);
        assert_point_set(Circle::new((0, 0), 1), &[(1, 0), (0, 1), (-1, 0), (0, -1)]);
        assert_point_set(
            Circle::new((5, 5), 2),
            &[
                (7, 5),
                (5, 7),
                (3, 5),
                (5, 3),
                (7, 6),
                (4, 7),
                (3, 4),
                (6, 3),
                (6, 7),
                (3, 6),
                (4, 3),
                (7, 4),
            ],
        );
        assert_point_set(
            Circle::new((0, 0), 4),
            &[
                (4, 0),
                (0, 4),
                (-4, 0),
                (0, -4),
                (4, 1),
                (-1, 4),
                (-4, -1),
                (1, -4),
                (3, 2),
                (-2, 3),
                (-3, -2),
                (2, -3),
                (2, 3),
                (-3, 2),
                (-2, -3),
                (3, -2),
                (1, 4),
                (-4, 1),
                (-1, -4),
                (4, -1),
            ],
        );
    }

    /// The `r = 3` circle rendered onto an 8x8 one-bit grid: one row per
    /// byte, most significant bit is the leftmost column.
    #[test]
    fn test_circle_shape() {
        let mut grid = [0u8; 8];
        for (x, y) in Circle::new((3, 3), 3) {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            grid[y as usize] |= 0x80 >> x;
        }
        #[rustfmt::skip]
        assert_eq!(grid, [
            0b00111000,
            0b01000100,
            0b10000010,
            0b10000010,
            0b10000010,
            0b01000100,
            0b00111000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_circle_for_each_matches_iter() {
        for r in 0..16 {
            let a: Vec<_> = Circle::new((3, -2), r).collect();
            let mut b = Vec::new();
            Circle::new((3, -2), r).for_each(|p| b.push(p));
            assert_eq!(a, b, "r={r}");
        }
    }

    #[test]
    fn test_reflections_are_one_to_one() {
        assert_eq!(reflect_x((2, 3)), (2, -3));
        assert_eq!(reflect_y((2, 3)), (-2, 3));
    }

    #[test]
    fn test_composed_circle_matches_point_set() {
        for r in -15..16 {
            let mut baseline: Vec<_> = Circle::new((3, -2), r).collect();
            let mut composed: Vec<_> = QuadArc::new(r)
                .and_map(reflect_x)
                .and_map(reflect_y)
                .translate(3, -2)
                .collect();
            baseline.sort_unstable();
            baseline.dedup();
            composed.sort_unstable();
            composed.dedup();
            assert_eq!(baseline, composed, "r={r}");
        }
    }

    #[cfg(feature = "fill")]
    fn expand(spans: &[crate::fill::Span]) -> Vec<Point> {
        let mut v = Vec::new();
        for h in spans {
            for x in h.x0..=h.x1 {
                v.push((x, h.y));
            }
        }
        v
    }

    /// The filled `r = 3` circle rendered onto an 8x8 one-bit grid: one row
    /// per byte, most significant bit is the leftmost column.
    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_fill_shape() {
        let mut grid = [0u8; 8];
        for h in CircleFill::new((3, 3), 3) {
            assert!(
                (0..8).contains(&h.y) && h.x0 >= 0 && h.x1 < 8,
                "{h:?} off grid"
            );
            for x in h.x0..=h.x1 {
                grid[h.y as usize] |= 0x80 >> x;
            }
        }
        #[rustfmt::skip]
        assert_eq!(grid, [
            0b00111000,
            0b01111100,
            0b11111110,
            0b11111110,
            0b11111110,
            0b01111100,
            0b00111000,
            0b00000000,
        ]);
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_fill() {
        use crate::fill::Span;

        let res: Vec<_> = CircleFill::new((0, 0), 0).collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for r in 0..16 {
            let spans: Vec<_> = CircleFill::new((3, -2), r).collect();

            for h in &spans {
                assert!(h.x0 <= h.x1, "r={r} {h:?}");
            }

            let mut ys: Vec<_> = spans.iter().map(|h| h.y).collect();
            let n = ys.len();
            ys.sort();
            ys.dedup();
            assert_eq!(ys.len(), n, "duplicate y r={r} {spans:?}");

            let filled = expand(&spans);
            for p in Circle::new((3, -2), r) {
                assert!(
                    filled.contains(&p),
                    "outline {p:?} not in fill r={r} {spans:?}"
                );
            }
        }
    }
}
