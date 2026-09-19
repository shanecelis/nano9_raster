
use crate::{Point};

/// We have to use our own `round_rect::QuadArc` that is _slightly_ different
/// than the `crate::QuadArc` in order to properly match the rounded rects.
#[derive(Clone, Copy)]
pub(crate) struct QuadArc {
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
