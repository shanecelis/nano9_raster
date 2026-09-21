//! Axis-aligned ellipses from Alois Zingl's `plotEllipse` and `plotEllipseRect`.
//!
//! [`Ellipse::new`] is a [`QuadArc`] from `(a, 0)` to `(0, b)`, reflected and
//! translated the same way as the circle outline. Circle keeps its own
//! single-radius `QuadArc`: folding both into one type would widen the circle
//! step to `i64` `a²`/`b²` arithmetic.

#[cfg(feature = "fill")]
use arraydeque::ArrayDeque;

#[cfg(feature = "fill")]
use crate::fill::Span;
use crate::{reflect_x, reflect_y, AndMap, Point, PointIteratorExt};

/// Iterator over one quarter of an axis-aligned ellipse centered at the origin.
///
/// The points run from `(a, 0)` toward `(0, b)`, including the flat-ellipse
/// tip Zingl plots after the main midpoint loop. Combine this with
/// [`PointIteratorExt`] to reflect and translate the arc.
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
        QuadArc::new(a, b)
            .and_map(reflect_x)
            .and_map(reflect_y)
            .translate(center.0, center.1)
    }

    /// Closed ellipse filling the rectangle with opposite corners `p0` and
    /// `p1`.
    #[inline]
    pub fn from_rect(p0: Point, p1: Point) -> impl Iterator<Item = Point> {
        EllipseRect::new(p0, p1)
    }
}

/// Iterator over filled-ellipse scanlines built from a [`QuadArc`].
#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "ellipse", feature = "fill"))))]
pub struct EllipseFill {
    xm: isize,
    ym: isize,
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
        EllipseFill {
            xm: center.0,
            ym: center.1,
            arc: QuadArc::new(a, b),
            last_y: None,
            pending: None,
        }
    }

    /// Filled ellipse inscribed in the rectangle with opposite corners `p0`
    /// and `p1`.
    #[inline]
    pub fn from_rect(p0: Point, p1: Point) -> impl Iterator<Item = Span> {
        EllipseRect::new(p0, p1).fill()
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
            if y != 0 {
                self.pending = Some(self.span(x, reflect_x((x, y)).1));
            }
            return Some(span);
        }
    }
}

enum EllipsePhase {
    Main { quad: u8 },
    Tip { which: u8 },
}

/// Iterator over an axis-aligned ellipse inscribed in a rectangle.
struct EllipseRect {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    a: i64,
    b: i64,
    b1: i64,
    dx: i64,
    dy: i64,
    err: i64,
    phase: EllipsePhase,
    done: bool,
}

impl EllipseRect {
    /// Closed ellipse filling the rectangle with opposite corners `p0` and `p1`.
    #[inline]
    fn new(p0: Point, p1: Point) -> Self {
        let (mut x0, mut y0) = p0;
        let (mut x1, y1) = p1;
        let a = (x1 - x0).abs() as i64;
        let b = (y1 - y0).abs() as i64;
        let b1 = b & 1;
        let dx = 4 * (1 - a) * b * b;
        let dy = 4 * (b1 + 1) * a * a;
        let err = dx + dy + b1 * a * a;

        if x0 > x1 {
            x0 = x1;
            x1 += a as isize;
        }
        if y0 > y1 {
            y0 = y1;
        }
        y0 += ((b + 1) / 2) as isize;
        let y1 = y0 - b1 as isize;
        let a = 8 * a * a;
        let b1 = 8 * b * b;

        EllipseRect {
            x0,
            y0,
            x1,
            y1,
            a,
            b,
            b1,
            dx,
            dy,
            err,
            phase: EllipsePhase::Main { quad: 0 },
            done: false,
        }
    }

    #[inline]
    fn points4(&self) -> [Point; 4] {
        [
            (self.x1, self.y0),
            (self.x0, self.y0),
            (self.x0, self.y1),
            (self.x1, self.y1),
        ]
    }

    #[inline]
    fn tip4(&self) -> [Point; 4] {
        [
            (self.x0 - 1, self.y0),
            (self.x1 + 1, self.y0),
            (self.x0 - 1, self.y1),
            (self.x1 + 1, self.y1),
        ]
    }

    #[inline]
    fn advance_main(&mut self) {
        let e2 = 2 * self.err;
        if e2 <= self.dy {
            self.y0 += 1;
            self.y1 -= 1;
            self.dy += self.a;
            self.err += self.dy;
        }
        if e2 >= self.dx || 2 * self.err > self.dy {
            self.x0 += 1;
            self.x1 -= 1;
            self.dx += self.b1;
            self.err += self.dx;
        }
        if self.x0 > self.x1 {
            self.phase = EllipsePhase::Tip { which: 0 };
        }
    }
}

#[cfg(feature = "fill")]
impl EllipseRect {
    #[inline]
    fn fill(self) -> EllipseRectFill {
        EllipseRectFill {
            e: self,
            pending: ArrayDeque::new(),
            open0: None,
            open1: None,
            finished: false,
        }
    }
}

/// Iterator over [`Span`] chords of a filled [`EllipseRect`]. Inclusive `[x0, x1]`.
#[cfg(feature = "fill")]
struct EllipseRectFill {
    e: EllipseRect,
    pending: ArrayDeque<Span, 2>,
    open0: Option<Span>,
    open1: Option<Span>,
    finished: bool,
}

#[cfg(feature = "fill")]
fn set_track(open: &mut Option<Span>, x0: isize, x1: isize, y: isize) -> Option<Span> {
    match *open {
        None => {
            *open = Some(Span { x0, x1, y });
            None
        }
        Some(h) if h.y == y => {
            *open = Some(Span {
                x0: h.x0.min(x0),
                x1: h.x1.max(x1),
                y,
            });
            None
        }
        Some(h) => {
            *open = Some(Span { x0, x1, y });
            Some(h)
        }
    }
}

#[cfg(feature = "fill")]
impl EllipseRectFill {
    fn push(&mut self, h: Span) {
        self.pending
            .push_back(h)
            .expect("EllipseRectFill pending overflow");
    }

    fn absorb_y0(&mut self, x0: isize, x1: isize, y: isize) {
        if let Some(h) = set_track(&mut self.open0, x0, x1, y) {
            self.push(h);
        }
    }

    fn absorb_y1(&mut self, x0: isize, x1: isize, y: isize) {
        if let Some(h) = set_track(&mut self.open1, x0, x1, y) {
            self.push(h);
        }
    }

    fn flush_opens(&mut self) {
        if let Some(h) = self.open0.take() {
            self.push(h);
        }
        if let Some(h) = self.open1.take() {
            self.push(h);
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for EllipseRectFill {
    type Item = Span;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(h) = self.pending.pop_front() {
                return Some(h);
            }

            if self.finished {
                return None;
            }

            if let EllipsePhase::Main { .. } = self.e.phase {
                self.absorb_y0(self.e.x0, self.e.x1, self.e.y0);
                if self.e.y0 != self.e.y1 {
                    self.absorb_y1(self.e.x0, self.e.x1, self.e.y1);
                }
                self.e.advance_main();
                continue;
            }

            if (self.e.y0 - self.e.y1) as i64 <= self.e.b {
                let x0 = self.e.x0 - 1;
                let x1 = self.e.x1 + 1;
                let y0 = self.e.y0;
                let y1 = self.e.y1;
                self.absorb_y0(x0, x1, y0);
                self.e.y0 += 1;
                if y1 != y0 {
                    self.absorb_y1(x0, x1, y1);
                }
                self.e.y1 -= 1;
                continue;
            }

            self.flush_opens();
            self.finished = true;
        }
    }
}

impl Iterator for EllipseRect {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.phase {
            EllipsePhase::Main { quad } => {
                let p = self.points4()[quad as usize];

                if quad < 3 {
                    self.phase = EllipsePhase::Main { quad: quad + 1 };
                } else {
                    self.phase = EllipsePhase::Main { quad: 0 };
                    self.advance_main();
                }

                Some(p)
            }
            EllipsePhase::Tip { which } => {
                // `while (y0 - y1 <= b)` — only test at the start of a 4-pixel group.
                if which == 0 && (self.y0 - self.y1) as i64 > self.b {
                    self.done = true;
                    return None;
                }

                let p = self.tip4()[which as usize];

                if which < 3 {
                    if which == 1 {
                        self.y0 += 1;
                    }
                    self.phase = EllipsePhase::Tip { which: which + 1 };
                } else {
                    self.y1 -= 1;
                    self.phase = EllipsePhase::Tip { which: 0 };
                }

                Some(p)
            }
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
        let res: Vec<_> = Ellipse::from_rect((0, 0), (8, 4)).collect();
        assert_eq!(
            res,
            [
                (8, 2),
                (0, 2),
                (0, 2),
                (8, 2),
                (7, 3),
                (1, 3),
                (1, 1),
                (7, 1),
                (6, 4),
                (2, 4),
                (2, 0),
                (6, 0),
                (5, 4),
                (3, 4),
                (3, 0),
                (5, 0),
                (4, 4),
                (4, 4),
                (4, 0),
                (4, 0),
                (4, 4),
                (4, 4),
                (4, 0),
                (4, 0)
            ]
        );
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
