// next_snowflake_u64(next_snowflake_u64)(positive): generated snowflake values should be monotonic and fit into u64.
// next_snowflake_id(next_snowflake_id)(positive): generated snowflake strings should be hexadecimal and parse back to u64.

use super::*;

#[test]
fn next_snowflake_u64_generates_monotonic_ids() {
    let first_id = next_snowflake_u64();

    let second_id = next_snowflake_u64();

    assert!(second_id > first_id);
}

#[test]
fn next_snowflake_id_generates_hex_string() {
    let snowflake_id = next_snowflake_id();

    let parsed_snowflake_id = u64::from_str_radix(&snowflake_id, 16);

    assert!(parsed_snowflake_id.is_ok());
}
