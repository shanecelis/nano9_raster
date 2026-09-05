//! Axis-aligned rounded rectangle.
//!
//! Inclusive corners `[p0, p1]`. A corner radius of `0` is a sharp rectangle.
//! Larger radii are quarter-disks: a pixel is inside a corner when
//! `(x − cx)² + (y − cy)² ≤ r²`. The radius is clamped so `2r ≤ min(w, h) − 2`.

#[cfg(feature = "fill")]
use crate::fill::{Fill, Span};
use crate::Point;

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
    use super::RoundRect;
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
}
