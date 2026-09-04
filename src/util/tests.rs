// next_snowflake_u64 (positive): generated snowflake values should be monotonic and fit into u64.
// next_snowflake_id (positive): generated snowflake strings should be base62 and round-trip via base62::decode.

use super::*;

#[test]
fn trim_owned_removes_outer_unicode_whitespace_in_place() {
    //
    let value = " \u{2003}example\n".to_string();

    assert_eq!(trim_owned(value), "example");
}

#[test]
fn trim_owned_returns_empty_for_whitespace_only_values() {
    //
    assert!(trim_owned("\u{2003}\t ".into()).is_empty());
}

#[test]
fn next_snowflake_u64_generates_monotonic_ids() {
    //
    let first_id = next_snowflake_u64();

    let second_id = next_snowflake_u64();

    assert!(second_id > first_id);
}

#[test]
fn next_snowflake_id_generates_base62_string() {
    //
    let snowflake_id = next_snowflake_id();

    let parsed = base62::decode(&snowflake_id);

    assert!(parsed.is_ok(), "id {snowflake_id:?} is not valid base62");
}
