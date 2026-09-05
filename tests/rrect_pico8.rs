#![cfg(all(feature = "round-rect", feature = "fill"))]

//! Pico-8 ground-truth bit patterns for rounded rects on an 8×8 canvas.
//!
//! Inclusive corners: `RoundRect::new((x0, y0), (x1, y1), r)` matches Pico-8
//! `rrect(x0, y0, x1 - x0 + 1, y1 - y0 + 1, r)` / `rrectfill` via [`Fill::fill`].
//!
//! `plot_binary` packs each row as `grid[y] |= 0x80 >> x` (MSB = x = 0), same as
//! the `thick_line` unit tests.
//!
//! Captured from real Pico-8 (`nano9/tests/golden/rrect8.p8`).

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
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 0)), [
        0b11111111,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b11111111,
    ]);
}

/// Pico-8 `rrect(0,0,8,8,1)`.
#[test]
fn pico_rrect_8x8_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 1)), [
        0b01111110,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b01111110,
    ]);
}

/// Pico-8 `rrect(0,0,8,8,2)`.
#[test]
fn pico_rrect_8x8_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 2)), [
        0b00111100,
        0b01000010,
        0b10000001,
        0b10000001,
        0b10000001,
        0b10000001,
        0b01000010,
        0b00111100,
    ]);
}

/// Pico-8 `rrect(0,0,8,8,3)`. On 8×8, r≥3 matches r=4 and r=5 (maxed / clamped).
#[test]
fn pico_rrect_8x8_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 3)), [
        0b00011000,
        0b01100110,
        0b01000010,
        0b10000001,
        0b10000001,
        0b01000010,
        0b01100110,
        0b00011000,
    ]);
}

/// Pico-8 `rrect(0,0,8,8,5)` — clamp past max (`min(w,h)/2` = 4).
#[test]
fn pico_rrect_8x8_r5() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 7), 5)), [
        0b00011000,
        0b01100110,
        0b01000010,
        0b10000001,
        0b10000001,
        0b01000010,
        0b01100110,
        0b00011000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,8,0)`.
#[test]
fn pico_rrectfill_8x8_r0() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 0).fill()), [
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,8,1)`.
#[test]
fn pico_rrectfill_8x8_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 1).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b01111110,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,8,2)`.
#[test]
fn pico_rrectfill_8x8_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 2).fill()), [
        0b00111100,
        0b01111110,
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00111100,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,8,3)`. On 8×8, r≥3 matches r=4 and r=5.
#[test]
fn pico_rrectfill_8x8_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 3).fill()), [
        0b00011000,
        0b01111110,
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b01111110,
        0b00011000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,8,5)` — clamp past max.
#[test]
fn pico_rrectfill_8x8_r5() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 7), 5).fill()), [
        0b00011000,
        0b01111110,
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b01111110,
        0b00011000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,4,0)`.
#[test]
fn pico_rrectfill_8x4_r0() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 0).fill()), [
        0b11111111,
        0b11111111,
        0b11111111,
        0b11111111,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,4,1)`.
#[test]
fn pico_rrectfill_8x4_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 1).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,4,2)` (r=2 and r=3 identical — max for h=4).
#[test]
fn pico_rrectfill_8x4_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 2).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,8,4,3)` — clamp.
#[test]
fn pico_rrectfill_8x4_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (7, 3), 3).fill()), [
        0b01111110,
        0b11111111,
        0b11111111,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,4,8,0)`.
#[test]
fn pico_rrectfill_4x8_r0() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 0).fill()), [
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
    ]);
}

/// Pico-8 `rrectfill(0,0,4,8,1)`.
#[test]
fn pico_rrectfill_4x8_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 1).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ]);
}

/// Pico-8 `rrectfill(0,0,4,8,2)`.
#[test]
fn pico_rrectfill_4x8_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 2).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ]);
}

/// Pico-8 `rrectfill(0,0,4,8,3)` — clamp.
#[test]
fn pico_rrectfill_4x8_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (3, 7), 3).fill()), [
        0b01100000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b11110000,
        0b01100000,
    ]);
}

/// Pico-8 `rrectfill(0,0,6,6,0)`.
#[test]
fn pico_rrectfill_6x6_r0() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 0).fill()), [
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,6,6,1)`.
#[test]
fn pico_rrectfill_6x6_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 1).fill()), [
        0b01111000,
        0b11111100,
        0b11111100,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,6,6,2)`.
#[test]
fn pico_rrectfill_6x6_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 2).fill()), [
        0b00110000,
        0b01111000,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00110000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,6,6,3)` — clamp (max = 3).
#[test]
fn pico_rrectfill_6x6_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (5, 5), 3).fill()), [
        0b00110000,
        0b01111000,
        0b11111100,
        0b11111100,
        0b01111000,
        0b00110000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,7,7,0)`.
#[test]
fn pico_rrectfill_7x7_r0() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 0).fill()), [
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,7,7,1)`.
#[test]
fn pico_rrectfill_7x7_r1() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 1).fill()), [
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,7,7,2)`.
#[test]
fn pico_rrectfill_7x7_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 2).fill()), [
        0b00111000,
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00111000,
        0b00000000,
    ]);
}

/// Pico-8 `rrectfill(0,0,7,7,3)` — clamp (max = 3 for 7×7).
#[test]
fn pico_rrectfill_7x7_r3() {
    #[rustfmt::skip]
    assert_eq!(plot_binary_fill(RoundRect::new((0, 0), (6, 6), 3).fill()), [
        0b00111000,
        0b01111100,
        0b11111110,
        0b11111110,
        0b11111110,
        0b01111100,
        0b00111000,
        0b00000000,
    ]);
}

/// Pico-8 `rrect(0,0,8,4,2)`.
#[test]
fn pico_rrect_8x4_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (7, 3), 2)), [
        0b01111110,
        0b10000001,
        0b10000001,
        0b01111110,
        0b00000000,
        0b00000000,
        0b00000000,
        0b00000000,
    ]);
}

/// Pico-8 `rrect(0,0,4,8,2)`.
#[test]
fn pico_rrect_4x8_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (3, 7), 2)), [
        0b01100000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b10010000,
        0b01100000,
    ]);
}

/// Pico-8 `rrect(0,0,6,6,2)`.
#[test]
fn pico_rrect_6x6_r2() {
    #[rustfmt::skip]
    assert_eq!(plot_binary(RoundRect::new((0, 0), (5, 5), 2)), [
        0b00110000,
        0b01001000,
        0b10000100,
        0b10000100,
        0b01001000,
        0b00110000,
        0b00000000,
        0b00000000,
    ]);
}
