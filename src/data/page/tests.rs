use super::*;

use std::collections::BTreeMap;

use crate::value::image::{ImageExt, ImageHash};

#[test]
fn reserved_page_serializes_absent_slot_as_null() {
    //
    let reserved_page_payload = ReservedPagePayload {
        page_id: "page-1".into(),
        index: 0,
        image_hash: ImageHash::new([0; 32]),
        byte_length: 1,
        ext: ImageExt::Png,
        slot: None,
    };

    let value = serde_json::to_value(reserved_page_payload).unwrap();

    assert!(value.get("slot").unwrap().is_null());
}

#[test]
fn reserved_page_serializes_required_slot_headers() {
    //
    let headers = BTreeMap::from([
        ("content-type".into(), "image/png".into()),
        (
            "x-amz-checksum-sha256".into(),
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
        ),
    ]);

    let reserved_page_payload = ReservedPagePayload {
        page_id: "page-1".into(),
        index: 0,
        image_hash: ImageHash::new([0; 32]),
        byte_length: 1,
        ext: ImageExt::Png,
        slot: Some(PageSlotVal {
            put_url: "https://upload.example/page-1".into(),
            image_version: 1,
            headers,
        }),
    };

    let value = serde_json::to_value(reserved_page_payload).unwrap();

    let upload_headers = value.get("slot").unwrap().get("headers").unwrap();

    assert_eq!(upload_headers.get("content-type").unwrap(), "image/png");

    assert_eq!(
        upload_headers.get("x-amz-checksum-sha256").unwrap(),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
    );
}
