// create(create)(positive): creating a comic should allocate workset-scoped index and update comic count.
// create(create)(positive): first-chapter creator preset roles are merged with chapter admin.
// create(create)(negative): missing workset should rollback without creating a comic.
// create(create)(negative): creator cannot preset a role missing from team membership.
// get_info(get_info)(positive): existing comic should return uploaded cover URL.
// get_info(get_info)(negative): missing comic should propagate an argument error.
// list_infos(list_infos)(positive): list should return workset comics sorted by last activity.
// list_infos(list_infos)(positive): empty workset contents should return an empty list after membership.
// list_infos(list_infos)(positive): fuzzy title should narrow results by display index, title, or author substring.
// list_infos(list_infos)(positive): stages filter should narrow by pinned chapter workflow state.
// list_infos(list_infos)(positive): pinned chapter assignments should be returned in comic order.
// list_infos(list_infos)(positive): pagination should be applied after filtering and sorting.
// list_infos(list_infos)(negative): invalid stages filter should return an argument error.
// list_infos(list_infos)(negative): pinned chapter assignments without pinned chapters should return an argument error.
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

use super::*;
use crate::data::instr::comic::{ListComicInfosInstr, UpdateComicInfoInstr};

use fixture::*;
use time::OffsetDateTime;

use crate::model::read::proj::comic::ComicInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::workset;
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::comic::ComicWithOpt;
use crate::value::role::{RoleField, RoleMask};

mod cover;
mod fixture;
mod list;
mod preset_assignment;

#[tokio::test]
async fn create_allocates_index_and_updates_count() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    let mut creator_member = admin_member("user-1", "team-1");

    creator_member.roles = creator_member
        .roles
        .union(RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_member(creator_member);

    let mut instr = create_instr("workset-1");

    instr.preset_assignment_roles = Some(RoleMask::from(RoleField::TRANSLATOR));

    let created = create((&mock, &mock), token("user-1"), instr).await;

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

    // First chapter
    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.chapters[0].id, created.chapter_id);

    assert_eq!(snapshot.chapters[0].comic_id, created.id);

    assert!(snapshot.chapters[0].is_pinned);

    assert_eq!(snapshot.chapters[0].index, 0);

    // Denormalised chapter counters
    assert_eq!(snapshot.comics[0].chapter_count, 1);

    // last_active_at should be set (not epoch)
    assert!(snapshot.comics[0].last_active_at.unix_timestamp() > 0);

    // Creator admin assignment
    assert_eq!(snapshot.assignments.len(), 1);

    assert_eq!(snapshot.assignments[0].chapter_id, created.chapter_id);

    assert_eq!(snapshot.assignments[0].user_id, "user-1");

    assert!(
        snapshot.assignments[0]
            .roles
            .has_every_role(&[RoleField::ADMIN, RoleField::TRANSLATOR])
    );
}

#[tokio::test]
async fn create_rolls_back_missing_workset() {
    //
    let mock = Mock::new();

    let err = create((&mock, &mock), token("user-1"), create_instr("missing"))
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(snapshot.comics.is_empty());
}

#[tokio::test]
async fn get_info_returns_uploaded_cover_url() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover.png",
    ));

    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_page(page("page-1", "chapter-1", 0, Some("fallback.png"), true));

    let found =
        get_info((&mock, &mock), token("user-1"), "comic-1".into()).await;

    assert!(found.is_ok());

    let found = found.ok().unwrap();

    assert_eq!(found.id, "comic-1");

    assert_eq!(
        found.cover_url,
        Some("https://test.local/get/cover.png".into())
    );

    assert_eq!(
        found.cover_thumbnail_url,
        Some("https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/cover.png".into())
    );
}

#[tokio::test]
async fn get_info_falls_back_to_uploaded_first_pinned_page() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1", 0));

    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_page(page("page-later", "chapter-1", 1, Some("later.png"), true));

    mock.seed_page(page("page-first", "chapter-1", 0, Some("first.png"), true));

    let found = get_info((&mock, &mock), token("user-1"), "comic-1".into())
        .await
        .ok()
        .unwrap();

    assert_eq!(
        found.cover_url,
        Some("https://test.local/get/first.png".into())
    );

    assert_eq!(
        found.cover_thumbnail_url,
        Some("https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/first.png".into())
    );
}

#[tokio::test]
async fn list_infos_omits_fallback_without_usable_first_pinned_page() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("no-chapter", "workset-1", 0));

    mock.seed_comic(comic("no-page", "workset-1", 1));

    mock.seed_comic(comic("not-uploaded", "workset-1", 2));

    mock.seed_chapter(chapter(
        "chapter-no-page",
        "no-page",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_chapter(chapter(
        "chapter-not-uploaded",
        "not-uploaded",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_page(page(
        "page-not-uploaded",
        "chapter-not-uploaded",
        0,
        Some("pending.png"),
        false,
    ));

    let found = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            incl_opt: Vec::new(),
            with_opt: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert!(
        found
            .comics
            .iter()
            .all(|comic_info| comic_info.cover_url.is_none())
    );

    assert!(
        found
            .comics
            .iter()
            .all(|comic_info| comic_info.cover_thumbnail_url.is_none())
    );
}

#[tokio::test]
async fn get_info_propagates_missing_comic() {
    //
    let mock = Mock::new();

    let err = get_info((&mock, &mock), token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_last_activity() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-2", "workset-1", 2));

    mock.seed_comic(comic("comic-1", "workset-1", 1));

    mock.seed_comic(comic("comic-other", "workset-2", 0));

    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    let list = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![ComicWithOpt::PinnedChapter],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.ok().unwrap();

    assert_eq!(list.comics.len(), 2);

    assert_eq!(list.comics[0].id, "comic-1");

    assert_eq!(list.comics[1].id, "comic-2");

    assert_eq!(list.pinned_chapters.len(), list.comics.len());

    assert_eq!(list.pinned_chapters[0].as_ref().unwrap().id, "chapter-1");

    assert!(list.pinned_chapters[1].is_none());

    assert_eq!(list.pinned_chapter_assignments.len(), list.comics.len());

    assert!(list.pinned_chapter_assignments[0].is_empty());

    assert!(list.pinned_chapter_assignments[1].is_empty());
}

#[tokio::test]
async fn list_infos_returns_empty_for_workset_contents() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let list = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await;

    assert!(list.is_ok());

    assert!(list.ok().unwrap().comics.is_empty());
}

#[tokio::test]
async fn list_infos_filters_by_pinned_chapter_stages() {
    //
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
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: Some(filter_mask.into()),
            offset: 0,
            limit: 10,
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.ok().unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics[0].id, "comic-active");
}

#[tokio::test]
async fn list_infos_rejects_invalid_stages_filter() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let err = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
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
async fn list_infos_applies_pagination() {
    //
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
        (&mock, &mock),
        token("user-1"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            offset: 1,
            limit: 1,
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.ok().unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics[0].id, "comic-1");
}

#[tokio::test]
async fn update_info_updates_comic() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1", 0));

    update_info(
        (&mock,),
        token("user-1"),
        UpdateComicInfoInstr {
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
    //
    let mock = Mock::new();

    let err = update_info(
        (&mock,),
        token("user-1"),
        UpdateComicInfoInstr {
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
