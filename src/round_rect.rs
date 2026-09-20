//! Axis-aligned rounded rectangle.
//!
//! Inclusive corners `[p0, p1]`. A corner radius of `0` is a sharp rectangle.
//! Pico-8 offsets each nonzero corner arc by one more pixel than its requested
//! radius: rrect radius `r` uses a [`QuadArc`] of radius `r + 1`, centered
//! `r + 1` pixels from the corner. The arc radius is capped at half the short
//! side when opposite corners meet.

#[cfg(feature = "fill")]
use crate::fill::{Fill, Span};
use crate::Point;
use crate::QuadArc;

#[cfg(feature = "aa")]
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

#[cfg(feature = "aa")]
fn isqrt_round(n: u64) -> u64 {
    let x = isqrt(n);
    if n - x * x > x {
        x + 1
    } else {
        x
    }
}

pub(crate) fn clamp_radius(w: isize, h: isize, r: isize) -> isize {
    r.abs().min(w.min(h) / 2)
}

#[inline]
fn arc_radius(w: isize, h: isize, r: isize) -> isize {
    if r == 0 {
        0
    } else if w == 3 && h == 3 && r == 1 {
        // Pico-8's smallest maxed rrect is a solid 3×3, unlike circ r=1.
        0
    } else {
        (r + 1).min(w.min(h) / 2)
    }
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

/// Inclusive rounded rectangle outline built from four [`QuadArc`] corners.
pub struct RoundRect {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    /// Radius and inset of the actual corner arc.
    arc_r: isize,
    arc: QuadArc,
    phase: u8,
    pos: isize,
}

impl RoundRect {
    /// Inclusive rounded rect with opposite corners `p0` and `p1`.
    ///
    /// Negative radii are treated as their absolute value. The radius is
    /// clamped to `floor(min(width, height) / 2)`.
    #[inline]
    pub fn new(p0: Point, p1: Point, r: isize) -> Self {
        let (x0, y0, x1, y1, r) = normalize_rect(p0, p1, r);
        let arc_r = arc_radius(x1 - x0 + 1, y1 - y0 + 1, r);
        let (phase, pos) = if x0 == x1 {
            (9, y0)
        } else if y0 == y1 {
            (8, x0)
        } else {
            (0, 0)
        };
        RoundRect {
            x0,
            y0,
            x1,
            y1,
            arc_r,
            arc: QuadArc::new(arc_r),
            phase,
            pos,
        }
    }

    #[cfg(feature = "aa")]
    pub(crate) fn row_span(&self, y: isize) -> Option<(isize, isize)> {
        if y < self.y0 || y > self.y1 {
            return None;
        }
        if self.arc_r == 0 || (y >= self.y0 + self.arc_r && y <= self.y1 - self.arc_r) {
            return Some((self.x0, self.x1));
        }
        let cy = if y < self.y0 + self.arc_r {
            self.y0 + self.arc_r
        } else {
            self.y1 - self.arc_r
        };
        let dy = (y - cy).unsigned_abs() as u64;
        let rem = (self.arc_r as u64) * (self.arc_r as u64) - dy * dy;
        let dx = if dy == self.arc_r as u64 {
            isqrt(self.arc_r.saturating_sub(1) as u64)
        } else {
            isqrt_round(rem)
        } as isize;
        Some((self.x0 + self.arc_r - dx, self.x1 - self.arc_r + dx))
    }

    #[inline]
    fn advance_phase(&mut self) {
        if self.phase == 7 {
            self.phase = 10;
            return;
        }
        self.phase += 1;
        match self.phase {
            1..=3 => self.arc = QuadArc::new(self.arc_r),
            4 | 5 => self.pos = self.x0 + self.arc_r + 1,
            6 | 7 => self.pos = self.y0 + self.arc_r + 1,
            _ => {}
        }
    }

    #[inline]
    fn corner_point(&self, x: isize, y: isize) -> Point {
        match self.phase {
            0 => (self.x0 + self.arc_r - x, self.y0 + self.arc_r - y),
            1 => (self.x1 - self.arc_r + x, self.y0 + self.arc_r - y),
            2 => (self.x1 - self.arc_r + x, self.y1 - self.arc_r + y),
            3 => (self.x0 + self.arc_r - x, self.y1 - self.arc_r + y),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn owns_corner_point(&self, (x, y): Point) -> bool {
        let mx = (self.x0 + self.x1) / 2;
        let my = (self.y0 + self.y1) / 2;
        match self.phase {
            0 => x <= mx && y <= my,
            1 => x > mx && y <= my,
            2 => x > mx && y > my,
            3 => x <= mx && y > my,
            _ => false,
        }
    }
}

impl Iterator for RoundRect {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.phase {
                0..=3 => {
                    if let Some((x, y)) = self.arc.next() {
                        let point = self.corner_point(x, y);
                        if self.owns_corner_point(point) {
                            return Some(point);
                        }
                        continue;
                    }
                    self.advance_phase();
                }
                4 | 5 => {
                    let end = self.x1 - self.arc_r - 1;
                    if self.pos <= end {
                        let x = self.pos;
                        self.pos += 1;
                        let y = if self.phase == 4 { self.y0 } else { self.y1 };
                        return Some((x, y));
                    }
                    self.advance_phase();
                }
                6 | 7 => {
                    let end = self.y1 - self.arc_r - 1;
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
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    arc_r: isize,
    arc: QuadArc,
    last_arc_y: Option<isize>,
    pending: Option<Span>,
    bands_done: bool,
    middle_y: isize,
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
impl Fill for RoundRect {
    fn fill(self) -> impl Iterator<Item = Span> {
        RoundRectFill {
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
            arc_r: self.arc_r,
            arc: QuadArc::new(self.arc_r),
            last_arc_y: None,
            pending: None,
            bands_done: self.arc_r == 0,
            middle_y: if self.arc_r == 0 {
                self.y0
            } else {
                self.y0 + self.arc_r + 1
            },
        }
    }
}

#[cfg(feature = "fill")]
impl RoundRectFill {
    #[inline]
    fn arc_span(&self, x: isize, y: isize, row: isize) -> Span {
        debug_assert!(y <= self.arc_r);
        Span {
            x0: self.x0 + self.arc_r - x,
            x1: self.x1 - self.arc_r + x,
            y: row,
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for RoundRectFill {
    type Item = Span;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(span) = self.pending.take() {
            return Some(span);
        }

        if !self.bands_done {
            let mid_y = (self.y0 + self.y1) / 2;
            while let Some((x, y)) = self.arc.next() {
                if self.last_arc_y == Some(y) {
                    continue;
                }
                self.last_arc_y = Some(y);

                let top_y = self.y0 + self.arc_r - y;
                let bottom_y = self.y1 - self.arc_r + y;
                let top = (top_y <= mid_y).then(|| self.arc_span(x, y, top_y));
                let bottom = (bottom_y > mid_y).then(|| self.arc_span(x, y, bottom_y));
                match (top, bottom) {
                    (Some(top), Some(bottom)) => {
                        self.pending = Some(bottom);
                        return Some(top);
                    }
                    (Some(span), None) | (None, Some(span)) => return Some(span),
                    (None, None) => continue,
                }
            }
            self.bands_done = true;
        }

        let middle_end = if self.arc_r == 0 {
            self.y1
        } else {
            self.y1 - self.arc_r - 1
        };
        if self.middle_y > middle_end {
            return None;
        }
        let y = self.middle_y;
        self.middle_y += 1;
        Some(Span {
            x0: self.x0,
            x1: self.x1,
            y,
        })
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

    #[test]
    fn negative_radius_and_reversed_corners_match() {
        let expected: Vec<_> = RoundRect::new((0, 0), (12, 8), 3).collect();
        assert_eq!(
            expected,
            RoundRect::new((12, 8), (0, 0), 3).collect::<Vec<_>>()
        );
        assert_eq!(
            expected,
            RoundRect::new((0, 0), (12, 8), -3).collect::<Vec<_>>()
        );
    }

    #[test]
    fn handles_sharp_and_degenerate_rects() {
        let mut sharp: Vec<_> = RoundRect::new((0, 0), (3, 2), 0).collect();
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
            RoundRect::new((2, 1), (2, 4), 2).collect::<Vec<_>>(),
            [(2, 1), (2, 2), (2, 3), (2, 4)]
        );
        assert_eq!(
            RoundRect::new((1, 3), (4, 3), 2).collect::<Vec<_>>(),
            [(1, 3), (2, 3), (3, 3), (4, 3)]
        );
    }

    // #[test]
    // fn contains_each_translated_quad_arc() {
    //     let (x0, y0, x1, y1, r) = (2, 3, 18, 13, 4);
    //     let points: Vec<_> = RoundRect::new((x0, y0), (x1, y1), r).collect();
    //     for (x, y) in QuadArc::new(r) {
    //         assert!(points.contains(&(x0 + r - x, y0 + r - y)));
    //         assert!(points.contains(&(x1 - r + x, y0 + r - y)));
    //         assert!(points.contains(&(x1 - r + x, y1 - r + y)));
    //         assert!(points.contains(&(x0 + r - x, y1 - r + y)));
    //     }
    // }

    #[test]
    fn never_repeats_a_pixel() {
        for x1 in 0..16 {
            for y1 in 0..12 {
                for r in 0..8 {
                    let mut points: Vec<_> = RoundRect::new((0, 0), (x1, y1), r).collect();
                    let len = points.len();
                    points.sort_unstable();
                    points.dedup();
                    assert_eq!(points.len(), len, "{x1}x{y1} r={r}");
                }
            }
        }
    }
}
