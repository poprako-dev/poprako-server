// next_snowflake_u64 (positive): generated snowflake values should be monotonic and fit into u64.
// next_snowflake_id (positive): generated snowflake strings should be base62 and round-trip via base62::decode.
// compress_archive (positive): archive payloads should survive bitcode and Zstd round trips.
// decompress_archive (negative): damaged archive bytes should return an unrecoverable error.

use super::*;

#[test]
fn next_snowflake_u64_generates_monotonic_ids() {
    let first_id = next_snowflake_u64();

    let second_id = next_snowflake_u64();

    assert!(second_id > first_id);
}

#[test]
fn next_snowflake_id_generates_base62_string() {
    let snowflake_id = next_snowflake_id();

    let parsed = base62::decode(&snowflake_id);

    assert!(parsed.is_ok(), "id {snowflake_id:?} is not valid base62");
}

#[test]
fn compress_archive_round_trips_payload() {
    let archive_payload = vec!["comic".to_string(), "chapter".to_string()];

    let archived_bytes = compress_archive(&archive_payload).unwrap();

    let decoded_payload: Vec<String> =
        decompress_archive(&archived_bytes).unwrap();

    assert_eq!(decoded_payload, archive_payload);
}

#[test]
fn decompress_archive_rejects_damaged_bytes() {
    let decode_result = decompress_archive::<Vec<String>>(&[1, 2, 3]);

    assert!(matches!(
        decode_result,
        Err(RegularError::Unrecoverable { .. })
    ));
}
