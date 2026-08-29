use poprako_orchestra::{Nucl as _, OperStep as _};
use time::OffsetDateTime;

use super::*;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page::PageInfo;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::prom::task::Task;
use crate::result::BaseError;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};

#[tokio::test]
async fn defer_records_non_object_payload() {
    let mock = Mock::new();

    let prom = mock.clone();

    mock.coord(async move |context| {
        let id = String::from("prom-invitation-1");

        let payload = TaskPayload::Invitation {
            payload: InvitationPayload::Member {
                invitation_id: String::from("invitation-1"),
            },
        };

        let task = Task {
            id: &id,
            payload: &payload,
            delay: None,
        };

        Defer::new(task).step_on(&prom, context).await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(snapshot.prom_records[0].id(), "prom-invitation-1");
}

#[tokio::test]
async fn chapter_task_waits_for_every_page_image() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    let Ok(stages) = StageMask::try_from(0) else {
        return;
    };

    mock.seed_chapter(ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 1,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages,
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    });

    mock.seed_page(PageInfo {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    });

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        let id = String::from("prom-chapter-1");

        let payload = TaskPayload::Chapter {
            payload: ChapterPayload::TryAdvanceRawProvideStage {
                chapter_id: "chapter-1".into(),
                actor_user_id: Some("user-1".into()),
            },
        };

        let task = Task {
            id: &id,
            payload: &payload,
            delay: None,
        };

        Defer::new(task).step_on(&prom, context).await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    process_pending(&mock).await.unwrap();

    let pending = mock.snapshot();

    assert!(pending.chapters.first().is_some_and(|chapter_info| {
        chapter_info
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    }));

    mock.seed_page_image_obj("page-1", "png");

    process_pending(&mock).await.unwrap();

    let completed = mock.snapshot();

    assert!(completed.chapters.first().is_some_and(|chapter_info| {
        chapter_info
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Completed)
    }));

    assert_eq!(completed.chapter_workflow_records.len(), 1);
}
