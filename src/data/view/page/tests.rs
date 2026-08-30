use super::PageInfoView;

#[test]
fn page_info_view_omits_absent_image_urls() {
    let page_info_view = PageInfoView {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        image_url: None,
        image_thumbnail_url: None,
        image_hash: None,
        ext: None,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: 0,
        updated_at: 0,
    };

    let serialized = serde_json::to_value(page_info_view).unwrap();

    let serde_json::Value::Object(object) = serialized else {
        panic!("page info value must serialize as an object");
    };

    assert!(!object.contains_key("image_url"));
    assert!(!object.contains_key("image_thumbnail_url"));
}
