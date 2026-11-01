#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

use std::collections::BTreeMap;

use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::upload::UploadHeaderView;
use crate::value::image::{ImageExt, ImageHash};

#[test]
fn reserved_page_source_serializes_absent_slot_as_null() {
    //
    let allocated_page_source_val = AllocatedPageSourceVal {
        page_source_id: "page_source--00000000001".into(),
        index: 0,
        image_hash: ImageHash::default(),
        ext: ImageExt::Png,
        image_ver: 1,
        slot: None,
    };

    let value = serde_json::to_value(allocated_page_source_val).unwrap();

    assert!(value.get("slot").unwrap().is_null());

    assert_eq!(value.get("image_version").unwrap(), 1);

    assert!(value.get("page_id").is_none());
}

#[test]
fn reserved_page_source_serializes_required_slot_headers() {
    //
    let headers = BTreeMap::from([
        ("content-type".into(), "image/png".into()),
        ("content-length".into(), "4096".into()),
    ]);

    let allocated_page_source_val = AllocatedPageSourceVal {
        page_source_id: "page_source--00000000001".into(),
        index: 0,
        image_hash: ImageHash::default(),
        ext: ImageExt::Png,
        image_ver: 1,
        slot: Some(ImageUploadSlotView {
            put_url: "https://upload.example/page_source--00000000001".into(),
            image_ver: 1,
            headers: UploadHeaderView::from_map(headers),
        }),
    };

    let value = serde_json::to_value(allocated_page_source_val).unwrap();

    let upload_headers = value.get("slot").unwrap().get("headers").unwrap();

    assert_eq!(
        upload_headers,
        &serde_json::json!([
            {"name": "content-length", "value": "4096"},
            {"name": "content-type", "value": "image/png"},
        ])
    );
}
