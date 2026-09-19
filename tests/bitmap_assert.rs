mod common;
use common::assert_bitmap_eq;

#[test]
fn equal_grids_pass() {
    assert_bitmap_eq!([0b1010u8, 0b0101], [0b1010u8, 0b0101], 4);
}

#[test]
fn mismatch_marks_missing_and_extra() {
    let err = std::panic::catch_unwind(|| {
        assert_bitmap_eq!([0b0100u8, 0b0010], [0b1000u8, 0b0010], 4);
    })
    .expect_err("grids differ");
    let msg = panic_message(&err);
    assert!(
        msg.contains("# match") && msg.contains("- missing") && msg.contains("+ extra"),
        "missing legend: {msg}"
    );
    assert!(
        msg.contains("0 | -+.."),
        "row 0 should be `-+..` (missing then extra): {msg}"
    );
    assert!(msg.contains("1 | ..#."), "row 1 should stay a match: {msg}");
}

fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        panic!("unexpected panic payload");
    }
}
