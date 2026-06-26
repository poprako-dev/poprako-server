// create(create)(positive): creating a comic should allocate workset-scoped index and update comic count.
// create(create)(negative): missing workset should rollback without creating a comic.
// get_info(get_info)(positive): existing comic should return uploaded cover URL.
// get_info(get_info)(negative): missing comic should propagate an argument error.
// list_infos(list_infos)(positive): list should return workset comics sorted by index.
// list_infos(list_infos)(positive): empty workset contents should return an empty list after membership.
// update_info(update_info)(positive): existing comic should update title, author, and description.
// update_info(update_info)(negative): missing comic should propagate an argument error.
// reserve_cover(reserve_cover)(positive): reservation should update cover state, enqueue check, and return put URL.
// reserve_cover(reserve_cover)(negative): missing comic should rollback cover and prom state.
// mark_cover_uploaded(mark_cover_uploaded)(positive): matching version should mark the comic cover uploaded.
// mark_cover_uploaded(mark_cover_uploaded)(negative): stale version should leave cover unuploaded.
// delete(delete)(positive): deleting a comic should remove it, decrement workset count, and enqueue cover deletion.
// delete(delete)(negative): missing comic should rollback state.
// mark_completed(mark_completed)(positive): marking completed should update completion state.
// mark_completed(mark_completed)(negative): missing comic should rollback state.

use super::*;

use time::OffsetDateTime;

use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::role::{RoleBit, RoleMask};
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part::prom::Payload;
use crate::part::prom::intention::{ImageIntention, ImageKind};
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::usecase::team::tests::workset;

fn comic(id: &str, workset_id: &str, index: i32) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index,
        title: format!("comic-{}", index),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 0,
        chapter_next_index: 0,
        creator_id: "user-1".into(),
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn comic_with_uploaded_cover(id: &str, workset_id: &str, cover_key: &str) -> ComicInfo {
    ComicInfo {
        cover_key: Some(cover_key.into()),
        cover_uploaded: true,
        cover_version: 1,
        ..comic(id, workset_id, 0)
    }
}

fn create_data(workset_id: &str) -> CreateComicData {
    CreateComicData {
        workset_id: workset_id.into(),
        title: "new".into(),
        author: "author".into(),
        description: Some("desc".into()),
    }
}

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn admin_member(user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        team_id: team_id.into(),
        role_mask: RoleMask::from(RoleBit::ADMIN),
    }
}

#[tokio::test]
async fn create_allocates_index_and_updates_count() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let created = create(&mock, &mock, token("user-1"), create_data("workset-1")).await;
    assert!(created.is_ok());
    let created = created.ok().unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(created.id, snapshot.comics[0].id);
    assert_eq!(snapshot.comics[0].index, 1);
    assert_eq!(snapshot.worksets[0].comic_count, 1);
    assert_eq!(snapshot.worksets[0].comic_next_index, 1);
    assert_eq!(snapshot.comics.len(), 1);
    assert_eq!(snapshot.comics[0].creator_id, "user-1");
}

#[tokio::test]
async fn create_rolls_back_missing_workset() {
    let mock = Mock::new();

    let err = create(&mock, &mock, token("user-1"), create_data("missing"))
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(snapshot.comics.is_empty());
}

#[tokio::test]
async fn get_info_returns_uploaded_cover_url() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover.png",
    ));

    let found = get_info(&mock, &mock, token("user-1"), "comic-1".into()).await;
    assert!(found.is_ok());
    let found = found.ok().unwrap();

    assert_eq!(found.id, "comic-1");
    assert_eq!(
        found.cover_url,
        Some("https://test.local/get/cover.png".into())
    );
}

#[tokio::test]
async fn get_info_propagates_missing_comic() {
    let mock = Mock::new();

    let err = get_info(&mock, &mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_index() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-2", "workset-1", 2));
    mock.seed_comic(comic("comic-1", "workset-1", 1));
    mock.seed_comic(comic("comic-other", "workset-2", 0));

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            workset_id: "workset-1".into(),
            with: vec![],
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "comic-1");
    assert_eq!(list[1].id, "comic-2");
}

#[tokio::test]
async fn list_infos_returns_empty_for_workset_contents() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            workset_id: "workset-1".into(),
            with: vec![],
        },
    )
    .await;
    assert!(list.is_ok());

    assert!(list.ok().unwrap().is_empty());
}

#[tokio::test]
async fn update_info_updates_comic() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let result = update_info(
        &mock,
        token("user-1"),
        UpdateComicInfoData {
            id: "comic-1".into(),
            title: "updated".into(),
            author: "updated-author".into(),
            description: Some("updated-desc".into()),
        },
    )
    .await;
    assert!(result.is_ok());
    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics[0].title, "updated");
    assert_eq!(snapshot.comics[0].author, "updated-author");
    assert_eq!(snapshot.comics[0].description, Some("updated-desc".into()));
}

#[tokio::test]
async fn update_info_propagates_missing_comic() {
    let mock = Mock::new();

    let err = update_info(
        &mock,
        token("user-1"),
        UpdateComicInfoData {
            id: "missing".into(),
            title: "updated".into(),
            author: "updated-author".into(),
            description: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn reserve_cover_updates_state_enqueues_check_and_returns_put_url() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let reserved = reserve_cover(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "comic-1".into(),
        ReserveComicCoverData {
            file_ext: "png".into(),
        },
    )
    .await;
    assert!(reserved.is_ok());
    let reserved = reserved.ok().unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(reserved.cover_version, 1);
    assert_eq!(
        reserved.put_url,
        "https://test.local/put/comic_cover/comic-1-1.png"
    );
    assert_eq!(snapshot.comics[0].cover_version, 1);
    assert_eq!(snapshot.prom_records.len(), 1);
    assert!(matches!(
        &snapshot.prom_records[0].payload,
        Payload::Image(ImageIntention::CheckUploaded {
            kind: ImageKind::ComicCover,
            resource_id,
            ..
        }) if resource_id == "comic-1"
    ));
}

#[tokio::test]
async fn reserve_cover_rolls_back_missing_comic() {
    let mock = Mock::new();

    let err = reserve_cover(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "missing".into(),
        ReserveComicCoverData {
            file_ext: "png".into(),
        },
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
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    let result = mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 2 },
    )
    .await;
    assert!(result.is_ok());

    assert!(mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_stale_version() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    let err = mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 1 },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(!mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn delete_removes_comic_updates_count_and_enqueues_cover_delete() {
    let mock = Mock::new();
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_workset(WorksetInfo {
        comic_count: 1,
        comic_next_index: 1,
        ..workset("workset-1", "team-1")
    });
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover.png",
    ));

    let result = delete(&mock, &mock, &mock, token("user-1"), "comic-1".into()).await;
    assert!(result.is_ok());
    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());
    assert_eq!(snapshot.worksets[0].comic_count, 0);
    assert_eq!(snapshot.prom_records.len(), 1);
    assert!(matches!(
        &snapshot.prom_records[0].payload,
        Payload::Image(ImageIntention::Delete { object_key }) if object_key == "cover.png"
    ));
}

#[tokio::test]
async fn delete_rolls_back_missing_comic() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let err = delete(&mock, &mock, &mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(snapshot.worksets.len(), 1);
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn mark_completed_updates_completion_state() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let result = mark_completed(&mock, &mock, token("user-1"), "comic-1".into(), true).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().comics[0].is_completed);
}

#[tokio::test]
async fn mark_completed_rolls_back_missing_comic() {
    let mock = Mock::new();
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let err = mark_completed(&mock, &mock, token("user-1"), "missing".into(), true)
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(!snapshot.comics[0].is_completed);
}
