//! Axis-aligned ellipses from Alois Zingl's `plotEllipse`.
//!
//! [`QuadArc`] runs `(a, 0) → (0, b)`. [`Ellipse::new`] reflects that arc
//! across both axes like the circle outline. [`Ellipse::from_rect`] maps the
//! same arc into the bounding box and reflects across the box midlines, so
//! even pixel sizes work without Zingl's `plotEllipseRect`.

#[cfg(feature = "fill")]
use crate::fill::Span;
use crate::{AndMap, Point};

/// Iterator over one quarter of an axis-aligned ellipse centered at the origin.
///
/// The points run from `(a, 0)` toward `(0, b)`, including the flat-ellipse
/// tip Zingl plots after the main midpoint loop.
#[derive(Clone, Copy)]
pub struct QuadArc {
    x: isize,
    y: isize,
    err: i64,
    a2: i64,
    b2: i64,
    b: isize,
}

impl QuadArc {
    /// Quarter-ellipse with horizontal radius `a` and vertical radius `b`.
    ///
    /// Negative radii are treated as their absolute value.
    #[inline]
    pub fn new(a: isize, b: isize) -> Self {
        let a = a.abs();
        let b = b.abs();
        let a2 = (a as i64) * (a as i64);
        let b2 = (b as i64) * (b as i64);
        QuadArc {
            x: a,
            y: 0,
            // Zingl `plotEllipse` starts at `x = -a`; this is that error term
            // with the sign of `x` flipped so the arc runs `(a, 0) → (0, b)`.
            err: (1 - 2 * a) as i64 * b2 + a2,
            a2,
            b2,
            b,
        }
    }

    #[inline]
    fn advance(&mut self) {
        let e2 = 2 * self.err;
        if e2 >= (1 - 2 * self.x) as i64 * self.b2 {
            self.x -= 1;
            self.err += (1 - 2 * self.x) as i64 * self.b2;
        }
        if e2 <= (2 * self.y + 1) as i64 * self.a2 {
            self.y += 1;
            self.err += (2 * self.y + 1) as i64 * self.a2;
        }
    }
}

impl Iterator for QuadArc {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= 0 {
            let point = (self.x, self.y);
            self.advance();
            return Some(point);
        }
        // `while (y++ < b)` — finish the tip of flat ellipses (`a = 1`).
        self.y += 1;
        if self.y > self.b {
            return None;
        }
        Some((0, self.y))
    }
}

/// Ellipse outline iterator factory.
pub struct Ellipse;

impl Ellipse {
    /// Closed ellipse centered at `center` with horizontal radius `a` and
    /// vertical radius `b`. Negative radii are treated as their absolute value.
    #[inline]
    pub fn new(center: Point, a: isize, b: isize) -> impl Iterator<Item = Point> {
        let a = a.abs();
        let b = b.abs();
        Self::from_rect((center.0 - a, center.1 - b), (center.0 + a, center.1 + b))
    }

    /// Closed ellipse filling the rectangle with opposite corners `p0` and
    /// `p1`.
    ///
    /// A [`QuadArc`] of radii `((x1-x0)/2, (y1-y0)/2)` is placed in the first
    /// quadrant of the box, then reflected across the horizontal and vertical
    /// midlines. Odd sizes match [`Self::new`]; even sizes split the center
    /// across two rows or columns so the outline still touches all four sides.
    #[inline]
    pub fn from_rect(p0: Point, p1: Point) -> impl Iterator<Item = Point> {
        let (x0, x1, y0, y1, a, b) = sorted_rect(p0, p1);
        QuadArc::new(a, b)
            .map(move |(x, y)| (x1 - a + x, y1 - b + y))
            .and_map(move |(x, y)| (x, y0 + y1 - y))
            .and_map(move |(x, y)| (x0 + x1 - x, y))
    }
}

#[inline]
fn sorted_rect(p0: Point, p1: Point) -> (isize, isize, isize, isize, isize, isize) {
    let x0 = p0.0.min(p1.0);
    let x1 = p0.0.max(p1.0);
    let y0 = p0.1.min(p1.1);
    let y1 = p0.1.max(p1.1);
    (x0, x1, y0, y1, (x1 - x0) / 2, (y1 - y0) / 2)
}

/// Iterator over filled-ellipse scanlines built from a [`QuadArc`].
#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "ellipse", feature = "fill"))))]
pub struct EllipseFill {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    a: isize,
    b: isize,
    arc: QuadArc,
    last_y: Option<isize>,
    pending: Option<Span>,
}

#[cfg(feature = "fill")]
impl EllipseFill {
    /// Filled ellipse centered at `center` with horizontal radius `a` and
    /// vertical radius `b`.
    #[inline]
    pub fn new(center: Point, a: isize, b: isize) -> Self {
        let a = a.abs();
        let b = b.abs();
        Self::from_rect((center.0 - a, center.1 - b), (center.0 + a, center.1 + b))
    }

    /// Filled ellipse inscribed in the rectangle with opposite corners `p0`
    /// and `p1`.
    #[inline]
    pub fn from_rect(p0: Point, p1: Point) -> Self {
        let (x0, x1, y0, y1, a, b) = sorted_rect(p0, p1);
        EllipseFill {
            x0,
            y0,
            x1,
            y1,
            a,
            b,
            arc: QuadArc::new(a, b),
            last_y: None,
            pending: None,
        }
    }

    #[inline]
    fn span(&self, x: isize, y: isize) -> Span {
        Span {
            x0: self.x0 + self.a - x,
            x1: self.x1 - self.a + x,
            y: self.y1 - self.b + y,
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for EllipseFill {
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
            let y_neg = self.y0 + self.b - y;
            if y_neg != span.y {
                self.pending = Some(Span {
                    x0: span.x0,
                    x1: span.x1,
                    y: y_neg,
                });
            }
            return Some(span);
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "fill")]
    use super::EllipseFill;
    use super::{Ellipse, QuadArc};
    #[cfg(feature = "fill")]
    use crate::common::plot_spans;
    use crate::common::{assert_bitmap_eq, plot_bits};
    use crate::{reflect_x, reflect_y, AndMap, Point, PointIteratorExt};
    use std::vec::Vec;

    fn zingl_plot_ellipse(center: Point, a: isize, b: isize) -> Vec<Point> {
        let a = a.abs();
        let b = b.abs();
        let xm = center.0;
        let ym = center.1;
        let mut x = -a;
        let mut y = 0isize;
        let b2 = (b as i64) * (b as i64);
        let a2 = (a as i64) * (a as i64);
        let mut err = (x as i64) * (2 * b2 + x as i64) + b2;
        let mut points = Vec::new();
        loop {
            points.push((xm - x, ym + y));
            points.push((xm + x, ym + y));
            points.push((xm + x, ym - y));
            points.push((xm - x, ym - y));
            let e2 = 2 * err;
            if e2 >= (x * 2 + 1) as i64 * b2 {
                x += 1;
                err += (x * 2 + 1) as i64 * b2;
            }
            if e2 <= (y * 2 + 1) as i64 * a2 {
                y += 1;
                err += (y * 2 + 1) as i64 * a2;
            }
            if x > 0 {
                break;
            }
        }
        while {
            y += 1;
            y <= b
        } {
            points.push((xm, ym + y));
            points.push((xm, ym - y));
        }
        points
    }

    fn sorted_unique(mut points: Vec<Point>) -> Vec<Point> {
        points.sort_unstable();
        points.dedup();
        points
    }

    #[test]
    fn test_ellipse() {
        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<5>(Ellipse::new((5, 2), 5, 2), 11), [
            0b00111111100, // ..#######..
            0b01000000010, // .#.......#.
            0b10000000001, // #.........#
            0b01000000010, // .#.......#.
            0b00111111100, // ..#######..
        ], 11);

        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<9>(Ellipse::new((1, 4), 1, 4), 3), [
            0b010, // .#.
            0b010, // .#.
            0b101, // #.#
            0b101, // #.#
            0b101, // #.#
            0b101, // #.#
            0b101, // #.#
            0b010, // .#.
            0b010, // .#.
        ], 3);
    }

    #[test]
    fn test_ellipse_matches_zingl_set() {
        for &(c, a, b) in &[
            ((0, 0), 0, 0),
            ((0, 0), 5, 2),
            ((0, 0), 1, 4),
            ((3, -2), 6, 6),
            ((1, 1), 0, 5),
            ((1, 1), 5, 0),
            ((4, 4), 7, 3),
            ((0, 0), 8, 1),
        ] {
            let got = sorted_unique(Ellipse::new(c, a, b).collect());
            let expected = sorted_unique(zingl_plot_ellipse(c, a, b));
            assert_eq!(got, expected, "center={c:?} a={a} b={b}");
        }
    }

    #[test]
    fn test_composed_ellipse_matches_point_set() {
        for r in -15..16 {
            for b in -7..8 {
                let mut baseline: Vec<_> = Ellipse::new((3, -2), r, b).collect();
                let mut composed: Vec<_> = QuadArc::new(r, b)
                    .and_map(reflect_x)
                    .and_map(reflect_y)
                    .translate(3, -2)
                    .collect();
                baseline.sort_unstable();
                baseline.dedup();
                composed.sort_unstable();
                composed.dedup();
                assert_eq!(baseline, composed, "a={r} b={b}");
            }
        }
    }

    #[test]
    fn test_ellipse_rect() {
        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<5>(Ellipse::from_rect((0, 0), (8, 4)), 9), [
            0b001111100, // ..#####..
            0b010000010, // .#.....#.
            0b100000001, // #.......#
            0b010000010, // .#.....#.
            0b001111100, // ..#####..
        ], 9);
    }

    #[test]
    fn test_ellipse_even_rect() {
        #[rustfmt::skip]
        assert_bitmap_eq!(plot_bits::<4>(Ellipse::from_rect((0, 0), (7, 3)), 8), [
            0b01111110, // .######.
            0b10000001, // #......#
            0b10000001, // #......#
            0b01111110, // .######.
        ], 8);
    }

    #[test]
    fn test_ellipse_rect_for_each_matches_iter() {
        for &(p0, p1) in &[
            ((0, 0), (8, 4)),
            ((0, 0), (0, 0)),
            ((2, 3), (12, 10)),
            ((10, 1), (1, 8)),
        ] {
            let a: Vec<_> = Ellipse::from_rect(p0, p1).collect();
            let mut b = Vec::new();
            Ellipse::from_rect(p0, p1).for_each(|p| b.push(p));
            assert_eq!(a, b, "{p0:?} {p1:?}");
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

    #[cfg(feature = "fill")]
    fn assert_fill_ok(spans: &[crate::fill::Span], outline: &[Point], label: &str) {
        for h in spans {
            assert!(h.x0 <= h.x1, "{label} {h:?}");
        }
        let n = spans.len();
        let mut ys: Vec<_> = spans.iter().map(|h| h.y).collect();
        ys.sort();
        ys.dedup();
        assert_eq!(ys.len(), n, "duplicate y {label} {spans:?}");

        let filled = expand(spans);
        for p in outline {
            assert!(filled.contains(p), "outline {p:?} not in fill {label}");
        }
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_ellipse_fill_shape() {
        #[rustfmt::skip]
        assert_bitmap_eq!(
            plot_spans::<5>(EllipseFill::new((5, 2), 5, 2).map(|h| (h.x0, h.x1, h.y)), 11),
            [
                0b00111111100, // ..#######..
                0b01111111110, // .#########.
                0b11111111111, // ###########
                0b01111111110, // .#########.
                0b00111111100, // ..#######..
            ],
            11
        );
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_ellipse_fill() {
        use crate::fill::Span;

        let res: Vec<_> = EllipseFill::new((0, 0), 0, 0).collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for &(c, a, b) in &[
            ((0, 0), 0, 0),
            ((0, 0), 5, 2),
            ((0, 0), 1, 4),
            ((3, -2), 6, 6),
            ((1, 1), 0, 5),
            ((1, 1), 5, 0),
        ] {
            let spans: Vec<_> = EllipseFill::new(c, a, b).collect();
            let outline: Vec<_> = Ellipse::new(c, a, b).collect();
            assert_fill_ok(&spans, &outline, "ellipse");
        }
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_ellipse_rect_fill() {
        use crate::fill::Span;

        let res: Vec<_> = EllipseFill::from_rect((0, 0), (0, 0)).collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for &(p0, p1) in &[
            ((0, 0), (0, 0)),
            ((0, 0), (8, 4)),
            ((2, 3), (12, 10)),
            ((10, 1), (1, 8)),
            ((5, 5), (5, 12)),
        ] {
            let spans: Vec<_> = EllipseFill::from_rect(p0, p1).collect();
            let outline: Vec<_> = Ellipse::from_rect(p0, p1).collect();
            assert_fill_ok(&spans, &outline, "ellipse_rect");
        }
    }
}
