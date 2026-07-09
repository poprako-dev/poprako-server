// create(create)(positive): creating a comic should allocate workset-scoped index and update comic count.
// create(create)(negative): missing workset should rollback without creating a comic.
// get_info(get_info)(positive): existing comic should return uploaded cover URL.
// get_info(get_info)(negative): missing comic should propagate an argument error.
// list_infos(list_infos)(positive): list should return workset comics sorted by last activity.
// list_infos(list_infos)(positive): empty workset contents should return an empty list after membership.
// list_infos(list_infos)(positive): fuzzy title should narrow results by display index, title, or author substring.
// list_infos(list_infos)(positive): is_completed filter should narrow results by completion state.
// list_infos(list_infos)(positive): stages filter should narrow by pinned chapter workflow state.
// list_infos(list_infos)(positive): pagination should be applied after filtering and sorting.
// list_infos(list_infos)(negative): invalid stages filter should return an argument error.
// list_infos(list_infos)(negative): completed filter cannot be combined with stages filter.
// update_info(update_info)(positive): existing comic should update title, author, and description.
// update_info(update_info)(negative): missing comic should propagate an argument error.
// reserve_cover(reserve_cover)(positive): reservation should update cover state, enqueue check, and return put URL.
// reserve_cover(reserve_cover)(negative): missing comic should rollback cover and prom state.
// mark_cover_uploaded(mark_cover_uploaded)(positive): matching version should mark the comic cover uploaded.
// mark_cover_uploaded(mark_cover_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_cover_uploaded(mark_cover_uploaded)(negative): stale version should leave cover unuploaded.
// mark_cover_uploaded(mark_cover_uploaded)(negative): old reservation replay should fail without marking current cover uploaded.
// delete(delete)(positive): deleting a comic should remove it, decrement workset count, and enqueue cover deletion.
// delete(delete)(negative): missing comic should rollback state.
// mark_completed(mark_completed)(positive): marking completed should update completion state and enqueue archive task.
// mark_completed(mark_completed)(negative): missing comic should rollback state and leave no prom records.

use super::*;

use time::OffsetDateTime;

use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part::prom::Payload;
use crate::part::prom::task::{ComicTask, ImageKind, ImageTask};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{
    assert_expected_message, assert_expected_variant,
    assert_one_image_check_record,
};
use crate::usecase::team::tests::workset;
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::role::{RoleField, RoleMask};

fn comic(id: &str, workset_id: &str, index: i32) -> ComicInfo {
    //
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
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn comic_with_uploaded_cover(
    id: &str,
    workset_id: &str,
    cover_key: &str,
) -> ComicInfo {
    ComicInfo {
        cover_key: Some(cover_key.into()),
        cover_uploaded: true,
        cover_version: 1,
        ..comic(id, workset_id, 0)
    }
}

fn chapter(id: &str, comic_id: &str, stage_mask: StageMask) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: stage_mask,
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn create_data(workset_id: &str) -> CreateComicData {
    CreateComicData {
        workset_id: workset_id.into(),
        title: "new".into(),
        author: "author".into(),
        description: Some("desc".into()),
        first_chapter_subtitle: None,
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
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

#[tokio::test]
async fn create_allocates_index_and_updates_count() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let created =
        create(&mock, &mock, token("user-1"), create_data("workset-1")).await;
    assert!(created.is_ok());
    let created = created.ok().unwrap();
    let snapshot = mock.snapshot();

    // Comic
    assert_eq!(created.id, snapshot.comics[0].id);
    assert_eq!(snapshot.comics[0].index, 0);
    assert_eq!(snapshot.comics[0].creator_id, "user-1");
    assert_eq!(snapshot.comics.len(), 1);

    // Workset
    assert_eq!(snapshot.worksets[0].comic_count, 1);
    assert_eq!(snapshot.worksets[0].comic_next_index, 1);

    // First chapter
    assert_eq!(snapshot.chapters.len(), 1);
    assert_eq!(snapshot.chapters[0].id, created.chapter_id);
    assert_eq!(snapshot.chapters[0].comic_id, created.id);
    assert!(snapshot.chapters[0].is_pinned);
    assert_eq!(snapshot.chapters[0].index, 0);

    // Denormalised chapter counters
    assert_eq!(snapshot.comics[0].chapter_count, 1);
    assert_eq!(snapshot.comics[0].chapter_next_index, 1);

    // last_active_at should be set (not epoch)
    assert!(snapshot.comics[0].last_active_at.unix_timestamp() > 0);

    // Creator admin assignment
    assert_eq!(snapshot.assignments.len(), 1);
    assert_eq!(snapshot.assignments[0].chapter_id, created.chapter_id);
    assert_eq!(snapshot.assignments[0].user_id, "user-1");
    assert!(
        snapshot.assignments[0]
            .roles
            .has_any_role(&[RoleField::ADMIN])
    );
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
async fn list_infos_filters_and_sorts_by_last_activity() {
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
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: None,
            stages: None,
            offset: 0,
            limit: 10,
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
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());

    assert!(list.ok().unwrap().is_empty());
}

#[tokio::test]
async fn list_infos_filters_by_fuzzy_title() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(ComicInfo {
        title: "Alpha Adventure".into(),
        author: "Alice".into(),
        ..comic("comic-alpha", "workset-1", 0)
    });
    mock.seed_comic(ComicInfo {
        title: "Beta Journey".into(),
        author: "Bob".into(),
        ..comic("comic-beta", "workset-1", 1)
    });
    mock.seed_comic(ComicInfo {
        title: "Gamma Quest".into(),
        author: "Carol".into(),
        ..comic("comic-gamma", "workset-1", 2)
    });

    // Match by title substring
    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("Beta".into()),
            is_completed: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-beta");

    // Match by author substring
    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("Carol".into()),
            is_completed: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-gamma");

    // Match by display index
    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("1".into()),
            is_completed: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-alpha");
}

#[tokio::test]
async fn list_infos_filters_by_is_completed() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(ComicInfo {
        is_completed: true,
        ..comic("comic-done", "workset-1", 0)
    });
    mock.seed_comic(ComicInfo {
        is_completed: false,
        ..comic("comic-ongoing", "workset-1", 1)
    });

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: Some(true),
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-done");
}

#[tokio::test]
async fn list_infos_filters_by_pinned_chapter_stages() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-active", "workset-1", 0));
    mock.seed_comic(comic("comic-pending", "workset-1", 1));

    let completed_translate_mask = StageMask::try_from(0u32)
        .ok()
        .unwrap()
        .try_set_phase(Stage::Translate, StagePhase::Completed)
        .ok()
        .unwrap();

    mock.seed_chapter(chapter(
        "chapter-active",
        "comic-active",
        completed_translate_mask,
    ));
    mock.seed_chapter(chapter(
        "chapter-pending",
        "comic-pending",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    let filter_mask = StageMask::try_filter_from(0u32)
        .ok()
        .unwrap()
        .try_set_phase(Stage::Translate, StagePhase::Completed)
        .ok()
        .unwrap();

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: Some(false),
            stages: Some(filter_mask.into()),
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-active");
}

#[tokio::test]
async fn list_infos_rejects_invalid_stages_filter() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let err = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: Some(false),
            stages: Some(0b01 << 8),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_rejects_completed_with_stages_filter() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let err = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: Some(true),
            stages: Some(0),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_applies_pagination() {
    let fixed_time = OffsetDateTime::now_utc();
    let mut comic_0_info = comic("comic-0", "workset-1", 0);
    comic_0_info.last_active_at = fixed_time;
    let mut comic_1_info = comic("comic-1", "workset-1", 1);
    comic_1_info.last_active_at = fixed_time;
    let mut comic_2_info = comic("comic-2", "workset-1", 2);
    comic_2_info.last_active_at = fixed_time;

    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic_0_info);
    mock.seed_comic(comic_1_info);
    mock.seed_comic(comic_2_info);

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListComicInfosData {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            is_completed: None,
            stages: None,
            offset: 1,
            limit: 1,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "comic-1");
}

#[tokio::test]
async fn update_info_updates_comic() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    update_info(
        &mock,
        token("user-1"),
        UpdateComicInfoData {
            id: "comic-1".into(),
            title: "updated".into(),
            author: "updated-author".into(),
            description: Some("updated-desc".into()),
        },
    )
    .await
    .ok()
    .unwrap();
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
    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::ComicCover,
        "comic-1",
        "comic_cover/comic-1-1.png",
        1,
    );
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

    mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 2 },
    )
    .await
    .ok()
    .unwrap();

    assert!(mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_accepts_repeated_matching_version() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(ComicInfo {
        cover_key: Some("cover.png".into()),
        cover_version: 2,
        ..comic("comic-1", "workset-1", 0)
    });

    let first = mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 2 },
    )
    .await;
    assert!(first.is_ok());
    let second = mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 2 },
    )
    .await;
    assert!(second.is_ok());

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

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-cover-upload",
    );
    assert!(!mock.snapshot().comics[0].cover_uploaded);
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_old_reservation_replay() {
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
    .await
    .ok()
    .unwrap();
    assert_eq!(reserved.cover_version, 2);

    let err = mark_cover_uploaded(
        &mock,
        token("user-1"),
        "comic-1".into(),
        MarkComicCoverUploadedData { cover_version: 1 },
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

    delete(&mock, &mock, &mock, token("user-1"), "comic-1".into())
        .await
        .ok()
        .unwrap();
    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());
    assert_eq!(snapshot.worksets[0].comic_count, 0);
    assert_eq!(snapshot.prom_records.len(), 1);
    assert!(matches!(
        snapshot.prom_records[0].payload(),
        Payload::Image(ImageTask::Delete { object_key }) if object_key == "cover.png"
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

    mark_completed(&mock, &mock, &mock, token("user-1"), "comic-1".into())
        .await
        .ok()
        .unwrap();

    let snapshot = mock.snapshot();
    assert!(snapshot.comics[0].is_completed);

    assert_eq!(snapshot.prom_records.len(), 1);
    assert_eq!(snapshot.prom_records[0].topic(), "comic_archive");
    assert_eq!(
        snapshot.prom_records[0].payload(),
        Payload::Comic(ComicTask::Archive {
            comic_id: "comic-1"
        }),
    );
}

#[tokio::test]
async fn mark_completed_rolls_back_missing_comic() {
    let mock = Mock::new();
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let err =
        mark_completed(&mock, &mock, &mock, token("user-1"), "missing".into())
            .await
            .err()
            .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(!snapshot.comics[0].is_completed);
    assert!(snapshot.prom_records.is_empty());
}

// process_pending_archive(mark_completed, process_pending)(positive):
//   After mark_completed enqueues ComicTask::Archive, calling
//   process_pending executes delete_cascade: the comic is removed.
#[tokio::test]
async fn process_pending_archive_executes_cascade_delete() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    mark_completed(&mock, &mock, &mock, token("user-1"), "comic-1".into())
        .await
        .ok()
        .unwrap();

    let snapshot = mock.snapshot();
    assert!(snapshot.comics[0].is_completed);
    assert_eq!(snapshot.prom_records.len(), 1);
    assert_eq!(
        snapshot.prom_records[0].payload(),
        Payload::Comic(ComicTask::Archive {
            comic_id: "comic-1"
        }),
    );

    crate::part_impl::prom::mock_impl::process_pending(&mock)
        .await
        .ok()
        .unwrap();

    let snapshot = mock.snapshot();
    assert!(
        !snapshot.comics.iter().any(|c| c.id == "comic-1"),
        "comic should be removed after archive cascade"
    );
}
