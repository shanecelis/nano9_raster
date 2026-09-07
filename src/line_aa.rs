//! Anti-aliased 2D line from Alois Zingl's `plotLineAA`.
//!
//! Coverage is inverted from Zingl's `setPixelAA`: `255` is fully on the curve,
//! `0` is fully off.

#[cfg(feature = "inclusive")]
use crate::Inclusive;
use crate::{Point, PointAa};

fn coverage_i(zingl_fade: isize) -> u8 {
    255 - if zingl_fade < 0 {
        0
    } else if zingl_fade > 255 {
        255
    } else {
        zingl_fade as u8
    }
}

/// An integer square root
///
/// Computes `floor(sqrt(x))` using a binary digit-by-digit square-root
/// algorithm.
///
/// This is the radix-2 specialization of the traditional digit-by-digit
/// ("longhand") square-root extraction method. Since binary root digits are
/// either 0 or 1, each digit can be selected using only a comparison and
/// subtraction. The radicand is processed in pairs of bits, hence the
/// descending powers of four.
///
/// M. Guy, "Fast Integer Square Root by Mr. Woo's Abacus Algorithm",
/// *University of Kent at Canterbury*, 1985.
///
/// The underlying digit-by-digit square-root method is much older and is
/// generally traced to work by François Viète around 1600.
fn isqrt(x: usize) -> usize {
    let mut n = x;
    let mut result: usize = 0;
    // Start with the largest representable power of 4.
    //
    // usize::BITS is a power of two on normal Rust targets, so BITS - 2 is
    // even and this gives 4^k rather than merely a power of two.
    let mut bit: usize = 1 << (usize::BITS - 2);
    // Find the largest power of 4 that does not exceed x. This identifies
    // the most significant pair of input bits that can contribute to sqrt(x).
    while bit > n {
        bit >>= 2;
    }
    // Determine one binary digit of the square root per iteration.
    while bit != 0 {
        // `result + bit` is the trial divisor corresponding to setting the
        // current root bit. If the remaining radicand can accommodate it,
        // accept that bit and subtract the trial value.
        if n >= result + bit {
            n -= result + bit;
            result = (result >> 1) + bit;
        } else {
            // The candidate bit does not fit, so this root bit is zero.
            result >>= 1;
        }

        // Move to the next pair of input bits / next binary root digit.
        bit >>= 2;
    }
    result
}

/// Anti-aliased 2D line
///
/// Interval: Half-open, `[start, end)`. The dropped end pixel is always
/// coverage `255`, so [`Inclusive`] restores it with no missing fade.
/// Source: Zingl `plotLineAA`
pub struct LineAa {
    x0: isize,
    y0: isize,
    x1: isize,
    y1: isize,
    dx: isize,
    dy: isize,
    sx: isize,
    sy: isize,
    err: isize,
    ed: usize,
    pending: [PointAa; 3],
    pending_len: u8,
    pending_i: u8,
    done: bool,
    inclusive: bool,
}

impl LineAa {
    /// Half-open anti-aliased line (`[start, end)`).
    pub fn new(start: Point, end: Point) -> Self {
        let (x0, y0) = start;
        let (x1, y1) = end;
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let ed = if dx + dy == 0 {
            1
        } else {
            isqrt((dx * dx + dy * dy) as usize)
        };
        let ed = if ed == 0 { 1 } else { ed };

        LineAa {
            x0,
            y0,
            x1,
            y1,
            dx,
            dy,
            sx,
            sy,
            err: dx - dy,
            ed,
            pending: [((0, 0), 0); 3],
            pending_len: 0,
            pending_i: 0,
            done: false,
            inclusive: false,
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
}

impl Iterator for LineAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(p) = self.pop_pending() {
            return Some(p);
        }
        if self.done {
            return None;
        }

        let ed = self.ed as isize;
        let fade = coverage_i(255 * (self.err - self.dx + self.dy).abs() / ed);
        self.push((self.x0, self.y0), fade);

        let e2 = self.err;
        let x2 = self.x0;
        if 2 * e2 >= -self.dx {
            if self.x0 == self.x1 {
                self.done = true;
                if !self.inclusive {
                    // Clear the queue.
                    self.pop_pending();
                    return None;
                } else {
                    return self.pop_pending();
                }
            }
            if e2 + self.dy < ed {
                self.push(
                    (self.x0, self.y0 + self.sy),
                    coverage_i(255 * (e2 + self.dy) / ed),
                );
            }
            self.err -= self.dy;
            self.x0 += self.sx;
        }
        if 2 * e2 <= self.dy {
            if self.y0 == self.y1 {
                self.done = true;
                if !self.inclusive {
                    // Clear the queue.
                    self.pop_pending();
                    return None;
                } else {
                    return self.pop_pending();
                }
            }
            if self.dx - e2 < ed {
                self.push(
                    (x2 + self.sx, self.y0),
                    coverage_i(255 * (self.dx - e2) / ed),
                );
            }
            self.err += self.dx;
            self.y0 += self.sy;
        }

        self.pop_pending()
    }
}

#[cfg(feature = "inclusive")]
#[cfg_attr(docsrs, doc(cfg(feature = "inclusive")))]
impl Inclusive for LineAa {
    type Item = PointAa;

    fn inclusive(mut self) -> impl Iterator<Item = PointAa> {
        self.inclusive = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{isqrt, LineAa};
    use std::vec::Vec;

    fn f64_floor_sqrt(x: usize) -> usize {
        (x as f64).sqrt() as usize
    }

    /// `(x as f64).sqrt() as usize` matches `isqrt` only while `x` is an integer
    /// `f64` can represent (every integer through 2^53).
    fn f64_preserves(x: usize) -> bool {
        x as f64 as usize == x
    }

    #[test]
    fn isqrt_matches_libm_small_values() {
        for x in 0usize..=10_000 {
            assert_eq!(isqrt(x), f64_floor_sqrt(x), "x = {x}");
        }
    }

    #[test]
    fn isqrt_matches_libm_around_perfect_squares() {
        for n in 0usize..=1 << 16 {
            let Some(square) = n.checked_mul(n) else {
                break;
            };
            assert_eq!(isqrt(square), n, "sqrt({n}^2)");
            assert_eq!(isqrt(square), f64_floor_sqrt(square), "x = {square}");
            if square > 0 {
                assert_eq!(isqrt(square - 1), n - 1, "sqrt({n}^2 - 1)");
                assert_eq!(
                    isqrt(square - 1),
                    f64_floor_sqrt(square - 1),
                    "x = {square} - 1"
                );
            }
            if let Some(above) = square.checked_add(1) {
                if f64_preserves(above) {
                    let expected = if n == 0 { 1 } else { n };
                    assert_eq!(isqrt(above), expected, "sqrt({n}^2 + 1)");
                    assert_eq!(isqrt(above), f64_floor_sqrt(above), "x = {above}");
                }
            }
        }
    }

    #[test]
    fn isqrt_matches_libm_line_length_inputs() {
        // LineAa passes `dx * dx + dy * dy` to `isqrt`.
        for dx in 0isize..=256 {
            for dy in 0isize..=256 {
                let x = (dx * dx + dy * dy) as usize;
                assert_eq!(isqrt(x), f64_floor_sqrt(x), "dx = {dx}, dy = {dy}");
            }
        }
    }

    #[test]
    fn isqrt_matches_libm_where_f64_is_exact() {
        for shift in [20u32, 24, 31, 32, 40, 52, 53] {
            if shift >= usize::BITS {
                continue;
            }
            for x in [(1usize << shift) - 1, 1 << shift] {
                if !f64_preserves(x) {
                    continue;
                }
                assert_eq!(isqrt(x), f64_floor_sqrt(x), "x = {x}");
                if x > 0 {
                    assert_eq!(isqrt(x - 1), f64_floor_sqrt(x - 1), "x = {x} - 1");
                }
            }
        }
    }

    #[test]
    fn test_line_aa() {
        let res: Vec<_> = LineAa::new((0, 0), (4, 0)).collect();
        assert_eq!(
            res,
            [((0, 0), 255), ((1, 0), 255), ((2, 0), 255), ((3, 0), 255)]
        );

        let res: Vec<_> = LineAa::new((0, 1), (6, 4)).collect();
        assert_eq!(
            res,
            [
                ((0, 1), 255),
                ((1, 1), 128),
                ((1, 2), 128),
                ((2, 2), 255),
                ((3, 2), 128),
                ((3, 3), 128),
                ((4, 3), 255),
                ((5, 3), 128),
                ((5, 4), 128)
            ]
        );

        let res: Vec<_> = LineAa::new((0, 0), (3, 3)).collect();
        assert_eq!(
            res,
            [
                ((0, 0), 255),
                ((0, 1), 64),
                ((1, 0), 64),
                ((1, 1), 255),
                ((1, 2), 64),
                ((2, 1), 64),
                ((2, 2), 255),
                ((2, 3), 64),
                ((3, 2), 64)
            ]
        );

        let res: Vec<_> = LineAa::new((3, 3), (3, 3)).collect();
        assert_eq!(res, []);
    }

    #[cfg(feature = "inclusive")]
    #[test]
    fn test_inclusive_line_aa() {
        use crate::Inclusive;

        let res: Vec<_> = LineAa::new((0, 0), (4, 0)).inclusive().collect();
        assert_eq!(
            res,
            [
                ((0, 0), 255),
                ((1, 0), 255),
                ((2, 0), 255),
                ((3, 0), 255),
                ((4, 0), 255)
            ]
        );

        let res: Vec<_> = LineAa::new((0, 1), (6, 4)).inclusive().collect();
        assert_eq!(
            res,
            [
                ((0, 1), 255),
                ((1, 1), 128),
                ((1, 2), 128),
                ((2, 2), 255),
                ((3, 2), 128),
                ((3, 3), 128),
                ((4, 3), 255),
                ((5, 3), 128),
                ((5, 4), 128),
                ((6, 4), 255)
            ]
        );

        let res: Vec<_> = LineAa::new((3, 3), (3, 3)).inclusive().collect();
        assert_eq!(res, [((3, 3), 255)]);
    }
}
