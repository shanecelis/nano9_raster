//! Fill the band between two y-monotonic walks with horizontal [`Span`]s.

use crate::fill::Span;
use crate::Point;
use core::iter::Peekable;

/// Pair a left and right point walk into inclusive left-to-right [`Span`]s.
///
/// Both iterators must be y-non-decreasing. On each row both walks visit, one
/// span covers every column from the leftmost pixel to the rightmost. A walk
/// that starts earlier or runs longer than the other contributes no spans on
/// those unmatched rows.
pub struct Spanner<L, R>
where
    L: Iterator<Item = Point>,
    R: Iterator<Item = Point>,
{
    left: Peekable<L>,
    right: Peekable<R>,
}

impl<L, R> Spanner<L, R>
where
    L: Iterator<Item = Point>,
    R: Iterator<Item = Point>,
{
    /// Fill between `left` and `right`.
    pub fn new(left: L, right: R) -> Self {
        Spanner {
            left: left.peekable(),
            right: right.peekable(),
        }
    }
}

fn skip_row<I>(it: &mut Peekable<I>, y: isize)
where
    I: Iterator<Item = Point>,
{
    while let Some(&(_, py)) = it.peek() {
        if py != y {
            break;
        }
        let _ = it.next();
    }
}

fn absorb_row<I>(it: &mut Peekable<I>, y: isize) -> (isize, isize)
where
    I: Iterator<Item = Point>,
{
    let mut lo = isize::MAX;
    let mut hi = isize::MIN;
    while let Some(&(x, py)) = it.peek() {
        if py != y {
            break;
        }
        let _ = it.next();
        lo = lo.min(x);
        hi = hi.max(x);
    }
    (lo, hi)
}

impl<L, R> Iterator for Spanner<L, R>
where
    L: Iterator<Item = Point>,
    R: Iterator<Item = Point>,
{
    type Item = Span;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let &(_, ly) = self.left.peek()?;
            let &(_, ry) = self.right.peek()?;
            if ly < ry {
                skip_row(&mut self.left, ly);
                continue;
            }
            if ry < ly {
                skip_row(&mut self.right, ry);
                continue;
            }
            let y = ly;
            let (l0, l1) = absorb_row(&mut self.left, y);
            let (r0, r1) = absorb_row(&mut self.right, y);
            return Some(Span {
                x0: l0.min(r0),
                x1: l1.max(r1),
                y,
            });
        }
    }
}

#[cfg(all(test, feature = "line"))]
mod tests {
    use super::Spanner;
    use crate::fill::Span;
    use crate::line::Line;
    use crate::Point;

    fn inclusive(a: Point, b: Point) -> impl Iterator<Item = Point> {
        Line::new(a, b).chain(core::iter::once(b))
    }

    fn plot(spans: impl Iterator<Item = Span>) -> [u8; 8] {
        let mut grid = [0u8; 8];
        for span in spans {
            for x in span.x0..=span.x1 {
                if !(0..8).contains(&x) || !(0..8).contains(&span.y) {
                    continue;
                }
                let bit = 0x80 >> x;
                let row = &mut grid[span.y as usize];
                assert!(*row & bit == 0, "overdraw at ({x}, {})", span.y);
                *row |= bit;
            }
        }
        grid
    }

    #[test]
    fn two_verticals_fill_a_rectangle() {
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive((1, 1), (1, 6)),
                inclusive((6, 1), (6, 6)),
            )),
            [
                0b00000000,
                0b01111110,
                0b01111110,
                0b01111110,
                0b01111110,
                0b01111110,
                0b01111110,
                0b00000000,
            ]
        );
    }

    #[test]
    fn vertical_and_diagonal_fill_a_wedge() {
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive((0, 0), (0, 7)),
                inclusive((0, 0), (7, 7)),
            )),
            [
                0b10000000,
                0b11000000,
                0b11100000,
                0b11110000,
                0b11111000,
                0b11111100,
                0b11111110,
                0b11111111,
            ]
        );
    }

    #[test]
    fn leftover_rows_produce_no_spans() {
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive((2, 0), (2, 7)),
                inclusive((5, 2), (5, 5)),
            )),
            [
                0b00000000,
                0b00000000,
                0b00111100,
                0b00111100,
                0b00111100,
                0b00111100,
                0b00000000,
                0b00000000,
            ]
        );
    }

    #[test]
    fn same_row_horizontals_make_one_span() {
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive((0, 3), (2, 3)),
                inclusive((5, 3), (7, 3)),
            )),
            [
                0b00000000,
                0b00000000,
                0b00000000,
                0b11111111,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ]
        );
    }

    #[test]
    fn disjoint_rows_are_empty() {
        let spans: std::vec::Vec<_> =
            Spanner::new(inclusive((0, 1), (7, 1)), inclusive((0, 5), (7, 5))).collect();
        assert!(spans.is_empty());
    }

    #[test]
    fn diamond_left_and_right() {
        let top = (3, 0);
        let left = (0, 3);
        let right = (7, 3);
        let bottom = (3, 7);
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive(top, left).chain(inclusive(left, bottom).skip(1)),
                inclusive(top, right).chain(inclusive(right, bottom).skip(1)),
            )),
            [
                0b00010000,
                0b00111100,
                0b01111110,
                0b11111111,
                0b01111110,
                0b01111100,
                0b00111000,
                0b00010000,
            ]
        );
    }

    #[test]
    fn two_diagonals_fill_the_band() {
        #[rustfmt::skip]
        assert_eq!(
            plot(Spanner::new(
                inclusive((0, 0), (3, 7)),
                inclusive((4, 0), (7, 7)),
            )),
            [
                0b11111000,
                0b11111000,
                0b01111100,
                0b01111100,
                0b00111110,
                0b00111110,
                0b00011111,
                0b00011111,
            ]
        );
    }
}
