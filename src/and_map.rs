//! Emit each item and its image: `x, f(x), x, f(x), …`.

/// Iterator extension: interleave each item with `f(item)`.
pub(crate) trait AndMap: Iterator + Sized {
    fn and_map<F>(self, mut f: F) -> impl Iterator<Item = Self::Item>
    where
        Self::Item: Copy,
        F: FnMut(Self::Item) -> Self::Item,
    {
        self.flat_map(move |x| [x, f(x)])
    }
}

impl<I: Iterator> AndMap for I {}

#[cfg(test)]
mod tests {
    use super::AndMap;
    use std::vec::Vec;

    #[test]
    fn interleaves_item_and_mapped() {
        let got: Vec<_> = (0..3).and_map(|x| x + 10).collect();
        assert_eq!(got, [0, 10, 1, 11, 2, 12]);
    }
}
