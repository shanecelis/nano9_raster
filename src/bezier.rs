//! Quadratic Bézier curves from Alois Zingl's `plotQuadBezier`.

use crate::line::Line;
use crate::Point;

enum SegState {
    Curve,
    Line(Line),
    Done,
}

/// One quadratic Bézier segment (gradient sign does not change).
struct QuadBezierSeg {
    x0: isize,
    y0: isize,
    x2: isize,
    y2: isize,
    sx: isize,
    sy: isize,
    xx: i64,
    yy: i64,
    xy: i64,
    dx: f64,
    dy: f64,
    err: f64,
    state: SegState,
}

impl QuadBezierSeg {
    fn new(
        mut x0: isize,
        mut y0: isize,
        x1: isize,
        y1: isize,
        mut x2: isize,
        mut y2: isize,
    ) -> Self {
        let mut sx = x2 - x1;
        let mut sy = y2 - y1;
        let mut xx = (x0 - x1) as i64;
        let mut yy = (y0 - y1) as i64;
        let mut cur = (xx * sy as i64 - yy * sx as i64) as f64;

        if (sx as i64) * (sx as i64) + (sy as i64) * (sy as i64) > xx * xx + yy * yy {
            x2 = x0;
            x0 = sx + x1;
            y2 = y0;
            y0 = sy + y1;
            cur = -cur;
        }

        if cur != 0.0 {
            xx += sx as i64;
            sx = if x0 < x2 { 1 } else { -1 };
            xx *= sx as i64;
            yy += sy as i64;
            sy = if y0 < y2 { 1 } else { -1 };
            yy *= sy as i64;

            let mut xy = 2 * xx * yy;
            xx *= xx;
            yy *= yy;

            if cur * (sx as f64) * (sy as f64) < 0.0 {
                xx = -xx;
                yy = -yy;
                xy = -xy;
                cur = -cur;
            }

            let dx = 4.0 * (sy as f64) * cur * ((x1 - x0) as f64) + (xx - xy) as f64;
            let dy = 4.0 * (sx as f64) * cur * ((y0 - y1) as f64) + (yy - xy) as f64;
            xx += xx;
            yy += yy;
            let err = dx + dy + xy as f64;

            return QuadBezierSeg {
                x0,
                y0,
                x2,
                y2,
                sx,
                sy,
                xx,
                yy,
                xy,
                dx,
                dy,
                err,
                state: SegState::Curve,
            };
        }

        QuadBezierSeg {
            x0,
            y0,
            x2,
            y2,
            sx: 0,
            sy: 0,
            xx: 0,
            yy: 0,
            xy: 0,
            dx: 0.0,
            dy: 0.0,
            err: 0.0,
            state: SegState::Line(Line::new((x0, y0), (x2, y2))),
        }
    }
}

impl Iterator for QuadBezierSeg {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            SegState::Done => None,
            SegState::Line(ref mut line) => match line.next() {
                Some(p) => Some(p),
                None => {
                    let end = (self.x2, self.y2);
                    self.state = SegState::Done;
                    Some(end)
                }
            },
            SegState::Curve => {
                let p = (self.x0, self.y0);
                if self.x0 == self.x2 && self.y0 == self.y2 {
                    self.state = SegState::Done;
                    return Some(p);
                }

                let step_y = 2.0 * self.err < self.dx;
                if 2.0 * self.err > self.dy {
                    self.x0 += self.sx;
                    self.dx -= self.xy as f64;
                    self.dy += self.yy as f64;
                    self.err += self.dy;
                }
                if step_y {
                    self.y0 += self.sy;
                    self.dy -= self.xy as f64;
                    self.dx += self.xx as f64;
                    self.err += self.dx;
                }

                if !(self.dy < 0.0 && self.dx > 0.0) {
                    self.state = SegState::Line(Line::new((self.x0, self.y0), (self.x2, self.y2)));
                }

                Some(p)
            }
        }
    }
}

/// Quadratic Bézier from `p0` to `p2` with control point `p1`
///
/// Any control-point configuration is accepted; the curve is split at gradient
/// sign changes the same way as Zingl's `plotQuadBezier`.
///
/// Inclusive: `[p0, p2]`
pub struct QuadBezier {
    segs: [QuadBezierSeg; 3],
    n: u8,
    i: u8,
    last: Option<Point>,
}

impl QuadBezier {
    /// Inclusive pixels of the quadratic Bézier (`[p0, p2]`).
    pub fn new(p0: Point, p1: Point, p2: Point) -> Self {
        let (specs, n) = segments(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1);
        QuadBezier {
            segs: [
                QuadBezierSeg::new(
                    specs[0].0 .0,
                    specs[0].0 .1,
                    specs[0].1 .0,
                    specs[0].1 .1,
                    specs[0].2 .0,
                    specs[0].2 .1,
                ),
                QuadBezierSeg::new(
                    specs[1].0 .0,
                    specs[1].0 .1,
                    specs[1].1 .0,
                    specs[1].1 .1,
                    specs[1].2 .0,
                    specs[1].2 .1,
                ),
                QuadBezierSeg::new(
                    specs[2].0 .0,
                    specs[2].0 .1,
                    specs[2].1 .0,
                    specs[2].1 .1,
                    specs[2].2 .0,
                    specs[2].2 .1,
                ),
            ],
            n: n as u8,
            i: 0,
            last: None,
        }
    }
}

impl Iterator for QuadBezier {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.n {
            match self.segs[self.i as usize].next() {
                Some(p) => {
                    if self.last == Some(p) {
                        continue;
                    }
                    self.last = Some(p);
                    return Some(p);
                }
                None => self.i += 1,
            }
        }
        None
    }
}

/// C `floor(v + 0.5)` — half-up toward +∞.
///
/// Look for the [core::f64::math::floor] to become stable so you can remove this.
fn iround(v: f64) -> isize {
    libm::floor(v + 0.5) as isize
}

pub(crate) fn segments(
    mut x0: isize,
    mut y0: isize,
    mut x1: isize,
    mut y1: isize,
    mut x2: isize,
    mut y2: isize,
) -> ([(Point, Point, Point); 3], usize) {
    let mut out = [((0, 0), (0, 0), (0, 0)); 3];
    let mut n = 0;

    let mut x = x0 - x1;
    let mut y = y0 - y1;
    let mut t = (x0 - 2 * x1 + x2) as f64;

    if (x as i64) * ((x2 - x1) as i64) > 0 {
        if (y as i64) * ((y2 - y1) as i64) > 0 {
            if ((y0 - 2 * y1 + y2) as f64 / t * x as f64).abs() > y.abs() as f64 {
                x0 = x2;
                x2 = x + x1;
                y0 = y2;
                y2 = y + y1;
            }
        }
        t = (x0 - x1) as f64 / t;
        let mut r = (1.0 - t) * ((1.0 - t) * y0 as f64 + 2.0 * t * y1 as f64) + t * t * y2 as f64;
        t = ((x0 as i64 * x2 as i64 - x1 as i64 * x1 as i64) as f64) * t / ((x0 - x1) as f64);
        x = iround(t);
        y = iround(r);
        r = (y1 - y0) as f64 * (t - x0 as f64) / ((x1 - x0) as f64) + y0 as f64;
        out[n] = ((x0, y0), (x, iround(r)), (x, y));
        n += 1;
        r = (y1 - y2) as f64 * (t - x2 as f64) / ((x1 - x2) as f64) + y2 as f64;
        x0 = x;
        x1 = x;
        y0 = y;
        y1 = iround(r);
    }

    if ((y0 - y1) as i64) * ((y2 - y1) as i64) > 0 {
        t = (y0 - 2 * y1 + y2) as f64;
        t = (y0 - y1) as f64 / t;
        let mut r = (1.0 - t) * ((1.0 - t) * x0 as f64 + 2.0 * t * x1 as f64) + t * t * x2 as f64;
        t = ((y0 as i64 * y2 as i64 - y1 as i64 * y1 as i64) as f64) * t / ((y0 - y1) as f64);
        x = iround(r);
        y = iround(t);
        r = (x1 - x0) as f64 * (t - y0 as f64) / ((y1 - y0) as f64) + x0 as f64;
        out[n] = ((x0, y0), (iround(r), y), (x, y));
        n += 1;
        r = (x1 - x2) as f64 * (t - y2 as f64) / ((y1 - y2) as f64) + x2 as f64;
        x0 = x;
        x1 = iround(r);
        y0 = y;
        y1 = y;
    }

    out[n] = ((x0, y0), (x1, y1), (x2, y2));
    n += 1;
    (out, n)
}

#[cfg(test)]
mod tests {
    use super::QuadBezier;
    use std::vec::Vec;

    #[test]
    fn test_quad_bezier() {
        let res: Vec<_> = QuadBezier::new((0, 0), (2, 0), (4, 0)).collect();
        assert_eq!(res, [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);

        let res: Vec<_> = QuadBezier::new((0, 0), (2, 4), (4, 0)).collect();
        assert_eq!(res, [(0, 0), (1, 1), (2, 2), (4, 0), (3, 1), (2, 2)]);

        let res: Vec<_> = QuadBezier::new((0, 0), (1, 3), (4, 3)).collect();
        assert_eq!(res, [(0, 0), (0, 1), (1, 2), (2, 3), (3, 3), (4, 3)]);
    }
}
