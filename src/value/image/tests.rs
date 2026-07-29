use super::*;

#[test]
fn image_hash_round_trips_canonical_base64() {
    //
    let encoded = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    let image_hash =
        serde_json::from_str::<ImageHash>(&format!("\"{}\"", encoded)).unwrap();

    assert_eq!(image_hash.to_base64(), encoded);

    assert_eq!(image_hash.bytes(), [0; 32]);
}

#[test]
fn image_hash_rejects_noncanonical_encodings() {
    //
    let invalid_hashes = [
        "00",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==",
        "__________________________________________8=",
        "00000000000000000000000000000000000000000000",
    ];

    for invalid_hash in invalid_hashes {
        //
        let result =
            serde_json::from_str::<ImageHash>(&format!("\"{}\"", invalid_hash));

        assert!(result.is_err());
    }
}

#[test]
fn image_extensions_provide_fixed_content_types() {
    //
    let mappings = [
        (ImageExt::Jpg, "jpg", "image/jpeg"),
        (ImageExt::Jpeg, "jpeg", "image/jpeg"),
        (ImageExt::Png, "png", "image/png"),
        (ImageExt::Gif, "gif", "image/gif"),
        (ImageExt::Webp, "webp", "image/webp"),
        (ImageExt::Svg, "svg", "image/svg+xml"),
        (ImageExt::Avif, "avif", "image/avif"),
        (ImageExt::Bmp, "bmp", "image/bmp"),
        (ImageExt::Tif, "tif", "image/tiff"),
        (ImageExt::Tiff, "tiff", "image/tiff"),
    ];

    for (image_extension, suffix, content_type) in mappings {
        //
        assert_eq!(image_extension.suffix(), suffix);

        assert_eq!(image_extension.content_type(), content_type);
    }

    assert!(ImageExt::parse("exe").is_none());
}
