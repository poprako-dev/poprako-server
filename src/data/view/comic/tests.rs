use super::*;

use crate::data::instr::comic::CreateComicInstr;

#[test]
fn comic_info_view_omits_none_fields() {
    //
    let comic_info_view = ComicInfoView {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 1,
        title: "Comic".into(),
        author: "Author".into(),
        description: None,
        cover_url: None,
        cover_thumbnail_url: None,
        chapter_count: 0,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: 0,
        is_archived: false,
        archived_at: None,
        created_at: 0,
        updated_at: 0,
    };

    let serialized = serde_json::to_value(comic_info_view).unwrap();

    let serde_json::Value::Object(object) = serialized else {
        panic!("comic info value must serialize as an object");
    };

    for field_name in [
        "description",
        "cover_url",
        "cover_thumbnail_url",
        "workset",
        "team",
        "creator",
    ] {
        assert!(!object.contains_key(field_name));
    }
}

#[test]
fn create_comic_instr_deserializes_missing_optional_fields_as_none() {
    //
    let create_comic_instr =
        serde_json::from_value::<CreateComicInstr>(serde_json::json!({
            "workset_id": "workset-1",
            "title": "Comic",
            "author": "Author",
        }))
        .unwrap();

    assert!(create_comic_instr.description.is_none());

    assert!(create_comic_instr.first_chapter_subtitle.is_none());
}
