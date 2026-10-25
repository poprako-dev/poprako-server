#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::PageSourceInfoView;

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn page_source_info_view_omits_absent_image_urls() {
    let page_source_info_view = PageSourceInfoView {
        id: "page_source--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        index: 0,
        raw_ident: None,
        image_ver: None,
        image_uploaded: false,
        image_url: None,
        image_optimized_url: None,
        image_thumbnail_url: None,
        image_hash: None,
        ext: None,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: 0,
        updated_at: 0,
    };

    let serialized = serde_json::to_value(page_source_info_view).unwrap();

    let serde_json::Value::Object(object) = serialized else {
        panic!("page info value must serialize as an object");
    };

    assert!(!object.contains_key("image_url"));
    assert!(!object.contains_key("image_optimized_url"));
    assert!(!object.contains_key("image_thumbnail_url"));
}
