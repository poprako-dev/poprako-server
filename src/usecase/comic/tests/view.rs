use super::*;

use crate::model::read::proj::page_source::PageSourceInfo;

fn seed_available_object(
    mock: &Mock,
    topic: &'static str,
    id: &str,
    domain: serde_json::Value,
    is_avail: bool,
) {
    let meta = ObjMeta {
        key: ObjKey {
            id: id.into(),
            ver: 1,
            domain,
        },
        is_avail,
        hash: vec![0; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry(topic)
        .or_default()
        .insert(
            id.into(),
            MockObjRecord {
                version: 1,
                meta: Some(meta),
            },
        );
}

fn seed_read_scope(mock: &Mock) {
    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));
    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));
    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 0));
}

fn seed_fallback_page(mock: &Mock, is_avail: bool) {
    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        StageMask::try_from(0).unwrap(),
    ));

    let created_at = OffsetDateTime::now_utc();

    mock.seed_page(PageSourceInfo {
        id: "page_source--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at,
        updated_at: created_at,
    });

    seed_available_object(
        mock,
        "page_source_image",
        "page_source--00000000001",
        serde_json::json!({
            "chapter_id": "chapter--00000000001",
            "page_source_id": "page_source--00000000001",
            "ext": "png",
        }),
        is_avail,
    );
}

#[tokio::test]
async fn get_info_returns_uploaded_cover_url() {
    let mock = Mock::new();

    seed_read_scope(&mock);
    seed_available_object(
        &mock,
        "comic_cover",
        "comic--00000000001",
        serde_json::json!({"comic_id": "comic--00000000001", "ext": "png"}),
        true,
    );

    let comic_view = get_info(
        (&mock, &mock),
        token("user--00000000001"),
        "comic--00000000001".into(),
        GetComicInfoInstr::default(),
    )
    .await
    .unwrap();

    assert_eq!(
        comic_view.cover_url.as_deref(),
        Some("https://obj.test/comic_cover/comic--00000000001/1.png"),
    );
    assert_eq!(
        comic_view.cover_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/comic_cover/comic--00000000001/1.png"),
    );
}

#[tokio::test]
async fn get_info_falls_back_to_uploaded_first_pinned_page() {
    let mock = Mock::new();

    seed_read_scope(&mock);
    seed_fallback_page(&mock, true);

    let comic_view = get_info(
        (&mock, &mock),
        token("user--00000000001"),
        "comic--00000000001".into(),
        GetComicInfoInstr::default(),
    )
    .await
    .unwrap();

    assert_eq!(
        comic_view.cover_url.as_deref(),
        Some(
            "https://obj.test/page_source/chapter--00000000001/\
                page_source--00000000001/1.png"
        ),
    );
    assert_eq!(
        comic_view.cover_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page_source/\
                chapter--00000000001/page_source--00000000001/1.png",
        ),
    );
}

#[tokio::test]
async fn list_infos_omits_fallback_without_usable_first_pinned_page() {
    let mock = Mock::new();

    seed_read_scope(&mock);
    seed_fallback_page(&mock, false);

    let list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: Vec::new(),
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.comics.len(), 1);
    assert!(list.comics.first().unwrap().cover_url.is_none());
    assert!(list.comics.first().unwrap().cover_thumbnail_url.is_none());
}
