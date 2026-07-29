// list_infos(list_infos)(positive): team member can list chapters sorted by newest index with pagination.
// list_infos(list_infos)(negative): non-member cannot list chapters.
// get_info(get_info)(positive): team member can read a chapter.
// get_info(get_info)(negative): missing chapter returns an argument error.
// get_pinned(get_pinned)(positive): pinned chapter is returned and missing pinned chapter returns none.
// get_pinned(get_pinned)(negative): non-member cannot read pinned chapter.
// create(create)(positive): team admin creates pinned chapter, unpins previous chapter, updates comic, and creates admin assignment.
// create(create)(positive): creator preset roles are merged with chapter admin.
// create(create)(negative): non-admin creation rolls back.
// create(create)(negative): creator cannot preset a role missing from team membership.
// update_info(update_info)(positive): chapter admin can update metadata and pin state.
// update_info(update_info)(negative): non-admin cannot update metadata.
// update_stage(update_stage)(positive): chapter admin can advance any stage.
// update_stage(update_stage)(negative): reviewer cannot advance another role's stage.
// update_stage(update_stage)(negative): invalid workflow transition is rejected.
// update_stage(update_stage)(positive): publishing enqueues page image deletion.
// update_stage(update_stage)(positive): role holder advances own stage.
// update_stage(update_stage)(negative): admin cannot advance when no role holder exists.
// update_stage(update_stage)(positive): admin with workflow role advances when they hold the role.
// update_stage(update_stage)(positive): admin reverts stage even when no role holder exists.
// delete(delete)(positive): admin deletes chapter descendants, enqueues page image deletion, repins latest remaining chapter, and decrements comic.
// delete(delete)(negative): non-admin delete rolls back.

use super::*;

use self::fixture::*;
use crate::complex::chapter::ChapterComplex;
use crate::data::chapter::{
    CreateChapterParams, ListChapterInfosParams, UpdateChapterInfoParams,
    UpdateChapterStageParams,
};
use crate::part::prom::payload::Payload;
use crate::part::prom::payload::image::Payload as ImagePayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::{ChapterInclOpt, Stage};
use crate::value::image::{ImageExtension, ImageHash};
use crate::value::role::{RoleField, RoleMask};

mod fixture;
mod preset_assignment;
mod stage;

#[tokio::test]
async fn list_infos_paginates_sorted_chapters() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_chapter(chapter("chapter-3", "comic-1", 3, false));

    mock.seed_chapter(chapter("chapter-2", "comic-1", 2, false));

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListChapterInfosParams {
            incl_opt: Vec::new(),
            comic_id: "comic-1".into(),
            offset: 1,
            limit: 1,
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.ok().unwrap();

    assert_eq!(list.len(), 1);

    assert_eq!(list[0].id, "chapter-2");
}

#[tokio::test]
async fn list_infos_resolves_embedded_comic_fallback_cover() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    mock.seed_page(page("page-1", "chapter-1", Some("page.png")));

    let found = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListChapterInfosParams {
            comic_id: "comic-1".into(),
            incl_opt: vec![ChapterInclOpt::Comic],
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(
        found[0].comic.as_ref().unwrap().cover_url,
        Some("https://test.local/get/page.png".into())
    );
}

#[tokio::test]
async fn list_infos_rejects_non_member() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "other", RoleMask::from(RoleField::TRANSLATOR));

    let err = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListChapterInfosParams {
            incl_opt: Vec::new(),
            comic_id: "comic-1".into(),
            offset: 0,
            limit: 20,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn get_info_returns_chapter() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    let found = get_info(&mock, token("user-1"), "chapter-1".into()).await;

    assert!(found.is_ok());

    assert_eq!(found.ok().unwrap().id, "chapter-1");
}

#[tokio::test]
async fn get_info_rejects_missing_chapter() {
    //
    let mock = Mock::new();

    let err = get_info(&mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn get_pinned_returns_some_and_none() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    let found = get_pinned(&mock, token("user-1"), "comic-1".into()).await;

    assert!(found.is_ok());

    assert_eq!(found.ok().unwrap().unwrap().id, "chapter-1");

    let empty_mock = Mock::new();

    seed_scope(&empty_mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    let found =
        get_pinned(&empty_mock, token("user-1"), "comic-1".into()).await;

    assert!(found.is_ok());

    assert!(found.ok().unwrap().is_none());
}

#[tokio::test]
async fn get_pinned_rejects_non_member() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "other", RoleMask::from(RoleField::TRANSLATOR));

    let err = get_pinned(&mock, token("user-1"), "comic-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_pins_chapter_and_creates_admin_assignment() {
    //
    let mock = Mock::new();

    let member_roles = RoleMask::from(RoleField::ADMIN)
        .union(RoleMask::from(RoleField::TRANSLATOR));

    seed_scope(&mock, "user-1", member_roles);

    mock.seed_chapter(chapter("chapter-old", "comic-1", 0, true));

    let created = create(
        &mock,
        &mock,
        token("user-1"),
        CreateChapterParams {
            comic_id: "comic-1".into(),
            subtitle: None,
            preset_assignment_roles: Some(RoleMask::from(
                RoleField::TRANSLATOR,
            )),
        },
    )
    .await;

    assert!(created.is_ok());

    let snapshot = mock.snapshot();

    let created_id = created.ok().unwrap().id;

    assert_eq!(snapshot.chapters.len(), 2);

    let default_subtitle = ChapterComplex::subtitle_or_default(None, 1);

    assert!(snapshot.chapters.iter().any(|chapter_info| {
        chapter_info.id == created_id
            && chapter_info.is_pinned
            && chapter_info.subtitle == default_subtitle
            && chapter_info.index == 1
    }));

    assert!(
        snapshot
            .chapters
            .iter()
            .any(|chapter_info| chapter_info.id == "chapter-old"
                && !chapter_info.is_pinned)
    );

    assert_eq!(snapshot.comics[0].chapter_count, 3);

    assert!(
        snapshot.assignments[0]
            .roles
            .has_every_role(&[RoleField::ADMIN, RoleField::TRANSLATOR])
    );
}

#[tokio::test]
async fn create_rolls_back_non_admin() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    let err = create(
        &mock,
        &mock,
        token("user-1"),
        CreateChapterParams {
            comic_id: "comic-1".into(),
            subtitle: Some("new".into()),
            preset_assignment_roles: None,
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());

    assert_eq!(snapshot.comics[0].chapter_count, 2);
}

#[tokio::test]
async fn update_info_admin_updates_metadata() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    update_info(
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterInfoParams {
            id: "chapter-1".into(),
            subtitle: Some("updated".into()),
            pin: Some(true),
        },
    )
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapters[0].subtitle, "updated");

    assert!(snapshot.chapters[0].is_pinned);
}

#[tokio::test]
async fn update_info_rejects_non_admin_metadata() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    let err = update_info(
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterInfoParams {
            id: "chapter-1".into(),
            subtitle: Some("updated".into()),
            pin: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn delete_removes_descendants_and_repins_latest_chapter() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    mock.seed_chapter(chapter("chapter-2", "comic-1", 2, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-2",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_page(page("page-1", "chapter-1", Some("page-1.png")));

    delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into())
        .await
        .ok()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.chapters[0].id, "chapter-2");

    assert!(snapshot.chapters[0].is_pinned);

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.pages.is_empty());

    assert_eq!(snapshot.comics[0].chapter_count, 1);

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn delete_rolls_back_non_admin() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    mock.seed_page(page("page-1", "chapter-1", Some("page-1.png")));

    let err = delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into())
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.pages.len(), 1);

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn update_stage_role_holder_advances_own_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Advance,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::Translate),
        StagePhase::Active
    );
}

#[tokio::test]
async fn update_stage_admin_rejected_when_no_role_holder() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let err = update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Advance,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_stage_admin_with_workflow_role_advances() {
    //
    let mock = Mock::new();

    seed_scope(
        &mock,
        "user-1",
        RoleMask::from(RoleField::ADMIN)
            .union(RoleMask::from(RoleField::TRANSLATOR)),
    );

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN)
            .union(RoleMask::from(RoleField::TRANSLATOR)),
    ));

    update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Advance,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::Translate),
        StagePhase::Active
    );
}

#[tokio::test]
async fn update_stage_admin_reverts_without_role_holder() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let mut chapter_info = chapter("chapter-1", "comic-1", 1, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Translate, StagePhase::Active)
        .ok()
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Revert,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::Translate),
        StagePhase::Pending
    );
}
