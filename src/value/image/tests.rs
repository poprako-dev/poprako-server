use super::*;

#[test]
fn image_hash_round_trips_canonical_base64() {
    let encoded = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let image_hash =
        serde_json::from_str::<ImageHash>(&format!("\"{}\"", encoded))
            .unwrap();

    assert_eq!(image_hash.to_base64(), encoded);
    assert_eq!(image_hash.bytes(), [0; 32]);
}

#[test]
fn image_hash_rejects_noncanonical_encodings() {
    let invalid_hashes = [
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "__________________________________________8=",
        "00000000000000000000000000000000000000000000",
    ];

    for invalid_hash in invalid_hashes {
        let result = serde_json::from_str::<ImageHash>(&format!(
            "\"{}\"",
            invalid_hash
        ));

        assert!(result.is_err());
    }
}

#[test]
fn image_extensions_provide_fixed_content_types() {
    assert_eq!(ImageExt::Png.content_type(), "image/png");
    assert_eq!(ImageExt::Svg.suffix(), "svg");
    assert!(ImageExt::parse("exe").is_none());
}
