//! Packed one-bit grids for tests.
//!
//! Rows are integers with MSB = x = 0, matching the `0b001…` goldens. Width is
//! explicit so leading empty columns stay visible.

use std::fmt::Write;
use std::format;
use std::string::{String, ToString};
use std::vec::Vec;

/// Pack points into `H` rows of `width` bits, MSB = x = 0.
#[track_caller]
pub fn plot_bits<const H: usize>(
    points: impl IntoIterator<Item = (isize, isize)>,
    width: u32,
) -> [u32; H] {
    let mut grid = [0u32; H];
    for (x, y) in points {
        assert!(
            (0..width as isize).contains(&x) && (0..H as isize).contains(&y),
            "({x},{y}) off {width}x{H}"
        );
        grid[y as usize] |= 1 << (width - 1 - x as u32);
    }
    grid
}

/// Pack inclusive `[x0, x1]` spans on row `y`, MSB = x = 0.
#[track_caller]
pub fn plot_spans<const H: usize>(
    spans: impl IntoIterator<Item = (isize, isize, isize)>,
    width: u32,
) -> [u32; H] {
    plot_bits(
        spans.into_iter().flat_map(|(x0, x1, y)| {
            assert!(x0 <= x1, "x0={x0} > x1={x1} y={y}");
            (x0..=x1).map(move |x| (x, y))
        }),
        width,
    )
}

/// Compare packed bitmap rows and panic with an overlay on mismatch.
///
/// Overlay glyphs: `#` match, `.` empty, `-` missing, `+` extra.
macro_rules! assert_bitmap_eq {
    ($actual:expr, $expected:expr, $width:expr $(,)?) => {
        $crate::common::assert_bitmaps($actual, $expected, $width)
    };
}

pub(crate) use assert_bitmap_eq;

#[track_caller]
pub fn assert_bitmaps<A, E, T>(actual: A, expected: E, width: u32)
where
    A: AsRef<[T]>,
    E: AsRef<[T]>,
    T: Into<u64> + Copy + PartialEq,
{
    let actual = actual.as_ref();
    let expected = expected.as_ref();
    if actual == expected {
        return;
    }
    panic!(
        "bitmap mismatch (# match  . empty  - missing  + extra)\n{}",
        overlay(actual, expected, width)
    );
}

fn overlay<T>(actual: &[T], expected: &[T], width: u32) -> String
where
    T: Into<u64> + Copy,
{
    let rows = actual.len().max(expected.len());
    let label = rows.saturating_sub(1).to_string().len().max(1);
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    let mut out = String::new();
    for y in 0..rows {
        let act = actual.get(y).copied().map(Into::into).unwrap_or(0);
        let exp = expected.get(y).copied().map(Into::into).unwrap_or(0);
        let _ = write!(out, "{y:label$} | ");
        for x in 0..width {
            let shift = width - 1 - x;
            let a = (act >> shift) & 1 != 0;
            let e = (exp >> shift) & 1 != 0;
            out.push(match (e, a) {
                (true, true) => '#',
                (false, false) => '.',
                (true, false) => {
                    missing.push((x, y as u32));
                    '-'
                }
                (false, true) => {
                    extra.push((x, y as u32));
                    '+'
                }
            });
        }
        out.push('\n');
    }
    let _ = writeln!(out, "missing: {}", fmt_points(&missing));
    let _ = write!(out, "extra: {}", fmt_points(&extra));
    out
}

fn fmt_points(points: &[(u32, u32)]) -> String {
    if points.is_empty() {
        return "none".into();
    }
    points
        .iter()
        .map(|(x, y)| format!("({x}, {y})"))
        .collect::<Vec<_>>()
        .join(", ")
}
