use super::*;

use std::collections::BTreeMap;

use crate::data::view::image::ImageUploadSlotView;
use crate::value::image::{ImageExt, ImageHash};

#[test]
fn reserved_page_serializes_absent_slot_as_null() {
    //
    let reserved_page_val = ReservedPageVal {
        page_id: "page-1".into(),
        index: 0,
        image_hash: ImageHash::new([0; 32]),
        ext: ImageExt::Png,
        slot: None,
    };

    let value = serde_json::to_value(reserved_page_val).unwrap();

    assert!(value.get("slot").unwrap().is_null());
}

#[test]
fn reserved_page_serializes_required_slot_headers() {
    //
    let headers = BTreeMap::from([
        ("content-type".into(), "image/png".into()),
        ("content-length".into(), "4096".into()),
    ]);

    let reserved_page_val = ReservedPageVal {
        page_id: "page-1".into(),
        index: 0,
        image_hash: ImageHash::new([0; 32]),
        ext: ImageExt::Png,
        slot: Some(ImageUploadSlotView {
            put_url: "https://upload.example/page-1".into(),
            image_version: 1,
            headers,
        }),
    };

    let value = serde_json::to_value(reserved_page_val).unwrap();

    let upload_headers = value.get("slot").unwrap().get("headers").unwrap();

    assert_eq!(upload_headers.get("content-type").unwrap(), "image/png");

    assert_eq!(upload_headers.get("content-length").unwrap(), "4096");

    assert!(upload_headers.get("x-amz-checksum-sha256").is_none());
}
