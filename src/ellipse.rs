//! Axis-aligned ellipses from Alois Zingl's `plotEllipse` and `plotEllipseRect`.

#[cfg(feature = "fill")]
use crate::fill::{Fill, Span};
use crate::Point;

enum EllipsePhase {
    Main { quad: u8 },
    Tip { which: u8 },
}

/// Iterator over the pixels of an axis-aligned ellipse given a center and radii
pub struct Ellipse {
    xm: isize,
    ym: isize,
    a: isize,
    b: isize,
    x: isize,
    y: isize,
    err: i64,
    phase: EllipsePhase,
    done: bool,
}

impl Ellipse {
    /// Closed ellipse centered at `center` with horizontal radius `a` and
    /// vertical radius `b`. Negative radii are treated as their absolute value.
    #[inline]
    pub fn new(center: Point, a: isize, b: isize) -> Self {
        let a = a.abs();
        let b = b.abs();
        let x = -a;
        let e2 = (b as i64) * (b as i64);
        let err = (x as i64) * (2 * e2 + x as i64) + e2;

        Ellipse {
            xm: center.0,
            ym: center.1,
            a,
            b,
            x,
            y: 0,
            err,
            phase: EllipsePhase::Main { quad: 0 },
            done: false,
        }
    }

    #[cfg(feature = "fill")]
    #[inline]
    fn step_fill(&mut self) {
        let a2 = (self.a as i64) * (self.a as i64);
        let b2 = (self.b as i64) * (self.b as i64);
        let e2 = 2 * self.err;
        if e2 >= (self.x * 2 + 1) as i64 * b2 {
            self.x += 1;
            self.err += (self.x * 2 + 1) as i64 * b2;
        }
        if e2 <= (self.y * 2 + 1) as i64 * a2 {
            self.y += 1;
            self.err += (self.y * 2 + 1) as i64 * a2;
        }
    }
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
impl Fill for Ellipse {
    #[inline]
    fn fill(self) -> impl Iterator<Item = Span> {
        EllipseFill {
            e: self,
            pending: None,
            last_y: None,
            tips: false,
        }
    }
}

/// Iterator over [`Span`] chords of a filled [`Ellipse`]. Inclusive `[x0, x1]`.
#[cfg(feature = "fill")]
pub(crate) struct EllipseFill {
    e: Ellipse,
    pending: Option<Span>,
    last_y: Option<isize>,
    tips: bool,
}

#[cfg(feature = "fill")]
impl Iterator for EllipseFill {
    type Item = Span;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(h) = self.pending.take() {
            return Some(h);
        }

        if !self.tips {
            loop {
                if self.e.x > 0 {
                    self.tips = true;
                    break;
                }
                if self.last_y != Some(self.e.y) {
                    self.last_y = Some(self.e.y);
                    let x0 = self.e.xm + self.e.x;
                    let x1 = self.e.xm - self.e.x;
                    let h = Span {
                        x0,
                        x1,
                        y: self.e.ym + self.e.y,
                    };
                    if self.e.y != 0 {
                        self.pending = Some(Span {
                            x0,
                            x1,
                            y: self.e.ym - self.e.y,
                        });
                    }
                    self.e.step_fill();
                    return Some(h);
                }
                self.e.step_fill();
            }
        }

        if self.e.y < self.e.b {
            self.e.y += 1;
            let h = Span {
                x0: self.e.xm,
                x1: self.e.xm,
                y: self.e.ym + self.e.y,
            };
            self.pending = Some(Span {
                x0: self.e.xm,
                x1: self.e.xm,
                y: self.e.ym - self.e.y,
            });
            return Some(h);
        }

        None
    }
}

impl Iterator for Ellipse {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.phase {
            EllipsePhase::Main { quad } => {
                let p = match quad {
                    0 => (self.xm - self.x, self.ym + self.y),
                    1 => (self.xm + self.x, self.ym + self.y),
                    2 => (self.xm + self.x, self.ym - self.y),
                    3 => (self.xm - self.x, self.ym - self.y),
                    _ => unreachable!(),
                };

                if quad < 3 {
                    self.phase = EllipsePhase::Main { quad: quad + 1 };
                } else {
                    let a2 = (self.a as i64) * (self.a as i64);
                    let b2 = (self.b as i64) * (self.b as i64);
                    let e2 = 2 * self.err;
                    if e2 >= (self.x * 2 + 1) as i64 * b2 {
                        self.x += 1;
                        self.err += (self.x * 2 + 1) as i64 * b2;
                    }
                    if e2 <= (self.y * 2 + 1) as i64 * a2 {
                        self.y += 1;
                        self.err += (self.y * 2 + 1) as i64 * a2;
                    }
                    if self.x <= 0 {
                        self.phase = EllipsePhase::Main { quad: 0 };
                    } else {
                        self.phase = EllipsePhase::Tip { which: 0 };
                    }
                }

                Some(p)
            }
            EllipsePhase::Tip { which } => {
                // `while (y++ < b)` — increment first, then plot if the old y was < b.
                if which == 0 {
                    self.y += 1;
                    if self.y > self.b {
                        self.done = true;
                        return None;
                    }
                }

                let p = if which == 0 {
                    (self.xm, self.ym + self.y)
                } else {
                    (self.xm, self.ym - self.y)
                };

                self.phase = EllipsePhase::Tip {
                    which: if which == 0 { 1 } else { 0 },
                };
                Some(p)
            }
        }
    }
}

/// Iterator over an axis-aligned ellipse inscribed in a rectangle
pub struct EllipseRect {
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
    pub fn new(p0: Point, p1: Point) -> Self {
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

    /// Call `f` with every outline pixel. Faster than the iterator when inlined.
    #[inline]
    pub fn for_each<F: FnMut(Point)>(mut self, mut f: F) {
        while let EllipsePhase::Main { .. } = self.phase {
            let pts = self.points4();
            f(pts[0]);
            f(pts[1]);
            f(pts[2]);
            f(pts[3]);
            self.advance_main();
        }
        while (self.y0 - self.y1) as i64 <= self.b {
            let pts = self.tip4();
            f(pts[0]);
            f(pts[1]);
            self.y0 += 1;
            f(pts[2]);
            f(pts[3]);
            self.y1 -= 1;
        }
    }
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
impl Fill for EllipseRect {
    #[inline]
    fn fill(self) -> impl Iterator<Item = Span> {
        EllipseRectFill {
            e: self,
            pending: [Span { x0: 0, x1: 0, y: 0 }; 2],
            pending_len: 0,
            pending_i: 0,
            open0: None,
            open1: None,
            finished: false,
        }
    }
}

/// Iterator over [`Span`] chords of a filled [`EllipseRect`]. Inclusive `[x0, x1]`.
#[cfg(feature = "fill")]
pub(crate) struct EllipseRectFill {
    e: EllipseRect,
    pending: [Span; 2],
    pending_len: u8,
    pending_i: u8,
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
        self.pending[self.pending_len as usize] = h;
        self.pending_len += 1;
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
            if (self.pending_i as usize) < self.pending_len as usize {
                let h = self.pending[self.pending_i as usize];
                self.pending_i += 1;
                return Some(h);
            }
            self.pending_len = 0;
            self.pending_i = 0;

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
    use super::{Ellipse, EllipseRect};
    #[cfg(feature = "fill")]
    use crate::Point;
    use std::vec::Vec;

    #[test]
    fn test_ellipse() {
        let res: Vec<_> = Ellipse::new((0, 0), 5, 2).collect();
        assert_eq!(
            res,
            [
                (5, 0),
                (-5, 0),
                (-5, 0),
                (5, 0),
                (4, 1),
                (-4, 1),
                (-4, -1),
                (4, -1),
                (3, 2),
                (-3, 2),
                (-3, -2),
                (3, -2),
                (2, 2),
                (-2, 2),
                (-2, -2),
                (2, -2),
                (1, 2),
                (-1, 2),
                (-1, -2),
                (1, -2),
                (0, 2),
                (0, 2),
                (0, -2),
                (0, -2)
            ]
        );

        let res: Vec<_> = Ellipse::new((0, 0), 1, 4).collect();
        assert_eq!(
            res,
            [
                (1, 0),
                (-1, 0),
                (-1, 0),
                (1, 0),
                (1, 1),
                (-1, 1),
                (-1, -1),
                (1, -1),
                (1, 2),
                (-1, 2),
                (-1, -2),
                (1, -2),
                (0, 3),
                (0, 3),
                (0, -3),
                (0, -3),
                (0, 4),
                (0, -4)
            ]
        );
    }

    #[test]
    fn test_ellipse_rect() {
        let res: Vec<_> = EllipseRect::new((0, 0), (8, 4)).collect();
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
            let a: Vec<_> = EllipseRect::new(p0, p1).collect();
            let mut b = Vec::new();
            EllipseRect::new(p0, p1).for_each(|p| b.push(p));
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
    fn test_ellipse_fill() {
        use crate::fill::{Fill, Span};

        let res: Vec<_> = Ellipse::new((0, 0), 0, 0).fill().collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for &(c, a, b) in &[
            ((0, 0), 0, 0),
            ((0, 0), 5, 2),
            ((0, 0), 1, 4),
            ((3, -2), 6, 6),
            ((1, 1), 0, 5),
            ((1, 1), 5, 0),
        ] {
            let spans: Vec<_> = Ellipse::new(c, a, b).fill().collect();
            let outline: Vec<_> = Ellipse::new(c, a, b).collect();
            assert_fill_ok(&spans, &outline, "ellipse");
        }
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_ellipse_rect_fill() {
        use crate::fill::{Fill, Span};

        let res: Vec<_> = EllipseRect::new((0, 0), (0, 0)).fill().collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for &(p0, p1) in &[
            ((0, 0), (0, 0)),
            ((0, 0), (8, 4)),
            ((2, 3), (12, 10)),
            ((10, 1), (1, 8)),
            ((5, 5), (5, 12)),
        ] {
            let spans: Vec<_> = EllipseRect::new(p0, p1).fill().collect();
            let outline: Vec<_> = EllipseRect::new(p0, p1).collect();
            assert_fill_ok(&spans, &outline, "ellipse_rect");
        }
    }
}
