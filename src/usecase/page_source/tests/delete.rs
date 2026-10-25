use super::super::delete::delete;
use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::model::read::proj::page_source::PageSourceRawIdentInfo;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::role::RoleField;

#[tokio::test]
async fn admin_delete_removes_page_sources_objects_and_clears_chapter_counts() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 2);

    {
        let mut state = mock.state.lock().unwrap();
        let chapter_info = &mut (*state.chapters.get_mut(0).unwrap());

        chapter_info.total_unit_count = 7;
        chapter_info.translated_unit_count = 5;
        chapter_info.proofread_unit_count = 3;

        drop(state);
    }

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::ADMIN),
    ));
    mock.seed_page(page_source_model("page_source--00000000001", 0));
    mock.seed_page(page_source_model("page_source--00000000002", 1));

    let page_source_key = seed_page_source_obj(
        &mock,
        "page_source--00000000001",
        1,
        true,
        (1, ImageExt::Png),
    );

    let before = OffsetDateTime::now_utc();

    mock.state.lock().unwrap().page_source_raw_idents.insert(
        "page_source--00000000001".into(),
        PageSourceRawIdentInfo {
            page_source_id: "page_source--00000000001".into(),
            raw_ident: "source.png".into(),
            created_at: before,
            updated_at: before,
        },
    );

    delete(
        (&mock, &mock, &mock),
        page_source_token("user--00000000001"),
        "chapter--00000000001".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();
    let chapter_info = &snapshot.chapters.first().unwrap();

    assert!(snapshot.pages.is_empty());
    assert!(snapshot.page_source_raw_idents.is_empty());
    assert_eq!(chapter_info.page_source_count, 0);
    assert_eq!(chapter_info.total_unit_count, 0);
    assert_eq!(chapter_info.translated_unit_count, 0);
    assert_eq!(chapter_info.proofread_unit_count, 0);
    assert!(snapshot.objs.get("page_source_image").unwrap().is_empty());
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(
        &snapshot.obj_tasks.first().unwrap().1,
        ObjTask::Delete { key } if key == &page_source_key
    ));
    assert!(snapshot.comics.first().unwrap().last_active_at >= before);
}

#[tokio::test]
async fn non_admin_delete_rejection_rolls_back_page_sources_and_objects() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 1);

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page_source_model("page_source--00000000001", 0));

    let page_source_key = seed_page_source_obj(
        &mock,
        "page_source--00000000001",
        1,
        true,
        (1, ImageExt::Png),
    );

    let error = delete(
        (&mock, &mock, &mock),
        page_source_token("user--00000000001"),
        "chapter--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Perm);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.chapters.first().unwrap().page_source_count, 1);
    assert_eq!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .key,
        page_source_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}
