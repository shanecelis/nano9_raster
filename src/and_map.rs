//! Point-iterator composition: `and_map`, reflection, and translation.

use crate::Point;

/// Iterator extension: interleave each item with `f(item)`.
pub trait AndMap: Iterator + Sized {
    /// Emit each item followed by its image under `f`.
    #[inline]
    fn and_map<F>(self, mut f: F) -> impl Iterator<Item = Self::Item>
    where
        Self::Item: Copy,
        F: FnMut(Self::Item) -> Self::Item,
    {
        self.flat_map(move |x| [x, f(x)])
    }
}

impl<I: Iterator> AndMap for I {}

/// Reflect a point across the x-axis.
#[inline]
pub fn reflect_x((x, y): Point) -> Point {
    (x, -y)
}

/// Reflect a point across the y-axis.
#[inline]
pub fn reflect_y((x, y): Point) -> Point {
    (-x, y)
}

/// An iterator that translates every point by a fixed offset.
pub struct Translate<I> {
    iter: I,
    dx: isize,
    dy: isize,
}

impl<I: Iterator<Item = Point>> Iterator for Translate<I> {
    type Item = Point;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x + self.dx, y + self.dy))
    }
}

/// Composable transforms for iterators over raster points.
pub trait PointIteratorExt: Iterator<Item = Point> + Sized {
    /// Translate every point by `(dx, dy)`.
    #[inline]
    fn translate(self, dx: isize, dy: isize) -> Translate<Self> {
        Translate { iter: self, dx, dy }
    }
}

impl<I: Iterator<Item = Point>> PointIteratorExt for I {}

#[cfg(test)]
mod tests {
    use super::{reflect_x, reflect_y, AndMap};
    use std::vec::Vec;

    #[test]
    fn interleaves_item_and_mapped() {
        let got: Vec<_> = (0..3).and_map(|x| x + 10).collect();
        assert_eq!(got, [0, 10, 1, 11, 2, 12]);
    }

    #[test]
    fn reflections_are_one_to_one() {
        assert_eq!(reflect_x((2, 3)), (2, -3));
        assert_eq!(reflect_y((2, 3)), (-2, 3));
    }
}
