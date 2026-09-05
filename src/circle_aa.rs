//! Anti-aliased circle from Fu and Niu's integral algorithm.
//!
//! B. Fu and L. Niu, "Integral Algorithm for Generating Anti-Aliasing Circle
//! Based on Bresenham Algorithm", *Advanced Materials Research*,
//! 490–495:1202–1206, 2012.
//!
//! Coverage is the crate convention: `255` is fully on the curve, `0` is
//! fully off.

use arraydeque::ArrayDeque;

#[cfg(feature = "fill")]
use crate::fill::{Fill, Plot, Span};
use crate::{Point, PointAa};

/// Anti-aliased axis-aligned circle
///
/// Integer-only. Walks the first octant with the Bresenham circle decision
/// and derives each pixel's coverage from the signed distance to the true
/// arc, `f / 2y`, where `f = x² + y² − r²` is maintained incrementally
/// (Fu & Niu, 2012). Each step yields the Bresenham pixel plus its radial
/// neighbor, mirrored eight ways.
///
/// Near the octant seams a pixel may be yielded more than once with
/// different coverage (also true of [`QuadBezierAa`](crate::QuadBezierAa));
/// blend with `max` when compositing.
pub struct CircleAa {
    xm: isize,
    ym: isize,
    #[cfg(feature = "fill")]
    r: isize,
    x: isize,
    y: isize,
    /// `x² + y² − r²` for the current octant pixel `(x, y)`.
    f: isize,
    pending: ArrayDeque<PointAa, 16>,
    done: bool,
}

impl CircleAa {
    /// Closed anti-aliased circle centered at `center` with the given
    /// `radius`.
    ///
    /// A radius of `0` yields the center point once. Negative radii are
    /// treated as their absolute value.
    pub fn new(center: Point, radius: isize) -> Self {
        let r = radius.abs();
        let mut c = CircleAa {
            xm: center.0,
            ym: center.1,
            #[cfg(feature = "fill")]
            r,
            x: 0,
            y: r,
            f: 0,
            pending: ArrayDeque::new(),
            done: false,
        };
        if r == 0 {
            c.push(center, 255);
            c.done = true;
        }
        c
    }

    fn push(&mut self, p: Point, coverage: u8) {
        self.pending
            .push_back((p, coverage))
            .expect("CircleAa pending overflow");
    }

    /// Push the eight symmetric copies of octant pixel `(x, y)`, skipping the
    /// duplicates that arise on the axes (`x == 0`, `y == 0`) and the
    /// diagonal (`x == y`).
    fn push8(&mut self, x: isize, y: isize, coverage: u8) {
        let (xm, ym) = (self.xm, self.ym);
        let candidates = [
            (xm + x, ym + y),
            (xm - x, ym + y),
            (xm + x, ym - y),
            (xm - x, ym - y),
            (xm + y, ym + x),
            (xm - y, ym + x),
            (xm + y, ym - x),
            (xm - y, ym - x),
        ];
        let start = self.pending.len();
        'candidate: for p in candidates {
            for prior in self.pending.iter().skip(start) {
                if prior.0 == p {
                    continue 'candidate;
                }
            }
            self.push(p, coverage);
        }
    }

    fn step(&mut self) {
        // Signed vertical distance from pixel (x, y) to the arc is f / 2y up
        // to a dropped ε²/2y term (Fu & Niu eq. 3; their numerators
        // D ± (2y − 1) equal 2f).
        let fade = (255 * self.f.abs() / (2 * self.y)).min(255) as u8;
        self.push8(self.x, self.y, 255 - fade);
        if fade > 0 {
            // f > 0: pixel is outside the arc, so the arc continues toward
            // the center (y − 1); f < 0: inside, toward y + 1. The neighbor
            // receives the remaining coverage.
            let ny = if self.f > 0 { self.y - 1 } else { self.y + 1 };
            self.push8(self.x, ny, fade);
        }

        // Bresenham advance. The paper's decision parameter for column x + 1
        // is D = 2·f(x+1, y) − 2y + 1; D ≥ 0 steps down to y − 1.
        self.x += 1;
        self.f += 2 * self.x - 1;
        if 2 * self.f - 2 * self.y + 1 >= 0 {
            self.f -= 2 * self.y - 1;
            self.y -= 1;
        }
        if self.x > self.y {
            self.done = true;
        }
    }
}

impl Iterator for CircleAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.pending.pop_front() {
                return Some(p);
            }
            if self.done {
                return None;
            }
            self.step();
        }
    }
}

#[cfg(feature = "fill")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "aa", feature = "circle", feature = "fill"))))]
impl Fill<Plot> for CircleAa {
    /// Filled anti-aliased disk: one solid [`Span`] per interior row plus
    /// per-pixel edge coverage [`Plot::Point`]s.
    ///
    /// Integer-only, from Vadillo's squared-distance formulation: a pixel is
    /// solid when `d² < r² − r` (inside the `r − ½` circle up to a dropped ¼)
    /// and fades linearly to zero at `d² = r² + r` with coverage
    /// `255·(r² + r − d²) / 2r`.
    ///
    /// J. R. Vadillo, "A novel technique to draw antialiased circles without
    /// floating point math nor square root", Versa Design S.L., 2023.
    #[inline]
    fn fill(self) -> impl Iterator<Item = Plot> {
        CircleAaFill::new((self.xm, self.ym), self.r)
    }
}

#[cfg(feature = "fill")]
enum FillPhase {
    /// `r == 0`: emit the center pixel once.
    Center,
    /// Left edge points, `dx` descending from `xe` toward `xs`.
    Left,
    /// The solid span, if any.
    Solid,
    /// Right edge points, `dx` ascending to `xe`.
    Right,
    Done,
}

/// Iterator over [`Plot`] instructions of a filled [`CircleAa`].
#[cfg(feature = "fill")]
struct CircleAaFill {
    xm: isize,
    ym: isize,
    r: isize,
    /// `(r − ½)² − ¼ = r² − r`; strictly inside is solid.
    rmin: i64,
    /// `(r + ½)² − ¼ = r² + r`; at or beyond is fully off.
    rmax: i64,
    /// Current row offset from the center, `−r ..= r`.
    dy: isize,
    /// Largest `dx` of a solid pixel on this row, `−1` if none.
    xs: isize,
    /// Largest `dx` of any covered pixel on this row.
    xe: isize,
    /// Column cursor within the current phase.
    dx: isize,
    phase: FillPhase,
}

#[cfg(feature = "fill")]
impl CircleAaFill {
    fn new((xm, ym): Point, r: isize) -> Self {
        let r64 = r as i64;
        let mut f = CircleAaFill {
            xm,
            ym,
            r,
            rmin: r64 * r64 - r64,
            rmax: r64 * r64 + r64,
            dy: -r,
            xs: -1,
            xe: 0,
            dx: 0,
            phase: if r == 0 {
                FillPhase::Center
            } else {
                FillPhase::Left
            },
        };
        if r > 0 {
            f.enter_row();
        }
        f
    }

    /// Largest `dx >= 0` with `dx² + dy² < limit`, or `-1` if none. `from` is
    /// the previous row's bound; both bounds are monotone per half-circle, so
    /// the two adjustment loops are amortized O(1) per row.
    fn bound(&self, limit: i64, from: isize) -> isize {
        let dy2 = (self.dy as i64) * (self.dy as i64);
        let mut dx = from.max(0);
        while ((dx + 1) as i64) * ((dx + 1) as i64) + dy2 < limit {
            dx += 1;
        }
        while dx >= 0 && (dx as i64) * (dx as i64) + dy2 >= limit {
            dx -= 1;
        }
        dx
    }

    fn enter_row(&mut self) {
        self.xs = self.bound(self.rmin, self.xs);
        self.xe = self.bound(self.rmax, self.xe);
        self.dx = self.xe;
        self.phase = FillPhase::Left;
    }

    /// Edge coverage `255·(rmax − d²) / 2r`; in the edge zone this lies in
    /// `0 ..= 255` without clamping.
    fn alpha(&self, dx: isize) -> u8 {
        let d2 = (dx as i64) * (dx as i64) + (self.dy as i64) * (self.dy as i64);
        (255 * (self.rmax - d2) / (2 * self.r as i64)) as u8
    }
}

#[cfg(feature = "fill")]
impl Iterator for CircleAaFill {
    type Item = Plot;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.phase {
                FillPhase::Center => {
                    self.phase = FillPhase::Done;
                    return Some(Plot::Point(((self.xm, self.ym), 255)));
                }
                FillPhase::Left => {
                    if self.dx > self.xs {
                        let dx = self.dx;
                        self.dx -= 1;
                        let a = self.alpha(dx);
                        if a > 0 {
                            return Some(Plot::Point(((self.xm - dx, self.ym + self.dy), a)));
                        }
                        continue;
                    }
                    self.phase = FillPhase::Solid;
                }
                FillPhase::Solid => {
                    self.phase = FillPhase::Right;
                    // The left pass already emitted dx == 0 when no span
                    // splits the row, so the right pass starts at 1.
                    self.dx = if self.xs < 0 { 1 } else { self.xs + 1 };
                    if self.xs >= 0 {
                        return Some(Plot::Span(Span {
                            x0: self.xm - self.xs,
                            x1: self.xm + self.xs,
                            y: self.ym + self.dy,
                        }));
                    }
                }
                FillPhase::Right => {
                    if self.dx <= self.xe {
                        let dx = self.dx;
                        self.dx += 1;
                        let a = self.alpha(dx);
                        if a > 0 {
                            return Some(Plot::Point(((self.xm + dx, self.ym + self.dy), a)));
                        }
                        continue;
                    }
                    if self.dy == self.r {
                        self.phase = FillPhase::Done;
                        return None;
                    }
                    self.dy += 1;
                    self.enter_row();
                }
                FillPhase::Done => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CircleAa;
    use std::vec::Vec;

    #[test]
    fn test_circle_aa_degenerate() {
        let res: Vec<_> = CircleAa::new((3, -2), 0).collect();
        assert_eq!(res, [((3, -2), 255)]);

        let res: Vec<_> = CircleAa::new((0, 0), 1).collect();
        assert_eq!(
            res,
            [((0, 1), 255), ((0, -1), 255), ((1, 0), 255), ((-1, 0), 255)]
        );
    }

    #[test]
    fn test_circle_aa_negative_radius() {
        let pos: Vec<_> = CircleAa::new((1, 2), 5).collect();
        let neg: Vec<_> = CircleAa::new((1, 2), -5).collect();
        assert_eq!(pos, neg);
    }

    #[test]
    fn test_circle_aa_r2() {
        let res: Vec<_> = CircleAa::new((0, 0), 2).collect();
        assert_eq!(
            res,
            [
                ((0, 2), 255),
                ((0, -2), 255),
                ((2, 0), 255),
                ((-2, 0), 255),
                ((1, 2), 192),
                ((-1, 2), 192),
                ((1, -2), 192),
                ((-1, -2), 192),
                ((2, 1), 192),
                ((-2, 1), 192),
                ((2, -1), 192),
                ((-2, -1), 192),
                ((1, 1), 63),
                ((-1, 1), 63),
                ((1, -1), 63),
                ((-1, -1), 63)
            ]
        );
    }

    /// The `r = 3` anti-aliased circle rendered onto an 8x8 grid with one hex
    /// nybble of opacity per pixel: `f` is fully on, `0` is off, and the most
    /// significant nybble is the leftmost column. Seam duplicates blend with
    /// `max`.
    #[test]
    fn test_circle_aa_shape() {
        let mut grid = [0u32; 8];
        for ((x, y), c) in CircleAa::new((3, 3), 3) {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            let shift = ((7 - x) * 4) as u32;
            let nyb = (c as u32) >> 4;
            let old = (grid[y as usize] >> shift) & 0xf;
            grid[y as usize] = (grid[y as usize] & !(0xf << shift)) | (old.max(nyb) << shift);
        }
        #[rustfmt::skip]
        assert_eq!(grid, [
            0x03dfd300,
            0x3c202c30,
            0xd20002d0,
            0xf00000f0,
            0xd20002d0,
            0x3c202c30,
            0x03dfd300,
            0x00000000,
        ]);
    }

    /// After max-blending duplicates, every pixel's coverage must be close to
    /// `255·(1 − d)` where `d` is the distance to the true arc along the
    /// pixel's minor axis.
    #[test]
    fn test_circle_aa_accuracy() {
        use std::collections::BTreeMap;

        for r in 2isize..=64 {
            let mut blended: BTreeMap<(isize, isize), u8> = BTreeMap::new();
            for (p, coverage) in CircleAa::new((0, 0), r) {
                let entry = blended.entry(p).or_insert(0);
                *entry = (*entry).max(coverage);
            }
            for (&(px, py), &coverage) in &blended {
                let (fx, fy) = (px as f64, py as f64);
                let rf = r as f64;
                let dist = if fy.abs() >= fx.abs() {
                    (fy.abs() - (rf * rf - fx * fx).sqrt()).abs()
                } else {
                    (fx.abs() - (rf * rf - fy * fy).sqrt()).abs()
                };
                let expected = 255.0 * (1.0 - dist).max(0.0);
                let err = (coverage as f64 - expected).abs();
                // The dropped ε²/2y term dominates at small radii.
                let tol = if r < 8 { 40.0 } else { 16.0 };
                assert!(
                    err <= tol,
                    "r={r} p=({px},{py}) coverage={coverage} expected={expected:.1}"
                );
            }
        }
    }

    /// The point set is symmetric under reflection across both axes and the
    /// diagonal.
    #[test]
    fn test_circle_aa_symmetric() {
        use std::collections::BTreeSet;

        for r in 0isize..=32 {
            let pts: BTreeSet<_> = CircleAa::new((0, 0), r).collect();
            for &((x, y), c) in &pts {
                assert!(pts.contains(&((-x, y), c)), "r={r} mirror x of ({x},{y})");
                assert!(pts.contains(&((x, -y), c)), "r={r} mirror y of ({x},{y})");
                assert!(pts.contains(&((y, x), c)), "r={r} swap of ({x},{y})");
            }
        }
    }

    /// Away from the octant seams, each column holds the Bresenham pixel and
    /// its neighbor, and they split full coverage between them.
    #[test]
    fn test_circle_aa_coverage_splits() {
        for r in 4isize..=32 {
            let pts: Vec<_> = CircleAa::new((0, 0), r).collect();
            // Strict first-octant interior: seam mirrors land at py <= px + 1,
            // so columns whose pixels all sit at py >= px + 3 are clean.
            for &((x, y), c) in &pts {
                if !(x > 0 && y >= x + 3 && c >= 128) {
                    continue;
                }
                // `(x, y)` is a main pixel. Its complement, if any, is the
                // vertical neighbor.
                if c == 255 {
                    continue;
                }
                let partner: isize = pts
                    .iter()
                    .filter(|&&((px, py), _)| px == x && (py - y).abs() == 1)
                    .map(|&(_, pc)| pc as isize)
                    .sum();
                assert_eq!(
                    c as isize + partner,
                    255,
                    "r={r} main ({x},{y}) coverage {c} partner {partner}"
                );
            }
        }
    }

    /// Expand a fill into `(point, alpha)` pixels, asserting span invariants.
    #[cfg(feature = "fill")]
    fn expand_fill(center: (isize, isize), r: isize) -> Vec<((isize, isize), u8)> {
        use crate::fill::{Fill, Plot};

        let mut pixels = Vec::new();
        let mut span_rows = Vec::new();
        for plot in CircleAa::new(center, r).fill() {
            match plot {
                Plot::Span(h) => {
                    assert!(h.x0 <= h.x1, "r={r} {h:?}");
                    assert!(!span_rows.contains(&h.y), "r={r} second span on {}", h.y);
                    span_rows.push(h.y);
                    for x in h.x0..=h.x1 {
                        pixels.push(((x, h.y), 255));
                    }
                }
                Plot::Point((p, a)) => {
                    assert!(a > 0, "r={r} zero-alpha point {p:?}");
                    pixels.push((p, a));
                }
            }
        }
        pixels
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_small() {
        use crate::fill::{Plot, Span};
        use crate::Fill;

        let res: Vec<_> = CircleAa::new((3, -2), 0).fill().collect();
        assert_eq!(res, [Plot::Point(((3, -2), 255))]);

        let res: Vec<_> = CircleAa::new((0, 0), 1).fill().collect();
        assert_eq!(
            res,
            [
                Plot::Point(((0, -1), 127)),
                Plot::Point(((-1, 0), 127)),
                Plot::Point(((0, 0), 255)),
                Plot::Point(((1, 0), 127)),
                Plot::Point(((0, 1), 127)),
            ]
        );

        let res: Vec<_> = CircleAa::new((0, 0), 2).fill().collect();
        assert_eq!(
            res,
            [
                Plot::Point(((-1, -2), 63)),
                Plot::Point(((0, -2), 127)),
                Plot::Point(((1, -2), 63)),
                Plot::Point(((-2, -1), 63)),
                Plot::Point(((-1, -1), 255)),
                Plot::Span(Span {
                    x0: 0,
                    x1: 0,
                    y: -1
                }),
                Plot::Point(((1, -1), 255)),
                Plot::Point(((2, -1), 63)),
                Plot::Point(((-2, 0), 127)),
                Plot::Span(Span {
                    x0: -1,
                    x1: 1,
                    y: 0
                }),
                Plot::Point(((2, 0), 127)),
                Plot::Point(((-2, 1), 63)),
                Plot::Point(((-1, 1), 255)),
                Plot::Span(Span { x0: 0, x1: 0, y: 1 }),
                Plot::Point(((1, 1), 255)),
                Plot::Point(((2, 1), 63)),
                Plot::Point(((-1, 2), 63)),
                Plot::Point(((0, 2), 127)),
                Plot::Point(((1, 2), 63)),
            ]
        );
    }

    /// The filled `r = 3` anti-aliased circle rendered onto an 8x8 grid with
    /// one hex nybble of opacity per pixel: `f` is fully on, `0` is off, and
    /// the most significant nybble is the leftmost column. Solid core of
    /// `f` spans, one linear rim, no interior dithering.
    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_shape() {
        use crate::fill::{Fill, Plot};

        let mut grid = [0u32; 8];
        let mut set = |x: isize, y: isize, c: u8| {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            let shift = ((7 - x) * 4) as u32;
            let nyb = (c as u32) >> 4;
            assert_eq!(grid[y as usize] >> shift & 0xf, 0, "({x},{y}) drawn twice");
            grid[y as usize] |= nyb << shift;
        };
        for plot in CircleAa::new((3, 3), 3).fill() {
            match plot {
                Plot::Span(h) => (h.x0..=h.x1).for_each(|x| set(x, h.y, 255)),
                Plot::Point(((x, y), c)) => set(x, y, c),
            }
        }
        #[rustfmt::skip]
        assert_eq!(grid, [
            0x00575000,
            0x0afffa00,
            0x5fffff50,
            0x7fffff70,
            0x5fffff50,
            0x0afffa00,
            0x00575000,
            0x00000000,
        ]);
    }

    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_negative_radius() {
        use crate::Fill;
        let pos: Vec<_> = CircleAa::new((1, 2), 5).fill().collect();
        let neg: Vec<_> = CircleAa::new((1, 2), -5).fill().collect();
        assert_eq!(pos, neg);
    }

    /// Every pixel matches a brute-force evaluation of Vadillo's rule over the
    /// bounding box: solid at `d² < r² − r`, else `255·(r² + r − d²) / 2r`.
    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_matches_brute_force() {
        use std::collections::BTreeMap;

        let (xm, ym) = (3isize, -2isize);
        for r in 1isize..=48 {
            let mut got = BTreeMap::new();
            for (p, a) in expand_fill((xm, ym), r) {
                assert!(got.insert(p, a).is_none(), "r={r} duplicate pixel {p:?}");
            }

            let (rmin, rmax) = (r * r - r, r * r + r);
            let mut expected = BTreeMap::new();
            for y in ym - r..=ym + r {
                for x in xm - r..=xm + r {
                    let d2 = (x - xm) * (x - xm) + (y - ym) * (y - ym);
                    if d2 < rmin {
                        expected.insert((x, y), 255u8);
                    } else if d2 < rmax {
                        let a = (255 * (rmax - d2) / (2 * r)) as u8;
                        if a > 0 {
                            expected.insert((x, y), a);
                        }
                    }
                }
            }
            assert_eq!(got, expected, "r={r}");
        }
    }

    /// Coverage tracks the true area profile `255·clamp(r + ½ − d, 0, 1)`.
    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_accuracy() {
        for r in 2isize..=64 {
            for ((x, y), a) in expand_fill((0, 0), r) {
                let d = ((x * x + y * y) as f64).sqrt();
                let expected = 255.0 * (r as f64 + 0.5 - d).clamp(0.0, 1.0);
                let tol = 2.0 + 160.0 / r as f64;
                assert!(
                    (a as f64 - expected).abs() <= tol,
                    "r={r} p=({x},{y}) alpha={a} expected={expected:.1}"
                );
            }
        }
    }

    /// The filled disk is symmetric under reflection and coordinate swap.
    #[cfg(feature = "fill")]
    #[test]
    fn test_circle_aa_fill_symmetric() {
        use std::collections::BTreeMap;

        for r in 0isize..=32 {
            let pixels: BTreeMap<_, _> = expand_fill((0, 0), r).into_iter().collect();
            for (&(x, y), &a) in &pixels {
                assert_eq!(pixels.get(&(-x, y)), Some(&a), "r={r} mirror x ({x},{y})");
                assert_eq!(pixels.get(&(x, -y)), Some(&a), "r={r} mirror y ({x},{y})");
                assert_eq!(pixels.get(&(y, x)), Some(&a), "r={r} swap ({x},{y})");
            }
        }
    }
}
