#![cfg(all(feature = "round-rect", feature = "fill"))]

//! Pico-8 ground-truth bit patterns for rounded rects.
//!
//! Inclusive corners: `RoundRect::new((x0, y0), (x1, y1), r)` matches Pico-8
//! `rrect(x0, y0, x1 - x0 + 1, y1 - y0 + 1, r)` / `rrectfill` via [`Fill::fill`].
//!
//! `plot_binary` packs each row as `grid[y] |= 0x80 >> x` (MSB = x = 0), same as
//! the `thick_line` unit tests.
//!
//! Captured from real Pico-8 (`nano9/tests/golden/rrect8.p8`, plus the
//! `rrect-square` / `rrect-radii` / `rrect-rect` goldens for sizes the 8×8
//! canvas cannot show).

mod common;
use common::assert_bitmap_eq;
use nano9_raster::{Fill, Point, RoundRect, Span};

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

fn plot_binary_fill(spans: impl Iterator<Item = Span>) -> [u8; 8] {
    let mut grid = [0u8; 8];
    for span in spans {
        assert!(span.x0 <= span.x1);
        for x in span.x0..=span.x1 {
            assert!(
                (0..8).contains(&x) && (0..8).contains(&span.y),
                "({x},{}) off grid",
                span.y
            );
            grid[span.y as usize] |= 0x80 >> x;
        }
    }
    grid
}

/// Pico-8 `rrect(0,0,8,8,0)` → inclusive (0,0)–(7,7).
#[test]
fn pico_rrect_8x8_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 0)), [
        0b11111111,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b11111111,
    ], 8);
}

/// Pico-8 `rrect(0,0,8,8,1)`.
#[test]
fn pico_rrect_8x8_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 1)), [
        0b01111110,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b01111110,
    ], 8);
}

/// Pico-8 `rrect(0,0,8,8,2)`.
#[test]
fn pico_rrect_8x8_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 2)), [
        0b00111100,
        0b01000010,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b01000010,
        0b00111100,
    ], 8);
}

/// Pico-8 `rrect(0,0,8,8,3)`. On 8×8, r≥3 matches r=4 and r=5 (maxed / clamped).
#[test]
fn pico_rrect_8x8_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 3)), [
        0b00011000,
        0b01100110,
        0b01000010,
        0b10000001,
        0b10000001,
        0b01000010,
        0b01100110,
        0b00011000,
    ], 8);
}

/// Pico-8 `rrect(0,0,8,8,5)` — clamp past max (`min(w,h)/2` = 4).
#[test]
fn pico_rrect_8x8_r5() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 5)), [
        0b00011000,
        0b01100110,
        0b01000010,
        0b10000001,
        0b10000001,
        0b01000010,
        0b01100110,
        0b00011000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,8,0)`.
#[test]
fn pico_rrectfill_8x8_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 0).fill()), [
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,8,1)`.
#[test]
fn pico_rrectfill_8x8_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 1).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b01111110,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,8,2)`.
#[test]
fn pico_rrectfill_8x8_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 2).fill()), [
        0b00111100,
        0b01111110,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00111100,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,8,3)`. On 8×8, r≥3 matches r=4 and r=5.
#[test]
fn pico_rrectfill_8x8_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 3).fill()), [
        0b00011000,
        0b01111110,
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b01111110,
        0b00011000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,8,5)` — clamp past max.
#[test]
fn pico_rrectfill_8x8_r5() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 5).fill()), [
        0b00011000,
        0b01111110,
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b01111110,
        0b00011000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,4,0)`.
#[test]
fn pico_rrectfill_8x4_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 0).fill()), [
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,4,1)`.
#[test]
fn pico_rrectfill_8x4_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 1).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,4,2)` (r=2 and r=3 identical — max for h=4).
#[test]
fn pico_rrectfill_8x4_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 2).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,8,4,3)` — clamp.
#[test]
fn pico_rrectfill_8x4_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 3).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,4,8,0)`.
#[test]
fn pico_rrectfill_4x8_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 0).fill()), [
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,4,8,1)`.
#[test]
fn pico_rrectfill_4x8_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 1).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,4,8,2)`.
#[test]
fn pico_rrectfill_4x8_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 2).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,4,8,3)` — clamp.
#[test]
fn pico_rrectfill_4x8_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 3).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,6,6,0)`.
#[test]
fn pico_rrectfill_6x6_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 0).fill()), [
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,6,6,1)`.
#[test]
fn pico_rrectfill_6x6_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 1).fill()), [
        0b01111000,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,6,6,2)`.
#[test]
fn pico_rrectfill_6x6_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 2).fill()), [
        0b00110000,
        0b01111000,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00110000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,6,6,3)` — clamp (max = 3).
#[test]
fn pico_rrectfill_6x6_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 3).fill()), [
        0b00110000,
        0b01111000,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00110000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,7,7,0)`.
#[test]
fn pico_rrectfill_7x7_r0() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 0).fill()), [
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,7,7,1)`.
#[test]
fn pico_rrectfill_7x7_r1() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 1).fill()), [
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,7,7,2)`.
#[test]
fn pico_rrectfill_7x7_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 2).fill()), [
        0b00111000,
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00111000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrectfill(0,0,7,7,3)` — clamp (max = 3 for 7×7).
#[test]
fn pico_rrectfill_7x7_r3() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 3).fill()), [
        0b00111000,
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00111000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrect(0,0,8,4,2)`.
#[test]
fn pico_rrect_8x4_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (7, 3), 2)), [
        0b01111110,
        0b10000001,
        0b10000001,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ], 8);
}

/// Pico-8 `rrect(0,0,4,8,2)`.
#[test]
fn pico_rrect_4x8_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (3, 7), 2)), [
        0b01100000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b01100000,
    ], 8);
}

/// Pico-8 `rrect(0,0,6,6,2)`.
#[test]
fn pico_rrect_6x6_r2() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_binary(RoundRect::new((0, 0), (5, 5), 2)), [
        0b00110000,
        0b01001000,
        0b10000100,
        0b10000100,
        0b01001000,
        0b00110000,
        0b00000000,
        0b00000000,
    ], 8);
}

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

fn plot_fill_bits<const H: usize>(spans: impl Iterator<Item = Span>, w: u32) -> [u32; H] {
    plot_bits::<H>(spans.flat_map(|h| (h.x0..=h.x1).map(move |x| (x, h.y))), w)
}

/// Pico-8 `rrectfill(0,0,11,11,5)` from `rrect-square.p8`. Maxed odd square
/// (`2r+1`); current clamp `(min−2)/2` is 4, not 5.
#[test]
fn pico_rrectfill_11x11_r5() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_fill_bits::<11>(RoundRect::new((0, 0), (10, 10), 5).fill(), 11), [
        0b00011111000, // ...#####...
        0b00111111100, // ..#######..
        0b01111111110, // .#########.
        0b11111111111, // ###########
        0b11111111111, // ###########
        0b11111111111, // ###########
        0b11111111111, // ###########
        0b11111111111, // ###########
        0b01111111110, // .#########.
        0b00111111100, // ..#######..
        0b00011111000, // ...#####...
    ], 11);
}

/// Pico-8 `rrect(0,0,11,11,5)` from `rrect-square.p8`.
#[test]
fn pico_rrect_11x11_r5() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_bits::<11>(RoundRect::new((0, 0), (10, 10), 5), 11), [
        0b00011111000, // ...#####...
        0b00100000100, // ..#.....#..
        0b01000000010, // .#.......#.
        0b10000000001, // #.........#
        0b10000000001, // #.........#
        0b10000000001, // #.........#
        0b10000000001, // #.........#
        0b10000000001, // #.........#
        0b01000000010, // .#.......#.
        0b00100000100, // ..#.....#..
        0b00011111000, // ...#####...
    ], 11);
}

/// Pico-8 `rrectfill(0,0,19,19,4)` from `rrect-radii.p8`. Caps stay apart
/// (`2r < min−1`); corners are a 45° chamfer, thinner than `circfill`.
#[test]
fn pico_rrectfill_19x19_r4() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_fill_bits::<19>(RoundRect::new((0, 0), (18, 18), 4).fill(), 19), [
        0b0001111111111111000, // ...#############...
        0b0011111111111111100, // ..###############..
        0b0111111111111111110, // .#################.
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b0111111111111111110, // .#################.
        0b0011111111111111100, // ..###############..
        0b0001111111111111000, // ...#############...
    ], 19);
}

/// Pico-8 `rrect(0,0,19,19,4)` from `rrect-radii.p8`.
#[test]
fn pico_rrect_19x19_r4() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_bits::<19>(RoundRect::new((0, 0), (18, 18), 4), 19), [
        0b0001111111111111000, // ...#############...
        0b0010000000000000100, // ..#.............#..
        0b0100000000000000010, // .#...............#.
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b1000000000000000001, // #.................#
        0b0100000000000000010, // .#...............#.
        0b0010000000000000100, // ..#.............#..
        0b0001111111111111000, // ...#############...
    ], 19);
}

/// Pico-8 `rrectfill(0,0,19,19,5)` from `rrect-radii.p8`. Caps stay apart;
/// same 45° chamfer family as r=4 (`cut = r−1−row`).
#[test]
fn pico_rrectfill_19x19_r5() {
    #[rustfmt::skip]
    assert_bitmap_eq!(plot_fill_bits::<19>(RoundRect::new((0, 0), (18, 18), 5).fill(), 19), [
        0b0000111111111110000, // ....###########....
        0b0001111111111111000, // ...#############...
        0b0011111111111111100, // ..###############..
        0b0111111111111111110, // .#################.
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b1111111111111111111, // ###################
        0b0111111111111111110, // .#################.
        0b0011111111111111100, // ..###############..
        0b0001111111111111000, // ...#############...
        0b0000111111111110000, // ....###########....
    ], 19);
}

/// Top-left 8×8 of Pico-8 `rrectfill(0,0,40,20,4)` from `rrect-rect.p8`.
/// Caps stay apart; chamfer, not a quarter-disk.
#[test]
fn pico_rrectfill_40x20_r4_corner() {
    let spans = RoundRect::new((0, 0), (39, 19), 4).fill();
    let mut grid = [0u8; 8];
    for span in spans {
        if !(0..8).contains(&span.y) {
            continue;
        }
        for x in span.x0..=span.x1 {
            if (0..8).contains(&x) {
                grid[span.y as usize] |= 0x80 >> x;
            }
        }
    }
    #[rustfmt::skip]
    assert_bitmap_eq!(grid, [
        0b00011111, // ...#####
        0b00111111, // ..######
        0b01111111, // .#######
        0b11111111, // ########
        0b11111111, // ########
        0b11111111, // ########
        0b11111111, // ########
        0b11111111, // ########
    ], 8);
}
