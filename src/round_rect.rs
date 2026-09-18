//! Axis-aligned rounded rectangle.
//!
//! Inclusive corners `[p0, p1]`. A corner radius of `0` is a sharp rectangle.
//! Larger radii are quarter-disks: a pixel is inside a corner when
//! `(x − cx)² + (y − cy)² ≤ r²`. The radius is clamped so `2r ≤ min(w, h) − 2`.

#[cfg(feature = "fill")]
use crate::fill::{Fill, Span};
use crate::{Point, QuadArc};

fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

pub(crate) fn clamp_radius(w: isize, h: isize, r: isize) -> isize {
    let max = (w.min(h) - 2).max(0) / 2;
    r.abs().min(max)
}

pub(crate) fn normalize_rect(
    p0: Point,
    p1: Point,
    r: isize,
) -> (isize, isize, isize, isize, isize) {
    let (x0, x1) = (p0.0.min(p1.0), p0.0.max(p1.0));
    let (y0, y1) = (p0.1.min(p1.1), p0.1.max(p1.1));
    let r = clamp_radius(x1 - x0 + 1, y1 - y0 + 1, r);
    (x0, y0, x1, y1, r)
}

/// Inclusive rounded rectangle outline
pub struct RoundRect {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    r: isize,
    y: isize,
    x: isize,
    lx: isize,
    rx: isize,
    prev: Option<(isize, isize)>,
    next: Option<(isize, isize)>,
    done: bool,
}

impl RoundRect {
    /// Inclusive rounded rect with opposite corners `p0` and `p1`.
    ///
    /// Negative radii are treated as their absolute value. The radius is
    /// clamped to `(min(width, height) − 2) / 2`.
    pub fn new(p0: Point, p1: Point, r: isize) -> Self {
        let (x0, y0, x1, y1, r) = normalize_rect(p0, p1, r);
        let mut rr = RoundRect {
            x0,
            y0,
            x1,
            y1,
            r,
            y: y0,
            x: x0,
            lx: x0,
            rx: x1,
            prev: None,
            next: None,
            done: false,
        };
        rr.enter_row(y0);
        rr
    }

    pub(crate) fn row_span(&self, y: isize) -> Option<(isize, isize)> {
        if y < self.y0 || y > self.y1 {
            return None;
        }
        if self.r == 0 || (y >= self.y0 + self.r && y <= self.y1 - self.r) {
            return Some((self.x0, self.x1));
        }
        let cy = if y < self.y0 + self.r {
            self.y0 + self.r
        } else {
            self.y1 - self.r
        };
        let dy = (y - cy).unsigned_abs() as u64;
        let rem = (self.r as u64) * (self.r as u64) - dy * dy;
        let dx = isqrt(rem) as isize;
        Some((self.x0 + self.r - dx, self.x1 - self.r + dx))
    }

    fn enter_row(&mut self, y: isize) {
        match self.row_span(y) {
            Some((lx, rx)) => {
                self.y = y;
                self.lx = lx;
                self.rx = rx;
                self.x = lx;
                self.prev = self.row_span(y - 1);
                self.next = self.row_span(y + 1);
            }
            None => self.done = true,
        }
    }

    fn is_row_outline(&self, x: isize) -> bool {
        if x == self.lx || x == self.rx {
            return true;
        }
        match self.prev {
            None => return true,
            Some((pl, pr)) if x < pl || x > pr => return true,
            _ => {}
        }
        match self.next {
            None => true,
            Some((nl, nr)) => x < nl || x > nr,
        }
    }
}

impl Iterator for RoundRect {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done {
            if self.x <= self.rx {
                let x = self.x;
                self.x += 1;
                if self.is_row_outline(x) {
                    return Some((x, self.y));
                }
            } else if self.y < self.y1 {
                self.enter_row(self.y + 1);
            } else {
                self.done = true;
            }
        }
        None
    }
}

/// Inclusive rounded rectangle outline built from four [`QuadArc`] corners.
pub struct RoundRect2 {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    r: isize,
    arc: QuadArc,
    phase: u8,
    pos: isize,
}

impl RoundRect2 {
    /// Inclusive rounded rect with opposite corners `p0` and `p1`.
    ///
    /// Negative radii are treated as their absolute value. The radius is
    /// clamped to `(min(width, height) − 2) / 2`.
    #[inline]
    pub fn new(p0: Point, p1: Point, r: isize) -> Self {
        let (x0, y0, x1, y1, r) = normalize_rect(p0, p1, r);
        let (phase, pos) = if x0 == x1 {
            (9, y0)
        } else if y0 == y1 {
            (8, x0)
        } else {
            (0, 0)
        };
        RoundRect2 {
            x0,
            y0,
            x1,
            y1,
            r,
            arc: QuadArc::new(r),
            phase,
            pos,
        }
    }

    #[inline]
    fn advance_phase(&mut self) {
        if self.phase == 7 {
            self.phase = 10;
            return;
        }
        self.phase += 1;
        match self.phase {
            1..=3 => self.arc = QuadArc::new(self.r),
            4 | 5 => self.pos = self.x0 + self.r + 1,
            6 | 7 => self.pos = self.y0 + self.r + 1,
            _ => {}
        }
    }

    #[inline]
    fn corner_point(&self, x: isize, y: isize) -> Point {
        match self.phase {
            0 => (self.x0 + self.r - x, self.y0 + self.r - y),
            1 => (self.x1 - self.r + x, self.y0 + self.r - y),
            2 => (self.x1 - self.r + x, self.y1 - self.r + y),
            3 => (self.x0 + self.r - x, self.y1 - self.r + y),
            _ => unreachable!(),
        }
    }
}

impl Iterator for RoundRect2 {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.phase {
                0..=3 => {
                    if let Some((x, y)) = self.arc.next() {
                        return Some(self.corner_point(x, y));
                    }
                    self.advance_phase();
                }
                4 | 5 => {
                    let end = self.x1 - self.r - 1;
                    if self.pos <= end {
                        let x = self.pos;
                        self.pos += 1;
                        let y = if self.phase == 4 { self.y0 } else { self.y1 };
                        return Some((x, y));
                    }
                    self.advance_phase();
                }
                6 | 7 => {
                    let end = self.y1 - self.r - 1;
                    if self.pos <= end {
                        let y = self.pos;
                        self.pos += 1;
                        let x = if self.phase == 6 { self.x0 } else { self.x1 };
                        return Some((x, y));
                    }
                    self.advance_phase();
                }
                8 => {
                    if self.pos <= self.x1 {
                        let x = self.pos;
                        self.pos += 1;
                        return Some((x, self.y0));
                    }
                    self.phase = 10;
                }
                9 => {
                    if self.pos <= self.y1 {
                        let y = self.pos;
                        self.pos += 1;
                        return Some((self.x0, y));
                    }
                    self.phase = 10;
                }
                _ => return None,
            }
        }
    }
}

#[cfg(feature = "fill")]
struct RoundRectFill {
    y: isize,
    y1: isize,
    geom: RoundRect,
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
impl Fill for RoundRect {
    fn fill(self) -> impl Iterator<Item = Span> {
        RoundRectFill {
            y: self.y0,
            y1: self.y1,
            geom: self,
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for RoundRectFill {
    type Item = Span;

    fn next(&mut self) -> Option<Self::Item> {
        while self.y <= self.y1 {
            let y = self.y;
            self.y += 1;
            if let Some((x0, x1)) = self.geom.row_span(y) {
                return Some(Span { x0, x1, y });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{RoundRect, RoundRect2};
    use crate::QuadArc;
    use std::vec::Vec;

    #[test]
    fn reversed_corners_match() {
        let a: Vec<_> = RoundRect::new((0, 0), (7, 7), 2).collect();
        let b: Vec<_> = RoundRect::new((7, 7), (0, 0), 2).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn negative_radius_matches_abs() {
        let pos: Vec<_> = RoundRect::new((0, 0), (7, 7), 2).collect();
        let neg: Vec<_> = RoundRect::new((0, 0), (7, 7), -2).collect();
        assert_eq!(pos, neg);
    }

    #[test]
    fn round_rect2_negative_radius_and_reversed_corners_match() {
        let expected: Vec<_> = RoundRect2::new((0, 0), (12, 8), 3).collect();
        assert_eq!(
            expected,
            RoundRect2::new((12, 8), (0, 0), 3).collect::<Vec<_>>()
        );
        assert_eq!(
            expected,
            RoundRect2::new((0, 0), (12, 8), -3).collect::<Vec<_>>()
        );
    }

    #[test]
    fn round_rect2_handles_sharp_and_degenerate_rects() {
        let mut sharp: Vec<_> = RoundRect2::new((0, 0), (3, 2), 0).collect();
        sharp.sort_unstable();
        assert_eq!(
            sharp,
            [
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 2),
                (2, 0),
                (2, 2),
                (3, 0),
                (3, 1),
                (3, 2)
            ]
        );
        assert_eq!(
            RoundRect2::new((2, 1), (2, 4), 2).collect::<Vec<_>>(),
            [(2, 1), (2, 2), (2, 3), (2, 4)]
        );
        assert_eq!(
            RoundRect2::new((1, 3), (4, 3), 2).collect::<Vec<_>>(),
            [(1, 3), (2, 3), (3, 3), (4, 3)]
        );
    }

    #[test]
    fn round_rect2_contains_each_translated_quad_arc() {
        let (x0, y0, x1, y1, r) = (2, 3, 18, 13, 4);
        let points: Vec<_> = RoundRect2::new((x0, y0), (x1, y1), r).collect();
        for (x, y) in QuadArc::new(r) {
            assert!(points.contains(&(x0 + r - x, y0 + r - y)));
            assert!(points.contains(&(x1 - r + x, y0 + r - y)));
            assert!(points.contains(&(x1 - r + x, y1 - r + y)));
            assert!(points.contains(&(x0 + r - x, y1 - r + y)));
        }
    }

    #[test]
    fn round_rect2_never_repeats_a_pixel() {
        for x1 in 0..16 {
            for y1 in 0..12 {
                for r in 0..8 {
                    let mut points: Vec<_> = RoundRect2::new((0, 0), (x1, y1), r).collect();
                    let len = points.len();
                    points.sort_unstable();
                    points.dedup();
                    assert_eq!(points.len(), len, "{x1}x{y1} r={r}");
                }
            }
        }
    }
}
