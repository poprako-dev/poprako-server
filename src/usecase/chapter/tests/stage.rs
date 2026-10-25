#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::task::ObjTask;
use time::OffsetDateTime;

use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceRawIdentInfo,
};
use crate::part_impl::repo::mock_impl::MockObjRecord;
use crate::value::chapter::stage::{StageOper, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

#[tokio::test]
async fn update_stage_admin_advances_any_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user--00000000001", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        1,
        false,
    ));

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Publish.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    let stages = &snapshot.chapters.first().unwrap().stages;

    assert_eq!(stages.get_phase(Stage::Publish), StagePhase::Completed);

    assert_eq!(snapshot.chapter_workflow_records.len(), 1);

    let workflow_record = snapshot.chapter_workflow_records.first().unwrap();

    assert_eq!(
        workflow_record.actor_user_id.as_deref(),
        Some("user--00000000001")
    );

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

    seed_scope(&mock, "user--00000000001", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        1,
        false,
    ));

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Revert.into(),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapter_workflow_records.len(), 0);

    assert_eq!(
        snapshot
            .chapters
            .first()
            .unwrap()
            .stages
            .get_phase(Stage::Translate),
        StagePhase::Pending,
    );
}

#[tokio::test]
async fn update_stage_rejects_reviewer_outside_review_stage() {
    //
    let mock = Mock::new();

    seed_scope(
        &mock,
        "user--00000000001",
        RoleMask::from(RoleField::REVIEWER),
    );

    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        1,
        false,
    ));

    mock.seed_assignment(assignment(
        "chapter--00000000001",
        "user--00000000001",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let error = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_stage_rejects_invalid_transition() {
    //
    let mock = Mock::new();

    seed_scope(
        &mock,
        "user--00000000001",
        RoleMask::from(RoleField::PUBLISHER),
    );

    let mut chapter_info =
        chapter("chapter--00000000001", "comic--00000000001", 1, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter--00000000001",
        "user--00000000001",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    let error = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Publish.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_stage_publish_enqueues_page_source_image_delete() {
    let mock = Mock::new();

    seed_scope(
        &mock,
        "user--00000000001",
        RoleMask::from(RoleField::PUBLISHER),
    );
    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        1,
        false,
    ));
    mock.seed_assignment(assignment(
        "chapter--00000000001",
        "user--00000000001",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    let created_at = OffsetDateTime::now_utc();

    mock.seed_page(PageSourceInfo {
        id: "page_source--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at,
        updated_at: created_at,
    });

    mock.state.lock().unwrap().page_source_raw_idents.insert(
        "page_source--00000000001".into(),
        PageSourceRawIdentInfo {
            page_source_id: "page_source--00000000001".into(),
            raw_ident: "source.png".into(),
            created_at,
            updated_at: created_at,
        },
    );

    let key = ObjKey {
        id: "page_source--00000000001".into(),
        ver: 1,
        image:
            "page_source/chapter--00000000001/page_source--00000000001/1.png"
                .into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_source_image")
        .or_default()
        .insert(
            "page_source--00000000001".into(),
            MockObjRecord {
                version: 1,
                meta: Some(ObjMeta {
                    key,
                    is_avail: true,
                    hash: vec![0; 32],
                    ext: "png".into(),
                }),
            },
        );

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Publish.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    let source_objs = snapshot.objs.get("page_source_image").unwrap();

    let source = source_objs.get("page_source--00000000001").unwrap();

    assert!(source.meta.is_none());
    assert!(snapshot.page_source_raw_idents.is_empty());
    assert_eq!(snapshot.pages.len(), 1);
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(
            task,
            ObjTask::Delete { key } if key.id == "page_source--00000000001"
        )
    }));

    let events = mock.drain_events();

    assert_eq!(events.len(), 2);
    assert!(matches!(
        *events.first().unwrap(),
        crate::part::effect::event::Event::ChapterWorkflowCompleted { .. }
    ));
    assert!(matches!(
        *events.get(1).unwrap(),
        crate::part::effect::event::Event::ChapterPublished { .. }
    ));
}

#[tokio::test]
async fn published_chapter_rejects_metadata_and_stage_updates() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user--00000000001", RoleMask::from(RoleField::ADMIN));

    let mut chapter_info =
        chapter("chapter--00000000001", "comic--00000000001", 0, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);

    let info_result = update_info(
        (&mock, &mock),
        token("user--00000000001"),
        UpdateChapterInfoInstr {
            id: "chapter--00000000001".into(),
            subtitle: Some("changed".into()),
        },
    )
    .await;

    assert!(matches!(info_result, Err(BaseError::Expected { .. })));

    let stage_result = update_stage(
        (&mock, &mock, &mock, &mock),
        token("user--00000000001"),
        UpdateChapterStageInstr {
            id: "chapter--00000000001".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Advance.into(),
        },
    )
    .await;

    assert!(matches!(stage_result, Err(BaseError::Expected { .. })));

    assert_eq!(
        mock.snapshot().chapters.first().unwrap().subtitle,
        "chapter 0"
    );
}
