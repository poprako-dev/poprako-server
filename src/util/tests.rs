// next_snowflake_u64 (positive): generated snowflake values should be monotonic and fit into u64.
// next_snowflake_id (positive): generated snowflake strings should be base64url and round-trip.

use super::*;

/// A minimal base64url decoder that only handles unpadded 11-char strings
/// produced by `u64_to_base64url`.
fn base64url_to_u64(s: &str) -> Option<u64> {
    let decode = |c: u8| -> Option<u64> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u64,
            b'a'..=b'z' => (c - b'a') as u64 + 26,
            b'0'..=b'9' => (c - b'0') as u64 + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        })
    };

    let src = s.as_bytes();
    if src.len() != 11 {
        return None;
    }

    let d0 = decode(src[0])?;
    let d1 = decode(src[1])?;
    let d2 = decode(src[2])?;
    let d3 = decode(src[3])?;
    let d4 = decode(src[4])?;
    let d5 = decode(src[5])?;
    let d6 = decode(src[6])?;
    let d7 = decode(src[7])?;
    let d8 = decode(src[8])?;
    let d9 = decode(src[9])?;
    let d10 = decode(src[10])?;

    Some(
        (d0 << 58)
            | (d1 << 52)
            | (d2 << 46)
            | (d3 << 40)
            | (d4 << 34)
            | (d5 << 28)
            | (d6 << 22)
            | (d7 << 16)
            | (d8 << 10)
            | (d9 << 4)
            | (d10 >> 2),
    )
}

#[test]
fn next_snowflake_u64_generates_monotonic_ids() {
    let first_id = next_snowflake_u64();

    let second_id = next_snowflake_u64();

    assert!(second_id > first_id);
}

#[test]
fn next_snowflake_id_generates_base64url_string() {
    let snowflake_id = next_snowflake_id();

    let parsed = base64url_to_u64(&snowflake_id);

    assert!(parsed.is_some(), "id {snowflake_id:?} is not valid base64url");
}

#[test]
fn u64_to_base64url_round_trips() {
    let test_vals = [
        0,
        1,
        u64::MAX,
        0x1234567890ABCDEF,
        42,
        0xDEADBEEF,
    ];

    for &v in &test_vals {
        let encoded = u64_to_base64url(v);
        let decoded = base64url_to_u64(&encoded);
        assert_eq!(decoded, Some(v), "failed round-trip for {v}");
    }
}
