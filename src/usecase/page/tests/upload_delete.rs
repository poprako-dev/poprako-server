use super::*;

use crate::data::instr::page::{
    MarkPageImageUploadedInstr, ReservePageImageInstr,
};
use crate::value::image::{ImageExt, ImageHash};

fn seed_mark_scope(mock: &Mock) {
    //
    seed_scope(mock);

    mock.seed_page(page("page-1", 0, Some("one.png"), false, 1));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
}

fn seed_published_scope(mock: &Mock) {
    //
    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    let mut chapter_info = chapter("chapter-1", "comic-1", 1);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
}

#[tokio::test]
async fn published_chapter_rejects_page_image_writes() {
    //
    let mock = Mock::new();

    seed_published_scope(&mock);

    let manifest_result = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: Some("page-1".into()),
                image_hash: ImageHash::new([0; 32]),
                new_byte_len: Some(4096),
                ext: ImageExt::Png,
            }],
        },
    )
    .await;

    assert!(matches!(manifest_result, Err(BaseError::Expected { .. })));

    let reserve_result = reserve_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([1; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Png,
        },
    )
    .await;

    assert!(matches!(reserve_result, Err(BaseError::Expected { .. })));

    let mark_result = mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 1 },
    )
    .await;

    assert!(mark_result.is_ok());

    assert_eq!(mock.snapshot().pages[0].image_version, Some(1));

    assert!(mock.snapshot().prom_records.is_empty());
}

async fn assert_delayed_check_clears_unverified_image(
    mock: Mock,
    expected_uploaded: bool,
    expected_deleted_image_keys: Vec<&str>,
) {
    //
    seed_mark_scope(&mock);

    reserve_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([0; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Png,
        },
    )
    .await
    .unwrap();

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages[0].is_image_uploaded, Some(expected_uploaded));

    assert_eq!(
        snapshot.deleted_image_keys,
        expected_deleted_image_keys
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>(),
    );
}

#[tokio::test]
async fn delayed_check_clears_absent_object_after_mark() {
    assert_delayed_check_clears_unverified_image(
        Mock::new().with_image_head_absent(),
        false,
        Vec::new(),
    )
    .await;
}

#[tokio::test]
async fn delayed_check_marks_existing_object_after_mark() {
    assert_delayed_check_clears_unverified_image(Mock::new(), true, Vec::new())
        .await;
}

#[tokio::test]
async fn mark_image_uploaded_rejects_stale_replay_then_accepts_current_version()
{
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page(
        "page-1",
        0,
        Some("page/chapter_chapter-1/page-1-1.png"),
        true,
        1,
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([1u8; 32]),
            new_byte_len: 8192,
            ext: ImageExt::Png,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 2);

    let err = mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 1 },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-page-image-upload",
    );

    assert_ne!(snapshot.pages[0].is_image_uploaded, Some(true));

    assert_eq!(snapshot.pages[0].image_version, Some(2));

    mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 2 },
    )
    .await
    .unwrap();

    assert_eq!(mock.snapshot().pages[0].is_image_uploaded, Some(true));
}

#[tokio::test]
async fn mark_image_uploaded_rejects_non_raw_provider() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("one.png"), false, 1));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let err = mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 1 },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_ne!(snapshot.pages[0].is_image_uploaded, Some(true));
}

#[tokio::test]
async fn delete_by_chapter_deletes_pages_and_clears_counters() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", RoleMask::from(RoleField::ADMIN)));

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    mock.seed_page(page("page-2", 1, None, false, 0));

    let deleted =
        delete((&mock, &mock, &mock), token("user-1"), "chapter-1".into())
            .await;

    assert!(deleted.is_ok());

    let snapshot = mock.snapshot();

    assert!(snapshot.pages.is_empty());

    assert_eq!(snapshot.chapters[0].page_count, 0);

    assert_eq!(snapshot.chapters[0].total_unit_count, 0);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert!(matches!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Image {
            payload: ImagePayload::Delete { object_key },
        } if object_key == "one.png"
    ));
}

#[tokio::test]
async fn delete_by_chapter_rejects_non_admin_and_rolls_back() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    let err =
        delete((&mock, &mock, &mock), token("user-1"), "chapter-1".into())
            .await
            .err()
            .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.chapters[0].page_count, 0);

    assert!(snapshot.prom_records.is_empty());
}
