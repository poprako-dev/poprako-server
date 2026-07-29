use super::{PageImageUploadPayload, ReservedPagePayload};

use std::collections::BTreeMap;

use crate::value::image::{ImageExtension, ImageHash};

#[test]
fn reserved_page_serializes_absent_upload_as_null() {
    //
    let reserved_page_payload = ReservedPagePayload {
        page_id: "page-1".into(),
        index: 0,
        image_hash: ImageHash::new([0; 32]),
        byte_length: 1,
        extension: ImageExtension::Png,
        upload: None,
    };

    let value = serde_json::to_value(reserved_page_payload).unwrap();

    assert!(value.get("upload").unwrap().is_null());
}

#[test]
fn reserved_page_serializes_required_upload_headers() {
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
        extension: ImageExtension::Png,
        upload: Some(PageImageUploadPayload {
            put_url: "https://upload.example/page-1".into(),
            image_version: 1,
            headers,
        }),
    };

    let value = serde_json::to_value(reserved_page_payload).unwrap();

    let upload_headers = value.get("upload").unwrap().get("headers").unwrap();

    assert_eq!(upload_headers.get("content-type").unwrap(), "image/png");

    assert_eq!(
        upload_headers.get("x-amz-checksum-sha256").unwrap(),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
    );
}
