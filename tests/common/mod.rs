//! Packed one-bit grid assertions for integration tests.
//!
//! Rows are integers with MSB = x = 0, matching the `0b001…` goldens. Width is
//! explicit so leading empty columns stay visible.

use std::fmt::Write;

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
                (true, false) => '-',
                (false, true) => '+',
            });
        }
        out.push('\n');
    }
    out
}
