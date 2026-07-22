// Calendar-month parsing and validation tests for comic archive export.

use super::*;

#[test]
fn parse_retained_accepts_distinct_selected_slots() {
    //
    let now = OffsetDateTime::from_unix_timestamp(1_784_678_400).unwrap();

    let months = ComicArchiveMonth::parse_retained(
        vec!["2026-06".into(), "2025-07".into()],
        now,
    )
    .unwrap();

    assert_eq!(months[0].label, "2025-07");

    assert_eq!(months[1].label, "2026-06");
}

#[test]
fn parse_retained_rejects_expired_slots() {
    //
    let now = OffsetDateTime::from_unix_timestamp(1_784_678_400).unwrap();

    let result =
        ComicArchiveMonth::parse_retained(vec!["2025-06".into()], now);

    assert!(matches!(result, Err(BaseError::Expected { .. })));
}
