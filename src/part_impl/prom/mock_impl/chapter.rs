use poprako_orchestra::{Nucl as _, OperStep as _};

use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::EffectEvent as _;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseRest, accept};
use crate::value::chapter::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Process a [`ChapterPayload`] task.
pub async fn process(mock: &Mock, task: &ChapterPayload) -> BaseRest<()> {
    //
    match task {
        //
        ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id,
            actor_user_id,
        } => process_raw_provide(mock, chapter_id, actor_user_id.clone()).await,
    }
}

// Internal implementation of `process_raw_provide`.
async fn process_raw_provide(
    mock: &Mock,
    chapter_id: &str,
    actor_user_id: Option<String>,
) -> BaseRest<()> {
    //
    let advanced = mock
        .coord(async move |context| {
            //
            let advanced = CompleteChapterRawProvide { id: chapter_id }
                .step_on(mock, context)
                .await?;

            if advanced {
                //
                let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                    chapter_id,
                    actor_user_id,
                    ChapterWorkflowRecordPayload::StageTransitioned {
                        stage: Stage::RawProvide,
                        previous_phase: StagePhase::Pending,
                        next_phase: StagePhase::Completed,
                        origin: ChapterWorkflowRecordOrigin::RawProvideCheck,
                    },
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&workflow_record_entry),
                }
                .step_on(mock, context)
                .await?;
            }

            accept(advanced)
        })
        .await?;

    if advanced {
        //
        Event::ChapterWorkflowCompleted(ChapterWorkflowCompletedEvent {
            chapter_id: chapter_id.to_string(),
            completed_stage: Stage::RawProvide,
        })
        .develop_on(mock)
        .await;
    }

    accept(())
}
