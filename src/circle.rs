//! Midpoint circle from Alois Zingl's `plotCircle`.

#[cfg(feature = "fill")]
use crate::fill::Span;
use crate::{reflect_x, reflect_y, AndMap, Point, PointIteratorExt};

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
        if r > -self.x {
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
    use super::{Circle, QuadArc};
    #[cfg(feature = "fill")]
    use crate::common::plot_spans;
    use crate::common::{assert_bitmap_eq, plot_bits};
    use crate::Point;
    use crate::{reflect_x, reflect_y, AndMap, PointIteratorExt};
    use std::vec::Vec;

    #[test]
    fn test_circle() {
        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<1>(Circle::new((0, 0), 0), 1), [
            0b1, // #
        ], 1);

        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<3>(Circle::new((1, 1), 1), 3), [
            0b010, // .#.
            0b101, // #.#
            0b010, // .#.
        ], 3);

        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<8>(Circle::new((5, 5), 2), 8), [
            0b00000000, // ........
            0b00000000, // ........
            0b00000000, // ........
            0b00001110, // ....###.
            0b00010001, // ...#...#
            0b00010001, // ...#...#
            0b00010001, // ...#...#
            0b00001110, // ....###.
        ], 8);

        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<9>(Circle::new((4, 4), 4), 9), [
            0b000111000, // ...###...
            0b011000110, // .##...##.
            0b010000010, // .#.....#.
            0b100000001, // #.......#
            0b100000001, // #.......#
            0b100000001, // #.......#
            0b010000010, // .#.....#.
            0b011000110, // .##...##.
            0b000111000, // ...###...
        ], 9);
    }

    /// The `r = 3` circle rendered onto an 8x8 one-bit grid: one row per
    /// byte, most significant bit is the leftmost column.
    #[test]
    fn test_circle_shape() {
        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<8>(Circle::new((3, 3), 3), 8), [
            0b00111000,
            0b01000100,
            0b10000010,
            0b10000010,
            0b10000010,
            0b01000100,
            0b00111000,
            0b00000000,
        ], 8);
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
        #[rustfmt::skip]
        assert_bitmap_eq!(
            plot_spans::<8>(CircleFill::new((3, 3), 3).map(|h| (h.x0, h.x1, h.y)), 8),
            [
                0b00111000,
                0b01111100,
                0b11111110,
                0b11111110,
                0b11111110,
                0b01111100,
                0b00111000,
                0b00000000,
            ],
            8
        );
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
