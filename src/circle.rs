//! Midpoint circle from Alois Zingl's `plotCircle`.

#[cfg(feature = "fill")]
use crate::fill::{Fill, Span};
use crate::Point;

/// Iterator over the pixels of a circle
pub struct Circle {
    xm: isize,
    ym: isize,
    x: isize,
    y: isize,
    err: isize,
    quad: u8,
    done: bool,
}

impl Circle {
    /// Closed circle centered at `center` with the given `radius`.
    ///
    /// A radius of `0` yields the center point once. Negative radii are treated
    /// as their absolute value.
    #[inline]
    pub fn new(center: Point, radius: isize) -> Self {
        let r = radius.abs();
        if r == 0 {
            return Circle {
                xm: center.0,
                ym: center.1,
                x: 0,
                y: 0,
                err: 0,
                quad: 4,
                done: false,
            };
        }

        Circle {
            xm: center.0,
            ym: center.1,
            x: -r,
            y: 0,
            err: 2 - 2 * r,
            quad: 0,
            done: false,
        }
    }

    #[inline]
    fn points4(&self) -> [Point; 4] {
        [
            (self.xm - self.x, self.ym + self.y),
            (self.xm - self.y, self.ym - self.x),
            (self.xm + self.x, self.ym - self.y),
            (self.xm + self.y, self.ym + self.x),
        ]
    }

    #[inline]
    fn advance(&mut self) {
        let r = self.err;
        if r <= self.y {
            self.y += 1;
            self.err += self.y * 2 + 1;
        }
        if r > self.x || self.err > self.y {
            self.x += 1;
            self.err += self.x * 2 + 1;
        }
        if self.x >= 0 {
            self.done = true;
        }
    }

    /// Call `f` with every outline pixel. Faster than the iterator when the
    /// body can be inlined (four plots per step, no per-pixel `next`).
    #[inline]
    pub fn for_each<F: FnMut(Point)>(mut self, mut f: F) {
        if self.quad == 4 {
            f((self.xm, self.ym));
            return;
        }
        while !self.done {
            let pts = self.points4();
            f(pts[0]);
            f(pts[1]);
            f(pts[2]);
            f(pts[3]);
            self.advance();
        }
    }
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(feature = "fill")))]
impl Fill for Circle {
    #[inline]
    fn fill(self) -> impl Iterator<Item = Span> {
        CircleFill {
            c: self,
            pending: [Span { x0: 0, x1: 0, y: 0 }; 4],
            pending_len: 0,
            pending_i: 0,
            last_y: None,
            last_x: None,
            open_py: None,
            open_my: None,
            open_px: None,
            open_mx: None,
            finished: false,
        }
    }
}

/// Iterator over [`Span`] chords of a filled [`Circle`]. Inclusive `[x0, x1]`.
#[cfg(feature = "fill")]
pub(crate) struct CircleFill {
    c: Circle,
    pending: [Span; 4],
    pending_len: u8,
    pending_i: u8,
    last_y: Option<isize>,
    last_x: Option<isize>,
    open_py: Option<Span>,
    open_my: Option<Span>,
    open_px: Option<Span>,
    open_mx: Option<Span>,
    finished: bool,
}

#[cfg(feature = "fill")]
fn widen(open: &mut Option<Span>, x0: isize, x1: isize, y: isize) {
    *open = Some(match *open {
        None => Span { x0, x1, y },
        Some(h) => Span {
            x0: h.x0.min(x0),
            x1: h.x1.max(x1),
            y,
        },
    });
}

#[cfg(feature = "fill")]
impl CircleFill {
    fn push(&mut self, h: Span) {
        self.pending[self.pending_len as usize] = h;
        self.pending_len += 1;
    }

    fn take_open(&mut self, open: &mut Option<Span>) {
        if let Some(h) = open.take() {
            self.push(h);
        }
    }

    fn absorb(&mut self) {
        let xm = self.c.xm;
        let ym = self.c.ym;
        let x = self.c.x;
        let y = self.c.y;
        if y <= x.abs() {
            let x0 = xm + x;
            let x1 = xm - x;
            widen(&mut self.open_py, x0, x1, ym + y);
            if y != 0 {
                widen(&mut self.open_my, x0, x1, ym - y);
            }
        }
        if x.abs() > y {
            let x0 = xm - y;
            let x1 = xm + y;
            widen(&mut self.open_px, x0, x1, ym + x);
            if x != 0 {
                widen(&mut self.open_mx, x0, x1, ym - x);
            }
        }
    }
}

#[cfg(feature = "fill")]
impl Iterator for CircleFill {
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

            if self.c.quad == 4 {
                if self.c.done {
                    return None;
                }
                self.c.done = true;
                return Some(Span {
                    x0: self.c.xm,
                    x1: self.c.xm,
                    y: self.c.ym,
                });
            }

            if self.finished {
                return None;
            }

            if self.c.done {
                let mut py = self.open_py.take();
                let mut my = self.open_my.take();
                let mut px = self.open_px.take();
                let mut mx = self.open_mx.take();
                self.take_open(&mut py);
                self.take_open(&mut my);
                self.take_open(&mut px);
                self.take_open(&mut mx);
                self.finished = true;
                continue;
            }

            if self.last_y.is_some() && self.last_y != Some(self.c.y) {
                let mut py = self.open_py.take();
                let mut my = self.open_my.take();
                self.take_open(&mut py);
                self.take_open(&mut my);
            }
            if self.last_x.is_some() && self.last_x != Some(self.c.x) {
                let mut px = self.open_px.take();
                let mut mx = self.open_mx.take();
                self.take_open(&mut px);
                self.take_open(&mut mx);
            }

            self.absorb();
            self.last_y = Some(self.c.y);
            self.last_x = Some(self.c.x);
            self.c.advance();
        }
    }
}

impl Iterator for Circle {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // r == 0: a single center pixel.
        if self.quad == 4 {
            self.done = true;
            return Some((self.xm, self.ym));
        }

        let p = self.points4()[self.quad as usize];

        if self.quad < 3 {
            self.quad += 1;
        } else {
            self.quad = 0;
            self.advance();
        }

        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::Circle;
    #[cfg(feature = "fill")]
    use crate::Point;
    use std::vec::Vec;

    #[test]
    fn test_circle() {
        let res: Vec<_> = Circle::new((0, 0), 0).collect();
        assert_eq!(res, [(0, 0)]);

        let res: Vec<_> = Circle::new((0, 0), 1).collect();
        assert_eq!(res, [(1, 0), (0, 1), (-1, 0), (0, -1)]);

        let res: Vec<_> = Circle::new((5, 5), 2).collect();
        assert_eq!(
            res,
            [
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
                (7, 4)
            ]
        );

        let res: Vec<_> = Circle::new((0, 0), 4).collect();
        assert_eq!(
            res,
            [
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
                (4, -1)
            ]
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
        use crate::fill::Fill;

        let mut grid = [0u8; 8];
        for h in Circle::new((3, 3), 3).fill() {
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
        use crate::fill::{Fill, Span};

        let res: Vec<_> = Circle::new((0, 0), 0).fill().collect();
        assert_eq!(res, [Span { x0: 0, x1: 0, y: 0 }]);

        for r in 0..16 {
            let spans: Vec<_> = Circle::new((3, -2), r).fill().collect();

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
