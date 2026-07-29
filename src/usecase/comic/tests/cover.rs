// reserve_cover(reserve_cover)(positive): reservation should update cover state, enqueue check, and return put URL.
// reserve_cover(reserve_cover)(negative): missing comic should rollback cover and prom state.
// mark_cover_uploaded(mark_cover_uploaded)(positive): matching version should mark the comic cover uploaded.
// mark_cover_uploaded(mark_cover_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_cover_uploaded(mark_cover_uploaded)(negative): stale version should leave cover unuploaded.
// mark_cover_uploaded(mark_cover_uploaded)(negative): old reservation replay should fail without marking current cover uploaded.
// delete(delete)(positive): deleting a comic should remove it, decrement workset count, and enqueue cover deletion.
// delete(delete)(negative): missing comic should rollback state.

use super::*;

use crate::data::comic::{
    MarkComicCoverUploadedParams, ReserveComicCoverParams,
};
use crate::model::comic::ComicInfo;
use crate::model::workset::WorksetInfo;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::image::{ImagePayload, ResourceKind};
use crate::test_util::{
    assert_expected_message, assert_one_image_check_record,
};
use crate::value::image::{ImageExt, ImageHash};

fn reserve_params(ext: ImageExt, hash_byte: u8) -> ReserveComicCoverParams {
    ReserveComicCoverParams {
        image_hash: ImageHash::new([hash_byte; 32]),
        new_byte_len: 4096,
        ext,
    }
}

#[tokio::test]
async fn reserve_cover_updates_state_enqueues_check_and_returns_put_url() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let reserved = reserve_cover(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        reserve_params(ImageExt::Png, 1),
    )
    .await;

    assert!(reserved.is_ok());

    let reserved = reserved.ok().unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 1);

    assert_eq!(
        reserved.slot.as_ref().unwrap().put_url,
        "https://test.local/put/comic_cover/comic-1-1.png"
    );

    assert_eq!(snapshot.comics[0].cover_version, 1);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_one_image_check_record(
        &snapshot.prom_records,
        ResourceKind::ComicCover,
        "comic-1",
        "comic_cover/comic-1-1.png",
        1,
    );
}

#[tokio::test]
async fn reserve_cover_rolls_back_missing_comic() {
    //
    let mock = Mock::new();

    let err = reserve_cover(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "missing".into(),
        reserve_params(ImageExt::Png, 1),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn mark_cover_uploaded_marks_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    mark_cover_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedParams { image_version: 2 },
    )
    .await
    .ok()
    .unwrap();

    assert!(mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_accepts_repeated_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    let first = mark_cover_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedParams { image_version: 2 },
    )
    .await;

    assert!(first.is_ok());

    let second = mark_cover_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedParams { image_version: 2 },
    )
    .await;

    assert!(second.is_ok());

    assert!(mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_stale_version() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    let err = mark_cover_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedParams { image_version: 1 },
    )
    .await
    .err()
    .unwrap();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-cover-upload",
    );

    assert!(!mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_old_reservation_replay() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(ComicInfo {
        cover_key: Some("comic_cover/comic-1-1.png".into()),
        cover_uploaded: true,
        cover_version: 1,
        ..comic("comic-1", "workset-1", 0)
    });

    let reserved = reserve_cover(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        reserve_params(ImageExt::Png, 1),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 2);

    let err = mark_cover_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedParams { image_version: 1 },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-cover-upload",
    );

    assert!(!snapshot.comics[0].cover_uploaded);

    assert_eq!(snapshot.comics[0].cover_version, 2);
}

#[tokio::test]
async fn delete_removes_comic_updates_count_and_enqueues_cover_delete() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_workset(WorksetInfo {
        comic_count: 1,
        ..workset("workset-1", "team-1")
    });

    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover.png",
    ));

    delete((&mock, &mock, &mock), token("user-1"), "comic-1".into())
        .await
        .ok()
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());

    assert_eq!(snapshot.worksets[0].comic_count, 0);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert!(matches!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Image(ImagePayload::Delete { object_key }) if object_key == "cover.png"
    ));
}

#[tokio::test]
async fn delete_rolls_back_missing_comic() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let err = delete((&mock, &mock, &mock), token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.worksets.len(), 1);

    assert!(snapshot.prom_records.is_empty());
}
