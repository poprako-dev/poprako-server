use super::*;

use crate::data::page::{MarkPageImageUploadedParams, ReservePageImageParams};

#[tokio::test]
async fn mark_image_uploaded_rejects_stale_replay_then_accepts_current_version()
{
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page(
        "page-1",
        0,
        Some("chapter_chapter-1/page_page-1-1.png"),
        true,
        1,
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        ReservePageImageParams {
            file_ext: "png".into(),
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.image_version, 2);

    let err = mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedParams { image_version: 1 },
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

    assert!(!snapshot.pages[0].image_uploaded);

    assert_eq!(snapshot.pages[0].image_version, 2);

    mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedParams { image_version: 2 },
    )
    .await
    .unwrap();

    assert!(mock.snapshot().pages[0].image_uploaded);
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
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedParams { image_version: 1 },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(!snapshot.pages[0].image_uploaded);
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
        delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into()).await;

    assert!(deleted.is_ok());

    let snapshot = mock.snapshot();

    assert!(snapshot.pages.is_empty());

    assert_eq!(snapshot.chapters[0].page_count, 0);

    assert_eq!(snapshot.chapters[0].total_unit_count, 0);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert!(matches!(
        snapshot.prom_records[0].payload(),
        Payload::Image(ImagePayload::Delete { object_key }) if object_key == "one.png"
    ));
}

#[tokio::test]
async fn delete_by_chapter_rejects_non_admin_and_rolls_back() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    let err = delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into())
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.chapters[0].page_count, 0);

    assert!(snapshot.prom_records.is_empty());
}
