//! Anti-aliased quadratic Bézier from Alois Zingl's `plotQuadBezierAA`.
//!
//! Coverage is inverted from Zingl's `setPixelAA`: `255` is fully on the curve,
//! `0` is fully off.

use crate::bezier::segments;
use crate::{LineAa, Point, PointAa};

fn coverage_f(zingl_fade: f64) -> u8 {
    255 - if zingl_fade <= 0.0 {
        0
    } else if zingl_fade >= 255.0 {
        255
    } else {
        zingl_fade as u8
    }
}

enum BezierAaState {
    Curve,
    Line(LineAa),
    Done,
}

/// One anti-aliased quadratic Bézier segment (gradient sign does not change).
///
/// Source: Zingl `plotQuadBezierSegAA`
struct QuadBezierAaSeg {
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
    state: BezierAaState,
    pending: [PointAa; 3],
    pending_len: u8,
    pending_i: u8,
}

impl QuadBezierAaSeg {
    fn new(p0: Point, p1: Point, p2: Point) -> Self {
        let (mut x0, mut y0) = p0;
        let (x1, y1) = p1;
        let (mut x2, mut y2) = p2;
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
            let dx = 4.0 * (sy as f64) * ((x1 - x0) as f64) * cur + (xx - xy) as f64;
            let dy = 4.0 * (sx as f64) * ((y0 - y1) as f64) * cur + (yy - xy) as f64;
            xx += xx;
            yy += yy;
            let err = dx + dy + xy as f64;
            return QuadBezierAaSeg {
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
                state: BezierAaState::Curve,
                pending: [((0, 0), 0); 3],
                pending_len: 0,
                pending_i: 0,
            };
        }

        QuadBezierAaSeg {
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
            state: BezierAaState::Line(LineAa::new((x0, y0), (x2, y2))),
            pending: [((0, 0), 0); 3],
            pending_len: 0,
            pending_i: 0,
        }
    }

    fn push(&mut self, p: Point, fade: u8) {
        self.pending[self.pending_len as usize] = (p, fade);
        self.pending_len += 1;
    }

    fn pop_pending(&mut self) -> Option<PointAa> {
        if self.pending_i < self.pending_len {
            let p = self.pending[self.pending_i as usize];
            self.pending_i += 1;
            if self.pending_i == self.pending_len {
                self.pending_i = 0;
                self.pending_len = 0;
            }
            Some(p)
        } else {
            None
        }
    }

    fn step_curve(&mut self) {
        let cur = (self.dx + self.xy as f64).min(-self.xy as f64 - self.dy);
        let mut ed = (self.dx + self.xy as f64).max(-self.xy as f64 - self.dy);
        ed += 2.0 * ed * cur * cur / (4.0 * ed * ed + cur * cur);
        let fade = coverage_f(255.0 * (self.err - self.dx - self.dy - self.xy as f64).abs() / ed);
        self.push((self.x0, self.y0), fade);

        if self.x0 == self.x2 || self.y0 == self.y2 {
            self.state = BezierAaState::Line(LineAa::new((self.x0, self.y0), (self.x2, self.y2)));
            return;
        }

        let x1 = self.x0;
        let cur = self.dx - self.err;
        let step_y = 2.0 * self.err + self.dy < 0.0;
        if 2.0 * self.err + self.dx > 0.0 {
            if self.err - self.dy < ed {
                self.push(
                    (self.x0, self.y0 + self.sy),
                    coverage_f(255.0 * (self.err - self.dy).abs() / ed),
                );
            }
            self.x0 += self.sx;
            self.dx -= self.xy as f64;
            self.dy += self.yy as f64;
            self.err += self.dy;
        }
        if step_y {
            if cur < ed {
                self.push((x1 + self.sx, self.y0), coverage_f(255.0 * cur.abs() / ed));
            }
            self.y0 += self.sy;
            self.dy -= self.xy as f64;
            self.dx += self.xx as f64;
            self.err += self.dx;
        }

        if !(self.dy < self.dx) {
            self.state = BezierAaState::Line(LineAa::new((self.x0, self.y0), (self.x2, self.y2)));
        }
    }
}

impl Iterator for QuadBezierAaSeg {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.pop_pending() {
                return Some(p);
            }
            match self.state {
                BezierAaState::Done => return None,
                BezierAaState::Line(ref mut line) => {
                    return line.next().or_else(|| {
                        self.state = BezierAaState::Done;
                        Some(((self.x2, self.y2), 255))
                    })
                }
                BezierAaState::Curve => self.step_curve(),
            }
        }
    }
}

/// Anti-aliased quadratic Bézier
///
/// Any control-point configuration is accepted; the curve is split at gradient
/// sign changes the same way as [`QuadBezier`](crate::QuadBezier).
///
/// Inclusive: `[p0, p2]`
/// Source: Zingl `plotQuadBezierAA`
pub struct QuadBezierAa {
    segs: [QuadBezierAaSeg; 3],
    n: u8,
    i: u8,
}

impl QuadBezierAa {
    /// Inclusive anti-aliased quadratic Bézier (`[p0, p2]`) with control
    /// point `p1`.
    pub fn new(p0: Point, p1: Point, p2: Point) -> Self {
        let (specs, n) = segments(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1);
        QuadBezierAa {
            segs: [
                QuadBezierAaSeg::new(specs[0].0, specs[0].1, specs[0].2),
                QuadBezierAaSeg::new(specs[1].0, specs[1].1, specs[1].2),
                QuadBezierAaSeg::new(specs[2].0, specs[2].1, specs[2].2),
            ],
            n: n as u8,
            i: 0,
        }
    }
}

impl Iterator for QuadBezierAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.n {
            match self.segs[self.i as usize].next() {
                Some(p) => return Some(p),
                None => self.i += 1,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::QuadBezierAa;
    use crate::Point;
    use crate::QuadBezier;
    use std::fs;
    use std::path::PathBuf;
    use std::vec::Vec;

    #[test]
    fn test_quad_bezier_aa() {
        let res: Vec<_> = QuadBezierAa::new((0, 0), (1, 3), (4, 3)).collect();
        assert_eq!(
            res,
            [
                ((0, 0), 255),
                ((1, 0), 19),
                ((0, 1), 153),
                ((0, 2), 30),
                ((1, 1), 135),
                ((1, 2), 218),
                ((1, 3), 22),
                ((2, 2), 129),
                ((2, 3), 152),
                ((3, 2), 18),
                ((3, 3), 233),
                ((3, 3), 255),
                ((4, 3), 255)
            ]
        );
    }

    /// An arch whose endpoints share an axis must still follow `QuadBezier`.
    /// `QuadBezierAa` splits at the same gradient-sign changes so the bulge
    /// is drawn instead of collapsing to the chord.
    #[test]
    fn test_quad_bezier_aa_follows_aliased_arch() {
        let p0 = (0, 20);
        let p1 = (20, 2);
        let p2 = (40, 20);

        let aliased: Vec<Point> = QuadBezier::new(p0, p1, p2).collect();
        let aa: Vec<(Point, u8)> = QuadBezierAa::new(p0, p1, p2)
            .filter(|(_, c)| *c > 0)
            .collect();

        let path = write_comparison_ppm("quad_bezier_aa_gap.ppm", p0, p1, p2, &aliased, &aa);
        std::eprintln!("wrote comparison image to {}", path.display());

        let missing: Vec<Point> = aliased
            .iter()
            .copied()
            .filter(|&p| !aa_covers(p, &aa))
            .collect();

        assert!(
            missing.is_empty(),
            "QuadBezierAa dropped {} aliased pixels (e.g. {:?}); see {}",
            missing.len(),
            missing.first(),
            path.display()
        );
    }

    fn aa_covers(p: Point, aa: &[(Point, u8)]) -> bool {
        aa.iter()
            .any(|&((x, y), c)| c > 0 && (x - p.0).abs() <= 1 && (y - p.1).abs() <= 1)
    }

    fn write_comparison_ppm(
        name: &str,
        p0: Point,
        p1: Point,
        p2: Point,
        aliased: &[Point],
        aa: &[(Point, u8)],
    ) -> PathBuf {
        let pad = 3;
        let xs = aliased
            .iter()
            .map(|p| p.0)
            .chain(aa.iter().map(|p| p.0 .0))
            .chain([p0.0, p1.0, p2.0]);
        let ys = aliased
            .iter()
            .map(|p| p.1)
            .chain(aa.iter().map(|p| p.0 .1))
            .chain([p0.1, p1.1, p2.1]);
        let min_x = xs.clone().min().unwrap() - pad;
        let max_x = xs.max().unwrap() + pad;
        let min_y = ys.clone().min().unwrap() - pad;
        let max_y = ys.max().unwrap() + pad;
        let w = (max_x - min_x + 1) as usize;
        let h = (max_y - min_y + 1) as usize;

        // Three panels: aliased | anti-aliased | overlay
        // overlay: green = both, red = aliased only, blue = AA only
        let panel = w + 1;
        let img_w = panel * 3 - 1;
        let scale = 8usize;
        let mut pix = std::vec![24u8; img_w * h * 3];

        let put = |buf: &mut [u8], px: usize, py: usize, r: u8, g: u8, b: u8| {
            if px < img_w && py < h {
                let i = (py * img_w + px) * 3;
                buf[i] = r;
                buf[i + 1] = g;
                buf[i + 2] = b;
            }
        };

        let to_xy = |p: Point| ((p.0 - min_x) as usize, (p.1 - min_y) as usize);

        for y in 0..h {
            put(&mut pix, w, y, 48, 48, 48);
            put(&mut pix, panel + w, y, 48, 48, 48);
        }

        for &p in aliased {
            let (x, y) = to_xy(p);
            put(&mut pix, x, y, 255, 255, 255);
            put(&mut pix, panel * 2 + x, y, 220, 40, 40);
        }
        for &(p, c) in aa {
            let (x, y) = to_xy(p);
            put(&mut pix, panel + x, y, c, c, c);
            let i = (y * img_w + (panel * 2 + x)) * 3;
            if i + 2 < pix.len() {
                if pix[i] == 220 && pix[i + 1] == 40 {
                    pix[i] = 40;
                    pix[i + 1] = 200;
                    pix[i + 2] = 40;
                } else {
                    pix[i] = 60;
                    pix[i + 1] = 120;
                    pix[i + 2] = 255;
                }
            }
        }
        for p in [p0, p2] {
            let (x, y) = to_xy(p);
            put(&mut pix, x, y, 255, 200, 0);
            put(&mut pix, panel + x, y, 255, 200, 0);
            put(&mut pix, panel * 2 + x, y, 255, 200, 0);
        }
        let (cx, cy) = to_xy(p1);
        put(&mut pix, cx, cy, 255, 140, 0);
        put(&mut pix, panel + cx, cy, 255, 140, 0);
        put(&mut pix, panel * 2 + cx, cy, 255, 140, 0);

        let sw = img_w * scale;
        let sh = h * scale;
        let mut scaled = std::vec![0u8; sw * sh * 3];
        for y in 0..h {
            for x in 0..img_w {
                let i = (y * img_w + x) * 3;
                let (r, g, b) = (pix[i], pix[i + 1], pix[i + 2]);
                for dy in 0..scale {
                    for dx in 0..scale {
                        let j = ((y * scale + dy) * sw + (x * scale + dx)) * 3;
                        scaled[j] = r;
                        scaled[j + 1] = g;
                        scaled[j + 2] = b;
                    }
                }
            }
        }
        let mut out = Vec::new();
        out.extend_from_slice(std::format!("P6\n{sw} {sh}\n255\n").as_bytes());
        out.extend_from_slice(&scaled);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, out).unwrap();
        path
    }
}
