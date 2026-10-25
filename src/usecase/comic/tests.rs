#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

mod cover;
mod fixture;
mod incl;
mod list;
mod preset_assignment;
mod view;

// create(create)(positive): creating a comic should allocate workset-scoped index and update comic count.
// create(create)(positive): first-chapter creator receives only the preset worker roles.
// create(create)(negative): missing workset should rollback without creating a comic.
// create(create)(negative): creator cannot preset a role missing from team membership.
// get_info(get_info)(positive): existing comic should return uploaded cover URL.
// get_info(get_info)(negative): missing comic should propagate an argument error.
// list_infos(list_infos)(positive): list should return workset comics sorted by last activity.
// list_infos(list_infos)(positive): empty workset contents should return an empty list after membership.
// list_infos(list_infos)(positive): fuzzy title should narrow results by display index, title, or author substring.
// list_infos(list_infos)(positive): stages filter should narrow by pinned chapter workflow state.
// list_infos(list_infos)(positive): lifecycle filter should distinguish active and archived comics.
// list_infos(list_infos)(positive): pinned chapter assignments with users should be returned in comic order.
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
// delete(delete)(positive): deleting a comic should mark it, decrement the active count, then sweep it.
// delete(delete)(negative): missing comic should rollback state.

use super::*;

use fixture::*;
use poprako_obj_dept::model::record::ObjKeyRecord as ObjKey;
use poprako_obj_dept::model::record::ObjMetaRecord as ObjMeta;
use time::OffsetDateTime;

use crate::data::instr::comic::{
    ListComicInfosInstr, MarkComicCoverUploadedInstr, UpdateComicInfoInstr,
};
use crate::model::read::proj::comic::ComicInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::{invalid_credential, user, workset};
use crate::usecase::comic::list::list_infos;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::comic::ComicWithOpt;
use crate::value::role::{RoleField, RoleMask};

const USER_ID: &str = "user--00000000001";
const COMIC_ID: &str = "comic--00000000001";
const CHAPTER_ID: &str = "chapter--00000000001";

fn seed_comic_cover_scope(mock: &Mock, version: u32) {
    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_comic(comic(COMIC_ID, "workset--00000000001", 0));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    let key = ObjKey {
        id: COMIC_ID.into(),
        ver: version,
        domain: serde_json::json!({"comic_id": COMIC_ID, "ext": "png"}),
    };

    let meta = ObjMeta {
        key,
        is_avail: false,
        hash: vec![0; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("comic_cover")
        .or_default()
        .insert(
            COMIC_ID.into(),
            MockObjRecord {
                version,
                meta: Some(meta),
            },
        );
}

async fn mark_comic_cover(
    mock: &Mock,
    version: u32,
) -> crate::result::BaseRest<()> {
    super::cover::mark_uploaded::<MockContext, _, _>(
        (mock, mock),
        token(USER_ID),
        COMIC_ID.into(),
        MarkComicCoverUploadedInstr { image_ver: version },
    )
    .await
}

#[tokio::test]
async fn create_allocates_index_and_updates_count() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    let mut creator_member = admin_member(USER_ID, "team--00000000001");

    creator_member.roles = creator_member
        .roles
        .union(RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_member(creator_member);

    let mut instr = create_instr("workset--00000000001");

    instr.title = "\u{2003}new\u{00a0}".into();

    instr.author = "\u{2003}author\u{00a0}".into();

    instr.description = Some("\u{2003}desc\u{00a0}".into());

    instr.first_chapter_subtitle = Some("\u{2003}First\u{00a0}".into());

    instr.preset_assignment_roles = Some(RoleMask::from(RoleField::TRANSLATOR));

    let created = create((&mock, &mock), token(USER_ID), instr).await;

    assert!(created.is_ok());

    let created = created.unwrap();

    let snapshot = mock.snapshot();

    // Comic
    let comic_info = snapshot.comics.first().unwrap();

    assert_eq!(created.id, comic_info.id);

    assert_eq!(comic_info.index, 0);

    assert_eq!(comic_info.creator_id, USER_ID);

    assert_eq!(comic_info.title, "new");

    assert_eq!(comic_info.author, "author");

    assert_eq!(comic_info.description.as_deref(), Some("desc"));

    assert_eq!(snapshot.comics.len(), 1);

    // Workset
    assert_eq!(snapshot.worksets.first().unwrap().comic_count, 1);

    // First chapter
    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.chapters.first().unwrap().id, created.chapter_id);

    assert_eq!(snapshot.chapters.first().unwrap().comic_id, created.id);

    assert!(snapshot.chapters.first().unwrap().is_pinned);

    assert_eq!(snapshot.chapters.first().unwrap().index, 0);

    assert_eq!(snapshot.chapters.first().unwrap().subtitle, "First");

    // Denormalised chapter counters
    assert_eq!(comic_info.chapter_count, 1);

    // last_active_at should be set (not epoch)
    assert!(comic_info.last_active_at.unix_timestamp() > 0);

    // Creator worker assignment
    assert_eq!(snapshot.assignments.len(), 1);

    assert_eq!(
        snapshot.assignments.first().unwrap().chapter_id,
        created.chapter_id
    );

    assert_eq!(snapshot.assignments.first().unwrap().user_id, USER_ID);

    assert_eq!(
        snapshot.assignments.first().unwrap().roles,
        RoleMask::from(RoleField::TRANSLATOR)
    );

    assert_eq!(snapshot.chapter_workflow_records.len(), 1);

    let record = snapshot.chapter_workflow_records.first().unwrap();

    assert_eq!(record.chapter_id, created.chapter_id);

    assert_eq!(record.actor_user_id.as_deref(), Some(USER_ID));

    assert!(matches!(
            record.payload,
            crate::value::chapter_workflow_record::
                ChapterWorkflowRecordPayload::ChapterCreated
        ));
}

#[tokio::test]
async fn create_rolls_back_missing_workset() {
    let mock = Mock::new();

    let error = create((&mock, &mock), token(USER_ID), create_instr("missing"))
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(snapshot.comics.is_empty());
}

#[tokio::test]
async fn mark_cover_uploaded_marks_current_generation_idempotently() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 3);

    mark_comic_cover(&mock, 3).await.unwrap();

    mark_comic_cover(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot()
            .objs
            .get("comic_cover")
            .unwrap()
            .get(COMIC_ID)
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_stale_generation() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 3);

    let error = mark_comic_cover(&mock, 2).await.err().unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(
        !mock
            .snapshot()
            .objs
            .get("comic_cover")
            .unwrap()
            .get(COMIC_ID)
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn get_info_propagates_missing_comic() {
    let mock = Mock::new();

    let error = get_info(
        (&mock, &mock),
        token(USER_ID),
        "missing".into(),
        GetComicInfoInstr::default(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_last_activity() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    let baseline = OffsetDateTime::now_utc();

    for comic_info in comics_with_opposed_activity(baseline) {
        mock.seed_comic(comic_info);
    }

    mock.seed_comic(comic("comic--85117610840", "workset--00000000002", 0));

    mock.seed_chapter(chapter(
        CHAPTER_ID,
        COMIC_ID,
        StageMask::try_from(0u32).unwrap(),
    ));

    let list = list_infos(
        (&mock, &mock),
        token(USER_ID),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![ComicWithOpt::PinnedChapter],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.unwrap();

    assert_eq!(list.comics.len(), 2);

    assert_eq!(list.comics.first().unwrap().id, COMIC_ID);

    assert_eq!(list.comics.get(1).unwrap().id, "comic--00000000002");

    assert_eq!(list.pinned_chapters.len(), list.comics.len());

    assert_eq!(
        list.pinned_chapters.first().unwrap().as_ref().unwrap().id,
        CHAPTER_ID
    );

    assert!(list.pinned_chapters.get(1).unwrap().is_none());

    assert_eq!(list.pinned_chapter_assignments.len(), list.comics.len());

    assert!(list.pinned_chapter_assignments.first().unwrap().is_empty());

    assert!(list.pinned_chapter_assignments.get(1).unwrap().is_empty());
}

#[tokio::test]
async fn list_infos_returns_empty_for_workset_contents() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    let list = list_infos(
        (&mock, &mock),
        token(USER_ID),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await;

    assert!(list.is_ok());

    assert!(list.unwrap().comics.is_empty());
}

#[tokio::test]
async fn list_infos_filters_by_pinned_chapter_stages() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    mock.seed_comic(comic("comic--28999254184", "workset--00000000001", 0));

    mock.seed_comic(comic("comic--47740658756", "workset--00000000001", 1));

    let completed_translate_mask = StageMask::try_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Translate, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter(
        "chapter--28999254184",
        "comic--28999254184",
        completed_translate_mask,
    ));

    mock.seed_chapter(chapter(
        "chapter--47740658756",
        "comic--47740658756",
        StageMask::try_from(0u32).unwrap(),
    ));

    let filter_mask = StageMask::try_filter_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Translate, StagePhase::Completed)
        .unwrap();

    let list = list_infos(
        (&mock, &mock),
        token(USER_ID),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: Some(filter_mask.into()),
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics.first().unwrap().id, "comic--28999254184");
}

#[tokio::test]
async fn list_infos_rejects_invalid_stages_filter() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    let error = list_infos(
        (&mock, &mock),
        token(USER_ID),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: Some(0b01 << 8),
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_applies_pagination() {
    let fixed_time = OffsetDateTime::now_utc();

    let mut comic_0_info =
        comic("comic--00000000000", "workset--00000000001", 0);

    comic_0_info.last_active_at = fixed_time;

    let mut comic_1_info = comic(COMIC_ID, "workset--00000000001", 1);

    comic_1_info.last_active_at = fixed_time;

    let mut comic_2_info =
        comic("comic--00000000002", "workset--00000000001", 2);

    comic_2_info.last_active_at = fixed_time;

    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    mock.seed_comic(comic_0_info);

    mock.seed_comic(comic_1_info);

    mock.seed_comic(comic_2_info);

    let list = list_infos(
        (&mock, &mock),
        token(USER_ID),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 1,
            limit: crate::value::pagination::PubListLimit::new(1).unwrap(),
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics.first().unwrap().id, COMIC_ID);
}

#[tokio::test]
async fn update_info_updates_comic() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member(USER_ID, "team--00000000001"));

    mock.seed_comic(comic(COMIC_ID, "workset--00000000001", 0));

    update_info(
        (&mock,),
        token(USER_ID),
        UpdateComicInfoInstr {
            id: COMIC_ID.into(),
            title: "\u{2003}updated\u{00a0}".into(),
            author: "\u{2003}updated-author\u{00a0}".into(),
            description: Some("\u{2003}updated-desc\u{00a0}".into()),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.first().unwrap().title, "updated");

    assert_eq!(snapshot.comics.first().unwrap().author, "updated-author");

    assert_eq!(
        snapshot.comics.first().unwrap().description,
        Some("updated-desc".into())
    );
}

#[tokio::test]
async fn update_info_propagates_missing_comic() {
    let mock = Mock::new();

    let error = update_info(
        (&mock,),
        token(USER_ID),
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

    assert_expected_variant(&error, ExpectedVariant::Args);
}
