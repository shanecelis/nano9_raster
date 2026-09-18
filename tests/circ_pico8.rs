#![cfg(feature = "circle")]

//! Pico-8 ground-truth bit patterns for `circ` / `circfill`.
//!
//! Captured from `nano9/tests/golden/circ-small.p8` (`circ-small-expected.png`).
//! Inclusive disk centered at `(r, r)` so a radius-`r` circle fits in
//! `(2r+1)×(2r+1)`. Each row is packed MSB = x = 0, same as the `0b001…`
//! tests in this crate.
//!
//! `r = 4` is the case that currently mismatches Zingl's 8-way `plotCircle`.

use nano9_raster::{Circle, Point};

fn plot_bits<const H: usize>(points: impl Iterator<Item = Point>, w: u32) -> [u32; H] {
    let mut grid = [0u32; H];
    for (x, y) in points {
        assert!(
            (0..w as isize).contains(&x) && (0..H as isize).contains(&y),
            "({x},{y}) off {w}x{H}"
        );
        grid[y as usize] |= 1 << (w - 1 - x as u32);
    }
    grid
}

/// Pico-8 `circ(4, 4, 4)` on a 9×9 canvas.
#[test]
fn pico_circ_r4() {
    #[rustfmt::skip]
    assert_eq!(plot_bits::<9>(Circle::new((4, 4), 4), 9), [
        0b000111000, // ...###...
        0b011000110, // .##...##.
        0b010000010, // .#.....#.
        0b100000001, // #.......#
        0b100000001, // #.......#
        0b100000001, // #.......#
        0b010000010, // .#.....#.
        0b011000110, // .##...##.
        0b000111000, // ...###...
    ]);
}

// /// How it looks currently.
// #[test]
// fn nano9_circ_r4() {
//     #[rustfmt::skip]
//     assert_eq!(plot_bits::<9>(Circle::new((4, 4), 4), 9), [
//         0b000111000, // ...###...
//         0b001000100, // ..#...#..
//         0b010000010, // .#.....#.
//         0b100000001, // #.......#
//         0b100000001, // #.......#
//         0b100000001, // #.......#
//         0b010000010, // .#.....#.
//         0b001000100, // ..#...#..
//         0b000111000, // ...###...
//     ]);
// }

#[cfg(feature = "fill")]
mod fill {
    use super::plot_bits;
    use nano9_raster::{Circle, Fill, Span};

    fn plot_fill<const H: usize>(spans: impl Iterator<Item = Span>, w: u32) -> [u32; H] {
        plot_bits::<H>(spans.flat_map(|h| (h.x0..=h.x1).map(move |x| (x, h.y))), w)
    }

    /// Pico-8 `circfill(4, 4, 4)` on a 9×9 canvas. Rows `dy = 3` are fat
    /// (`.#######.`) compared with 8-way Zingl fill.
    #[test]
    fn pico_circfill_r4() {
        #[rustfmt::skip]
        assert_eq!(plot_fill::<9>(Circle::new((4, 4), 4).fill(), 9), [
            0b000111000, // ...###...
            0b011111110, // .#######.
            0b011111110, // .#######.
            0b111111111, // #########
            0b111111111, // #########
            0b111111111, // #########
            0b011111110, // .#######.
            0b011111110, // .#######.
            0b000111000, // ...###...
        ]);
    }
}
