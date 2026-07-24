// Comic archive retention calendar-boundary tests.

use super::*;

#[test]
fn retained_cutoff_keeps_the_same_month_from_last_year() {
    //
    let now = OffsetDateTime::from_unix_timestamp(1_784_678_400).unwrap();

    let cutoff = retained_cutoff(now).unwrap();

    assert_eq!(cutoff.year(), 2025);

    assert_eq!(cutoff.month(), Month::July);

    assert_eq!(cutoff.day(), 1);
}

#[test]
fn next_month_crosses_the_year_boundary() {
    //
    let start = PrimitiveDateTime::new(
        Date::from_calendar_date(2025, Month::December, 1).unwrap(),
        Time::MIDNIGHT,
    )
    .assume_utc();

    let end = next_month(start).unwrap();

    assert_eq!(end.year(), 2026);

    assert_eq!(end.month(), Month::January);
}
