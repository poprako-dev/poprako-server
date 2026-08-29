use super::*;

use crate::value::chapter::stage::{StageOper, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

#[tokio::test]
async fn update_stage_admin_advances_any_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-2",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Publish.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    let stages = &snapshot.chapters[0].stages;

    assert_eq!(stages.get_phase(Stage::Publish), StagePhase::Completed);

    assert_eq!(snapshot.chapter_workflow_records.len(), 1);

    let workflow_record = &snapshot.chapter_workflow_records[0];

    assert_eq!(workflow_record.actor_user_id.as_deref(), Some("user-1"));

    assert!(matches!(
        &workflow_record.payload,
        ChapterWorkflowRecordPayload::StageTransitioned {
            stage: Stage::Publish,
            previous_phase: StagePhase::Pending,
            next_phase: StagePhase::Completed,
            origin: ChapterWorkflowRecordOrigin::Manual,
        }
    ));
}

#[tokio::test]
async fn update_stage_noop_does_not_create_workflow_record() {
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Revert.into(),
        },
    )
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapter_workflow_records.len(), 0);

    assert_eq!(
        snapshot.chapters[0].stages.get_phase(Stage::Translate),
        StagePhase::Pending,
    );
}

#[tokio::test]
async fn update_stage_rejects_reviewer_outside_review_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::REVIEWER));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let err = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_stage_rejects_invalid_transition() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::PUBLISHER));

    let mut chapter_info = chapter("chapter-1", "comic-1", 1, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .ok()
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    let err = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Publish.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn published_chapter_rejects_metadata_and_stage_updates() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let mut chapter_info = chapter("chapter-1", "comic-1", 0, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let info_result = update_info(
        (&mock, &mock),
        token("user-1"),
        UpdateChapterInfoInstr {
            id: "chapter-1".into(),
            subtitle: Some("changed".into()),
        },
    )
    .await;

    assert!(matches!(info_result, Err(BaseError::Expected { .. })));

    let stage_result = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await;

    assert!(matches!(stage_result, Err(BaseError::Expected { .. })));

    assert_eq!(mock.snapshot().chapters[0].subtitle, "chapter 0");
}
