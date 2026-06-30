// list_infos(list_infos)(positive): team member can list chapters sorted by newest index with pagination.
// list_infos(list_infos)(negative): non-member cannot list chapters.
// get_info(get_info)(positive): team member can read a chapter.
// get_info(get_info)(negative): missing chapter returns an argument error.
// get_pinned(get_pinned)(positive): pinned chapter is returned and missing pinned chapter returns none.
// get_pinned(get_pinned)(negative): non-member cannot read pinned chapter.
// create(create)(positive): team admin creates pinned chapter, unpins previous chapter, updates comic, and creates admin assignment.
// create(create)(negative): non-admin creation rolls back.
// update_info(update_info)(positive): chapter admin can update metadata and pin state.
// update_info(update_info)(negative): non-admin cannot update metadata.
// update_stage(update_stage)(positive): workflow role can advance an allowed stage.
// update_stage(update_stage)(negative): invalid workflow transition is rejected.
// update_stage(update_stage)(positive): publishing enqueues page image deletion.
// delete(delete)(positive): admin deletes chapter descendants, enqueues page image deletion, repins latest remaining chapter, and decrements comic.
// delete(delete)(negative): non-admin delete rolls back.

use super::*;

use time::OffsetDateTime;

use crate::complex::chapter::ChapterComplex;
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::workset::WorksetInfo;
use crate::part::prom::Payload;
use crate::part::prom::intention::ImageIntention;
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::{WorkflowStage, WorkflowStageMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        comic_next_index: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 1,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 2,
        chapter_next_index: 2,
        creator_id: "user-1".into(),
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str, index: i32, is_pinned: bool) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        is_pinned,
        index,
        subtitle: format!("chapter {}", index),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: WorkflowStageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str, team_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        team_id: team_id.into(),
        roles: role_mask,
    }
}

fn assignment(chapter_id: &str, user_id: &str, role_mask: RoleMask) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn page(id: &str, chapter_id: &str, image_key: Option<&str>) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        index: 0,
        image_key: image_key.map(Into::into),
        image_uploaded: image_key.is_some(),
        image_version: 1,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock, user_id: &str, role_mask: RoleMask) {
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1"));
    mock.seed_member(member(user_id, "team-1", role_mask));
}

#[tokio::test]
async fn list_infos_paginates_sorted_chapters() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));
    mock.seed_chapter(chapter("chapter-3", "comic-1", 3, false));
    mock.seed_chapter(chapter("chapter-2", "comic-1", 2, false));

    let list = list_infos(
        &mock,
        token("user-1"),
        ListChapterInfosData {
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
async fn list_infos_rejects_non_member() {
    let mock = Mock::new();
    seed_scope(&mock, "other", RoleMask::from(RoleField::TRANSLATOR));

    let err = list_infos(
        &mock,
        token("user-1"),
        ListChapterInfosData {
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
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    let found = get_info(&mock, token("user-1"), "chapter-1".into()).await;
    assert!(found.is_ok());

    assert_eq!(found.ok().unwrap().id, "chapter-1");
}

#[tokio::test]
async fn get_info_rejects_missing_chapter() {
    let mock = Mock::new();

    let err = get_info(&mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn get_pinned_returns_some_and_none() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, true));

    let found = get_pinned(&mock, token("user-1"), "comic-1".into()).await;
    assert!(found.is_ok());
    assert_eq!(found.ok().unwrap().unwrap().id, "chapter-1");

    let empty_mock = Mock::new();
    seed_scope(&empty_mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    let found = get_pinned(&empty_mock, token("user-1"), "comic-1".into()).await;
    assert!(found.is_ok());
    assert!(found.ok().unwrap().is_none());
}

#[tokio::test]
async fn get_pinned_rejects_non_member() {
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
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));
    mock.seed_chapter(chapter("chapter-old", "comic-1", 2, true));

    let created = create(
        &mock,
        &mock,
        token("user-1"),
        CreateChapterData {
            comic_id: "comic-1".into(),
            subtitle: None,
        },
    )
    .await;
    assert!(created.is_ok());
    let snapshot = mock.snapshot();
    let created_id = created.ok().unwrap().id;

    assert_eq!(snapshot.chapters.len(), 2);
    let default_subtitle = ChapterComplex::subtitle_or_default(None, 2);

    assert!(snapshot.chapters.iter().any(|chapter_info| {
        chapter_info.id == created_id
            && chapter_info.is_pinned
            && chapter_info.subtitle == default_subtitle
            && chapter_info.index == 2
    }));
    assert!(
        snapshot
            .chapters
            .iter()
            .any(|chapter_info| chapter_info.id == "chapter-old" && !chapter_info.is_pinned)
    );
    assert_eq!(snapshot.comics[0].chapter_count, 3);
    assert_eq!(snapshot.comics[0].chapter_next_index, 3);
    assert!(
        snapshot.assignments[0]
            .roles
            .has_any_role(&[RoleField::ADMIN])
    );
}

#[tokio::test]
async fn create_rolls_back_non_admin() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    let err = create(
        &mock,
        &mock,
        token("user-1"),
        CreateChapterData {
            comic_id: "comic-1".into(),
            subtitle: Some("new".into()),
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
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let result = update_info(
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterInfoData {
            id: "chapter-1".into(),
            subtitle: Some("updated".into()),
            pin: Some(true),
        },
    )
    .await;
    assert!(result.is_ok());
    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapters[0].subtitle, "updated");
    assert!(snapshot.chapters[0].is_pinned);
}

#[tokio::test]
async fn update_info_rejects_non_admin_metadata() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    let err = update_info(
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterInfoData {
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
async fn update_stage_workflow_role_advances_stage() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let result = update_stage(
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageData {
            id: "chapter-1".into(),
            stage: WorkflowStage::Translate,
            event: WorkflowEvent::Advance,
        },
    )
    .await;
    assert!(result.is_ok());

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(WorkflowStage::Translate),
        StagePhase::Active
    );
}

#[tokio::test]
async fn update_stage_rejects_invalid_transition() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::PUBLISHER));
    let mut chapter_info = chapter("chapter-1", "comic-1", 1, false);
    chapter_info.stages = chapter_info
        .stages
        .set_phase(WorkflowStage::Publish, StagePhase::Completed);
    mock.seed_chapter(chapter_info);
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    let err = update_stage(
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageData {
            id: "chapter-1".into(),
            stage: WorkflowStage::Publish,
            event: WorkflowEvent::Advance,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_stage_publish_enqueues_page_image_delete() {
    let mock = Mock::new();
    seed_scope(&mock, "user-1", RoleMask::from(RoleField::PUBLISHER));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PUBLISHER),
    ));
    mock.seed_page(page("page-1", "chapter-1", Some("page-1.png")));

    let result = update_stage(
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageData {
            id: "chapter-1".into(),
            stage: WorkflowStage::Publish,
            event: WorkflowEvent::Advance,
        },
    )
    .await;
    assert!(result.is_ok());
    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 1);
    let Payload::Image(ImageIntention::Delete { object_key }) = &snapshot.prom_records[0].payload
    else {
        panic!("expected image delete payload");
    };
    assert_eq!(object_key, "page-1.png");
    assert_eq!(snapshot.pages[0].image_key.as_deref(), Some("page-1.png"));
    assert!(snapshot.pages[0].image_uploaded);
}

#[tokio::test]
async fn delete_removes_descendants_and_repins_latest_chapter() {
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

    let result = delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into()).await;
    assert!(result.is_ok());
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
