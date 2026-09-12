use poprako_obj_dept::oper::ClearObjs;
use poprako_orchestra::{Nucl as _, OperStep as _};
use time::OffsetDateTime;

use super::*;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page::PageInfo;
use crate::part::obj_dept::PageImage;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::prom::task::Task;
use crate::part_impl::prom::task_flow::{TaskFlow, WAIT_TIMEOUT};
use crate::result::BaseError;
use crate::usecase::chapter::stage::{
    RawProvideAdvance, try_advance_raw_provide,
};
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};

// wait_deadline_preserves_success(TaskFlow)(negative): only unresolved waiting expires.
#[test]
fn wait_deadline_preserves_success() {
    let now = OffsetDateTime::now_utc();

    let requested_at = now - WAIT_TIMEOUT;

    let wait = || TaskFlow::Wait {
        err_message: "pending upload".into(),
    };

    assert!(matches!(
        wait().limit_wait(requested_at, now),
        TaskFlow::Dead { .. }
    ));

    assert!(matches!(wait().limit_wait(now, now), TaskFlow::Wait { .. }));

    assert!(matches!(
        TaskFlow::Complete.limit_wait(requested_at, now),
        TaskFlow::Complete
    ));

    assert!(matches!(
        wait().limit_wait(requested_at, now - time::Duration::seconds(1)),
        TaskFlow::Wait { .. }
    ));
}

// repeated_chapter_requests_remain_independent(Mock Defer)(positive): each request retains its own payload and delay.
#[tokio::test]
async fn repeated_chapter_requests_remain_independent() {
    let mock = Mock::new();

    for actor_id in ["first", "latest"] {
        let id = actor_id.to_owned();

        let payload = TaskPayload::Chapter {
            payload: ChapterPayload::TryAdvanceRawProvideStage {
                chapter_id: "shared-chapter".into(),
                actor_user_id: id.clone(),
            },
        };

        mock.coord(async |context| {
            let task = Task {
                id: &id,
                payload: &payload,
                delay: Some(std::time::Duration::from_mins(20)),
            };

            Defer::new(task).step_on(&mock, context).await
        })
        .await
        .unwrap();
    }

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 2);

    for (record, expected) in
        snapshot.prom_records.iter().zip(["first", "latest"])
    {
        assert!(matches!(record.payload(), TaskPayload::Chapter {
            payload: ChapterPayload::TryAdvanceRawProvideStage { actor_user_id, .. }
        } if actor_user_id == expected));

        assert!(record.visible_at() > OffsetDateTime::now_utc());
    }
}

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
                actor_user_id: "user-1".into(),
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

    let outcome = try_advance_raw_provide(
        (&mock, &mock, &mock, &mock),
        "chapter-1",
        None,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RawProvideAdvance::Pending));

    process_pending(&mock).await.unwrap();

    let pending = mock.snapshot();

    assert!(pending.chapters.first().is_some_and(|chapter_info| {
        chapter_info
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    }));

    mock.seed_page_image_obj("page-1", "png");

    let outcome = try_advance_raw_provide(
        (&mock, &mock, &mock, &mock),
        "chapter-1",
        Some("user-1".into()),
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RawProvideAdvance::Advanced));

    process_pending(&mock).await.unwrap();

    let completed = mock.snapshot();

    assert!(completed.chapters.first().is_some_and(|chapter_info| {
        chapter_info
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Completed)
    }));

    assert_eq!(completed.chapter_workflow_records.len(), 1);
}

// completed_raw_provide_ignores_missing_and_cleared_images(try_advance_raw_provide)(positive): completed work converges even when its former image objects are unavailable.
#[tokio::test]
async fn completed_raw_provide_ignores_missing_and_cleared_images() {
    for had_image in [false, true] {
        let mock = Mock::new();

        let time = OffsetDateTime::now_utc();

        let stages = StageMask::try_from(0)
            .unwrap()
            .try_set_phase(Stage::RawProvide, StagePhase::Completed)
            .unwrap();

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

        if had_image {
            mock.seed_page_image_obj("page-1", "png");

            mock.coord(async |context| {
                ClearObjs::<PageImage>::new(&["page-1".to_string()])
                    .step_on(&mock, context)
                    .await
                    .map_err(BaseError::from)
            })
            .await
            .unwrap();
        }

        let outcome = try_advance_raw_provide(
            (&mock, &mock, &mock, &mock),
            "chapter-1",
            None,
        )
        .await
        .unwrap();

        assert!(matches!(outcome, RawProvideAdvance::Unchanged));

        assert!(mock.snapshot().chapter_workflow_records.is_empty());
    }
}

// empty_raw_provide_does_not_wait_or_complete_the_stage(try_advance_raw_provide)(positive): an empty chapter has no upload work to wait for and produces no completion event.
#[tokio::test]
async fn empty_raw_provide_does_not_wait_or_complete_the_stage() {
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_chapter(ChapterInfo {
        id: "chapter-empty".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0).unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    });

    let outcome = try_advance_raw_provide(
        (&mock, &mock, &mock, &mock),
        "chapter-empty",
        None,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, RawProvideAdvance::Unchanged));

    let snapshot = mock.snapshot();

    assert!(
        snapshot.chapters[0]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    );

    assert!(snapshot.chapter_workflow_records.is_empty());
}
