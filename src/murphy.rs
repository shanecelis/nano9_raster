//! Murphy's 1978 perpendicular thickening.
//!
//! A. S. Murphy, "Line Thickening by Modification to Bresenham's Algorithm",
//! *IBM Technical Disclosure Bulletin*, 20(12):5358–5366, 1978.
//!
//! The outer walk steps along the chord; at each point an inner [`Line`]
//! draws a spoke perpendicular to the chord. Difference terms are shared so
//! the two loops stay in phase. When both would take a diagonal step, the
//! outer walk does a double square move and the extra intermediate spoke is
//! drawn — that is the hole-filling step on steep and 45° strokes.
//!
//! No parallel-line variant, no anti-aliasing.

use crate::line::Line;
use crate::Point;

fn offset(p: Point, dir: Point, steps: isize) -> Point {
    if steps <= 0 || (dir.0 == 0 && dir.1 == 0) {
        return p;
    }
    let target = (p.0 + dir.0, p.1 + dir.1);
    Line::new(p, target).nth(steps as usize).unwrap_or(target)
}

/// Filled thick line from Murphy's perpendicular thickening.
///
/// Inclusive: `[start, end]`. Width `wd` is rounded; a width of `1` is the
/// spine alone.
pub struct ThickLineFill {
    x: isize,
    y: isize,
    sx: isize,
    sy: isize,
    dx: isize,
    dy: isize,
    steep: bool,
    ku: isize,
    kv: isize,
    kd: isize,
    kt: isize,
    d0: isize,
    d1: isize,
    half: isize,
    steps_left: isize,
    spoke: Line,
    spoke_end: Point,
    pending_spoke_end: bool,
    pending_second_square: bool,
    done: bool,
}

impl ThickLineFill {
    /// Inclusive thick line (`[start, end]`) with width `wd`.
    pub fn new(start: Point, end: Point, wd: f32) -> Self {
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let sx = dx.signum();
        let sy = dy.signum();
        let adx = dx.abs();
        let ady = dy.abs();
        let steep = ady > adx;
        let (u, v) = if steep { (ady, adx) } else { (adx, ady) };
        let half = (wd.abs() / 2.0) as isize;
        let mut line = ThickLineFill {
            x: start.0,
            y: start.1,
            sx,
            sy,
            dx,
            dy,
            steep,
            ku: 2 * u,
            kv: 2 * v,
            kd: 2 * v - 2 * u,
            kt: u - 2 * v,
            d0: 0,
            d1: 0,
            half: half.max(0),
            steps_left: u,
            spoke: Line::new(start, start),
            spoke_end: start,
            pending_spoke_end: false,
            pending_second_square: false,
            done: false,
        };
        line.start_spoke((start.0, start.1));
        line
    }

    fn start_spoke(&mut self, p: Point) {
        let left = offset(p, (-self.dy, self.dx), self.half);
        let right = offset(p, (self.dy, -self.dx), self.half);
        self.spoke = Line::new(left, right);
        self.spoke_end = right;
        self.pending_spoke_end = true;
    }

    fn square_along(&mut self) {
        if self.steep {
            self.y += self.sy;
        } else {
            self.x += self.sx;
        }
    }

    fn diagonal_along(&mut self) {
        self.x += self.sx;
        self.y += self.sy;
    }

    fn first_square(&mut self) {
        self.square_along();
    }

    fn second_square(&mut self) {
        if self.steep {
            self.x += self.sx;
        } else {
            self.y += self.sy;
        }
    }

    fn advance_spine(&mut self) {
        if self.pending_second_square {
            self.pending_second_square = false;
            self.second_square();
            self.start_spoke((self.x, self.y));
            return;
        }
        if self.steps_left == 0 {
            self.done = true;
            return;
        }
        self.steps_left -= 1;
        if self.d0 < self.kt {
            self.square_along();
            self.d0 += self.kv;
            self.start_spoke((self.x, self.y));
            return;
        }
        self.d0 = self.d0 - self.ku + self.kv;
        // Inner loop of length 1 never steps, so it cannot diagonal.
        if self.half > 0 && self.d1 >= self.kt {
            self.first_square();
            self.d1 -= self.kd;
            self.pending_second_square = true;
            self.start_spoke((self.x, self.y));
        } else {
            self.diagonal_along();
            self.d1 -= self.kv;
            self.start_spoke((self.x, self.y));
        }
    }
}

impl Iterator for ThickLineFill {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            if let Some(p) = self.spoke.next() {
                return Some(p);
            }
            if self.pending_spoke_end {
                self.pending_spoke_end = false;
                return Some(self.spoke_end);
            }
            self.advance_spine();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThickLineFill;
    use crate::line::Line;
    use crate::Point;
    use std::collections::BTreeSet;
    use std::vec::Vec;

    fn plot_binary(points: impl Iterator<Item = Point>) -> [u8; 8] {
        let mut grid = [0u8; 8];
        for (x, y) in points {
            if !(0..8).contains(&x) || !(0..8).contains(&y) {
                continue;
            }
            grid[y as usize] |= 0x80 >> x;
        }
        grid
    }

    #[test]
    fn width_one_is_inclusive_line() {
        let start = (0, 3);
        let end = (7, 3);
        let fill: BTreeSet<_> = ThickLineFill::new(start, end, 1.0).collect();
        let mut line: BTreeSet<_> = Line::new(start, end).collect();
        line.insert(end);
        assert_eq!(fill, line);
    }

    #[test]
    fn width_one_diagonal_is_inclusive_line() {
        let start = (0, 0);
        let end = (7, 7);
        let fill: BTreeSet<_> = ThickLineFill::new(start, end, 1.0).collect();
        let mut line: BTreeSet<_> = Line::new(start, end).collect();
        line.insert(end);
        assert_eq!(fill, line);
    }

    #[test]
    fn degenerate_is_the_point() {
        let fill: Vec<_> = ThickLineFill::new((3, 4), (3, 4), 3.0).collect();
        assert_eq!(fill, [(3, 4)]);
    }

    /// Horizontal width 3: a vertical spoke of 3 at each spine pixel.
    #[test]
    fn test_thick_line_fill_shape_horizontal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 3), (7, 3), 3.0)), [
            0b00000000,
            0b00000000,
            0b11111111,
            0b11111111,
            0b11111111,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 3), (7, 3), 1.0)), [
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
    fn test_thick_line_fill_shape_shallow() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 0), (5, 2), 3.0)), [
            0b11110000,
            0b11111100,
            0b00111100,
            0b00001100,
            0b00000000,
            0b00000000,
            0b00000000,
            0b00000000,
        ]);
    }

    /// Vertical width 3: a horizontal spoke of 3 at each spine pixel.
    #[test]
    fn test_thick_line_fill_shape_vertical() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((3, 0), (3, 7), 3.0)), [
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
            0b00111000,
        ]);
    }

    /// Empty pixels sandwiched on a 4-axis between filled neighbors.
    fn sandwiched_holes(fill: &BTreeSet<Point>) -> BTreeSet<Point> {
        let mut pts = fill.iter().copied();
        let Some((x0, y0)) = pts.next() else {
            return BTreeSet::new();
        };
        let (mut minx, mut maxx, mut miny, mut maxy) = (x0, x0, y0, y0);
        for (x, y) in pts {
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
        }
        let mut holes = BTreeSet::new();
        for y in miny..=maxy {
            for x in minx..=maxx {
                let p = (x, y);
                if fill.contains(&p) {
                    continue;
                }
                let lr = fill.contains(&(x - 1, y)) && fill.contains(&(x + 1, y));
                let ud = fill.contains(&(x, y - 1)) && fill.contains(&(x, y + 1));
                if lr || ud {
                    holes.insert(p);
                }
            }
        }
        holes
    }

    fn in_grid_8(holes: BTreeSet<Point>) -> BTreeSet<Point> {
        holes
            .into_iter()
            .filter(|&(x, y)| (0..8).contains(&x) && (0..8).contains(&y))
            .collect()
    }

    /// 45° width 3: double-square extras fill the 4-connected holes.
    #[test]
    fn test_thick_line_fill_shape_diagonal() {
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 0), (7, 7), 3.0)), [
            0b11110000,
            0b11111000,
            0b11111100,
            0b01111110,
            0b00111111,
            0b00011111,
            0b00001111,
            0b00000111,
        ]);
        #[rustfmt::skip]
        assert_eq!(plot_binary(ThickLineFill::new((0, 7), (7, 0), 3.0)), [
            0b00000111,
            0b00001111,
            0b00011111,
            0b00111111,
            0b01111110,
            0b11111100,
            0b11111000,
            0b11110000,
        ]);
    }

    #[test]
    fn diagonal_width_three_has_no_regular_holes() {
        for (start, end) in [((0, 0), (7, 7)), ((0, 7), (7, 0)), ((7, 7), (0, 0))] {
            let fill: BTreeSet<_> = ThickLineFill::new(start, end, 3.0).collect();
            assert_eq!(
                in_grid_8(sandwiched_holes(&fill)),
                BTreeSet::new(),
                "{start:?}->{end:?}"
            );
        }
    }
}
