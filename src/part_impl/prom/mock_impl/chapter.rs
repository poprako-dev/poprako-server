use poprako_orchestra::OperRun as _;

use crate::part::effect::EffectEvent as _;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseRest, accept};
use crate::value::chapter::Stage;

/// Process a [`ChapterPayload`] task.
pub async fn process(mock: &Mock, task: &ChapterPayload) -> BaseRest<()> {
    //
    match task {
        //
        ChapterPayload::TryAdvanceRawProvideStage { chapter_id } => {
            process_raw_provide(mock, chapter_id).await
        }
    }
}

// Internal implementation of `process_raw_provide`.
async fn process_raw_provide(mock: &Mock, chapter_id: &str) -> BaseRest<()> {
    //
    // Internal implementation detail.
    let advanced = CompleteChapterRawProvide { id: chapter_id }
        .run_on(mock)
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
