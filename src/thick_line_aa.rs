//! Anti-aliased outline of a thick line.
//!
//! Hard-boundary pixels use [`ThickLineFillAa`] coverage; exterior fade
//! (`c > 0` and not in the filled set) is kept as the AA fringe.

use crate::thick_line::is_fill_outline;
use crate::thick_line_fill::ThickLineFill;
use crate::{Point, PointAa};

/// Inclusive anti-aliased thick-line outline (`[start, end]`) of width `wd`.
pub struct ThickLineAa {
    start: Point,
    end: Point,
    wd: f32,
    fill: ThickLineFill,
}

impl ThickLineAa {
    /// Inclusive anti-aliased thick-line outline (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> Self {
        ThickLineAa {
            start,
            end,
            wd,
            fill: ThickLineFill::new(start, end, wd),
        }
    }
}

impl Iterator for ThickLineAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (p, c) = self.fill.next_covered()?;
            if c == 0 {
                continue;
            }
            if c < 128 || is_fill_outline(self.start, self.end, self.wd, p) {
                return Some((p, c));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThickLineAa;
    use crate::thick_line::ThickLine;
    use crate::thick_line_fill::ThickLineFill;
    use crate::thick_line_fill_aa::ThickLineFillAa;
    use crate::PointAa;
    use std::collections::{BTreeMap, BTreeSet};

    fn plot_hex(points: impl Iterator<Item = PointAa>) -> [u32; 8] {
        let mut grid = [0u32; 8];
        for ((x, y), c) in points {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            let shift = ((7 - x) * 4) as u32;
            let nyb = (c as u32) >> 4;
            let old = (grid[y as usize] >> shift) & 0xf;
            grid[y as usize] = (grid[y as usize] & !(0xf << shift)) | (old.max(nyb) << shift);
        }
        grid
    }

    fn blend(iter: impl Iterator<Item = PointAa>) -> BTreeMap<(isize, isize), u8> {
        let mut pixels: BTreeMap<(isize, isize), u8> = BTreeMap::new();
        for (p, c) in iter {
            if c == 0 {
                continue;
            }
            pixels
                .entry(p)
                .and_modify(|old| *old = (*old).max(c))
                .or_insert(c);
        }
        pixels
    }

    #[test]
    fn hard_outline_matches_thresholded_aa() {
        for (start, end, wd) in [
            ((0, 3), (7, 3), 1.0),
            ((0, 0), (5, 2), 3.0),
            ((1, 0), (2, 6), 3.0),
            ((6, 1), (1, 5), 2.5),
        ] {
            let hard: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            let from_aa: BTreeSet<_> = ThickLineAa::new(start, end, wd)
                .filter(|(_, c)| *c >= 128)
                .map(|(p, _)| p)
                .collect();
            assert_eq!(hard, from_aa, "{start:?}->{end:?} wd={wd}");
        }
    }

    #[test]
    fn exterior_fade_is_kept() {
        for (start, end, wd) in [
            ((0, 0), (5, 2), 3.0),
            ((1, 0), (2, 6), 3.0),
            ((6, 1), (1, 5), 2.5),
        ] {
            let hard: BTreeSet<_> = ThickLineFill::new(start, end, wd).collect();
            let outline = blend(ThickLineAa::new(start, end, wd));
            for (p, c) in ThickLineFillAa::new(start, end, wd) {
                if c > 0 && !hard.contains(&p) {
                    assert_eq!(
                        outline.get(&p),
                        Some(&c),
                        "missing fade {p:?} on {start:?}->{end:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_thick_line_aa_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 3), (7, 3), 1.0)), [
            0x00000000,
            0x00000000,
            0x00000000,
            0xffffffff,
            0x00000000,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
    }

    #[test]
    fn test_thick_line_aa_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((0, 0), (5, 2), 3.0)), [
            0xfffe8200,
            0xf0000f00,
            0x28ef0f00,
            0x0005bf00,
            0x00000200,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
    }

    #[test]
    fn test_thick_line_aa_shape_steep() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineAa::new((1, 0), (2, 6), 3.0)), [
            0x0ff00000,
            0x0ff30000,
            0x0ff50000,
            0x0f080000,
            0x0f0a0000,
            0x0f0d0000,
            0x0fff0000,
            0x00000000,
        ]);
    }
}
