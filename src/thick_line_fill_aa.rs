//! Anti-aliased filled thick line from Alois Zingl's `plotLineWidth`.
//!
//! Coverage is inverted from Zingl's `setPixelAA`: `255` is fully on the curve,
//! `0` is fully off.

use crate::thick_line_fill::ThickLineFill;
use crate::{Point, PointAa};

/// Anti-aliased filled line of a given pixel width
///
/// Inclusive: `[start, end]`.
pub struct ThickLineFillAa {
    line: ThickLineFill,
}

impl ThickLineFillAa {
    /// Inclusive anti-aliased filled thick line (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> Self {
        ThickLineFillAa {
            line: ThickLineFill::new(start, end, wd),
        }
    }
}

impl Iterator for ThickLineFillAa {
    type Item = PointAa;

    fn next(&mut self) -> Option<Self::Item> {
        self.line.next_covered()
    }
}

#[cfg(test)]
mod tests {
    use super::ThickLineFillAa;
    use crate::thick_line_fill::ThickLineFill;
    use crate::PointAa;
    use std::vec::Vec;

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

    #[test]
    fn test_thick_line_fill_aa() {
        let res: Vec<_> = ThickLineFillAa::new((0, 0), (4, 0), 1.0).collect();
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

        let res: Vec<_> = ThickLineFillAa::new((0, 0), (5, 2), 3.0).collect();
        assert_eq!(
            res,
            [
                ((0, 0), 255),
                ((0, 1), 255),
                ((0, 2), 37),
                ((1, 0), 255),
                ((1, 1), 255),
                ((1, 2), 132),
                ((2, 0), 255),
                ((3, 0), 226),
                ((4, 0), 132),
                ((5, 0), 37),
                ((2, 1), 255),
                ((2, 2), 226),
                ((3, 1), 255),
                ((3, 2), 255),
                ((3, 3), 84),
                ((4, 1), 255),
                ((4, 2), 255),
                ((4, 3), 179),
                ((5, 1), 255),
                ((5, 2), 255),
                ((5, 3), 255),
                ((5, 4), 37)
            ]
        );
    }

    #[test]
    fn test_thick_line_fill_matches_aa_threshold() {
        for (start, end, wd) in [
            ((0, 0), (4, 0), 1.0),
            ((0, 0), (5, 2), 3.0),
            ((1, 0), (2, 6), 3.0),
            ((6, 1), (1, 5), 2.5),
        ] {
            let hard: Vec<_> = ThickLineFill::new(start, end, wd).collect();
            let from_aa: Vec<_> = ThickLineFillAa::new(start, end, wd)
                .filter(|(_, c)| *c >= 128)
                .map(|(p, _)| p)
                .collect();
            assert_eq!(hard, from_aa, "{start:?} -> {end:?} wd={wd}");
        }
    }

    /// Horizontal hairline. Compare the binary plot in `thick_line_fill`.
    #[test]
    fn test_thick_line_fill_aa_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineFillAa::new((0, 3), (7, 3), 1.0)), [
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

    /// Shallow width-3 stroke. Binary counterpart is thresholded at nybble 8.
    #[test]
    fn test_thick_line_fill_aa_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineFillAa::new((0, 0), (5, 2), 3.0)), [
            0xfffe8200,
            0xffffff00,
            0x28efff00,
            0x0005bf00,
            0x00000200,
            0x00000000,
            0x00000000,
            0x00000000,
        ]);
    }

    /// Steep width-3 stroke. Binary counterpart is thresholded at nybble 8.
    #[test]
    fn test_thick_line_fill_aa_shape_steep() {
        #[rustfmt::skip]
        assert_eq!(plot_hex(ThickLineFillAa::new((1, 0), (2, 6), 3.0)), [
            0x0ff00000,
            0x0ff30000,
            0x0ff50000,
            0x0ff80000,
            0x0ffa0000,
            0x0ffd0000,
            0x0fff0000,
            0x00000000,
        ]);
    }
}
