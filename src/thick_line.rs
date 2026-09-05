//! Outline of a thick line: the 4-connected boundary of [`ThickLineFill`].
//!
//! Named for Murphy's 1978 thickening paper. The filled stroke itself is
//! Zingl's `plotLineWidth`; this type traces that pixel set.

use crate::thick_line_fill::ThickLineFill;
use crate::Point;

pub(crate) fn is_fill_outline(start: Point, end: Point, wd: f32, (x, y): Point) -> bool {
    let Some((y0, y1)) = ThickLineFill::axis_span(start, end, wd, true, x) else {
        return false;
    };
    if y < y0 || y > y1 {
        return false;
    }
    if y == y0 || y == y1 {
        return true;
    }
    let left_open = match ThickLineFill::axis_span(start, end, wd, true, x - 1) {
        None => true,
        Some((a, b)) => y < a || y > b,
    };
    let right_open = match ThickLineFill::axis_span(start, end, wd, true, x + 1) {
        None => true,
        Some((a, b)) => y < a || y > b,
    };
    left_open || right_open
}

/// Inclusive thick-line outline (`[start, end]`) of width `wd`.
///
/// Yields the 4-connected boundary of [`ThickLineFill`].
pub struct ThickLine {
    start: Point,
    end: Point,
    wd: f32,
    x: isize,
    x_max: isize,
    y0: isize,
    y: isize,
    y_hi: isize,
    prev: Option<(isize, isize)>,
    next: Option<(isize, isize)>,
    done: bool,
}

impl ThickLine {
    /// Inclusive thick-line outline (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> Self {
        let Some(((x_min, _), (x_max, _))) = ThickLineFill::fill_bounds(start, end, wd) else {
            return ThickLine {
                start,
                end,
                wd,
                x: 0,
                x_max: -1,
                y0: 0,
                y: 0,
                y_hi: -1,
                prev: None,
                next: None,
                done: true,
            };
        };
        let curr = ThickLineFill::axis_span(start, end, wd, true, x_min);
        let mut line = ThickLine {
            start,
            end,
            wd,
            x: x_min,
            x_max,
            y0: 0,
            y: 0,
            y_hi: -1,
            prev: None,
            next: ThickLineFill::axis_span(start, end, wd, true, x_min + 1),
            done: false,
        };
        line.enter(curr);
        line
    }

    fn enter(&mut self, curr: Option<(isize, isize)>) {
        match curr {
            Some((y0, y1)) => {
                self.y0 = y0;
                self.y = y0;
                self.y_hi = y1;
            }
            None => {
                self.done = true;
            }
        }
    }

    fn is_outline(&self, y: isize) -> bool {
        if y == self.y0 || y == self.y_hi {
            return true;
        }
        let left_open = match self.prev {
            None => true,
            Some((a, b)) => y < a || y > b,
        };
        let right_open = match self.next {
            None => true,
            Some((a, b)) => y < a || y > b,
        };
        left_open || right_open
    }

    fn advance_column(&mut self) {
        if self.x == self.x_max {
            self.done = true;
            return;
        }
        self.prev = ThickLineFill::axis_span(self.start, self.end, self.wd, true, self.x);
        self.x += 1;
        self.next = ThickLineFill::axis_span(self.start, self.end, self.wd, true, self.x + 1);
        let curr = ThickLineFill::axis_span(self.start, self.end, self.wd, true, self.x);
        self.enter(curr);
    }
}

impl Iterator for ThickLine {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done {
            if self.y <= self.y_hi {
                let y = self.y;
                self.y += 1;
                if self.is_outline(y) {
                    return Some((self.x, y));
                }
            } else {
                self.advance_column();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{is_fill_outline, ThickLine};
    use crate::thick_line_fill::ThickLineFill;
    use crate::Point;
    use std::collections::BTreeSet;

    fn plot_binary(points: impl Iterator<Item = Point>) -> [u8; 8] {
        let mut grid = [0u8; 8];
        for (x, y) in points {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&y),
                "({x},{y}) off grid"
            );
            grid[y as usize] |= 0x80 >> x;
        }
        grid
    }

    fn boundary(start: Point, end: Point, wd: f32) -> BTreeSet<Point> {
        let fill: BTreeSet<_> = ThickLineFill::new(start, end, wd).collect();
        fill.iter()
            .copied()
            .filter(|&(x, y)| {
                let nbrs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                nbrs.iter()
                    .any(|&(dx, dy)| !fill.contains(&(x + dx, y + dy)))
            })
            .collect()
    }

    #[test]
    fn outline_is_fill_boundary() {
        for (start, end, wd) in [
            ((0, 3), (7, 3), 1.0),
            ((0, 0), (5, 2), 3.0),
            ((1, 0), (2, 6), 3.0),
            ((6, 1), (1, 5), 2.5),
            ((4, 4), (4, 4), 3.0),
            ((0, 0), (0, 5), 2.0),
        ] {
            let from_iter: BTreeSet<_> = ThickLine::new(start, end, wd).collect();
            let expected = boundary(start, end, wd);
            assert_eq!(from_iter, expected, "{start:?}->{end:?} wd={wd}");
            for &p in &from_iter {
                assert!(is_fill_outline(start, end, wd, p), "{p:?}");
            }
        }
    }

    #[test]
    fn width_one_matches_fill() {
        let fill: BTreeSet<_> = ThickLineFill::new((0, 3), (7, 3), 1.0).collect();
        let outline: BTreeSet<_> = ThickLine::new((0, 3), (7, 3), 1.0).collect();
        assert_eq!(fill, outline);
    }

    #[test]
    fn test_thick_line_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 3), (7, 3), 1.0)), [
            0b00000000,
            0b00000000,
            0b00000000,
            0b11111111,
            0b00000000,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((0, 0), (5, 2), 3.0)), [
            0b11111000,
            0b10000100,
            0b01110100,
            0b00001100,
            0b00000000,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    #[test]
    fn test_thick_line_shape_steep() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLine::new((1, 0), (2, 6), 3.0)), [
            0b01100000,
            0b01100000,
            0b01100000,
            0b01010000,
            0b01010000,
            0b01010000,
            0b01110000,
            0b00000000,
        ]);
    }
}
