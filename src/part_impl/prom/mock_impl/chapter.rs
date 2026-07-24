use poprako_orchestra::Run as _;

use crate::part::effect::EffectDevelop;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedPayload;
use crate::part::prom::payload::chapter::AdvanceRawProvide;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseResult, accept};
use crate::value::chapter::Stage;

/// Process an [`AdvanceRawProvide`] task.
pub async fn process_advance_raw_provide(
    mock: &Mock,
    task: &AdvanceRawProvide,
) -> BaseResult<()> {
    //
    let advanced = mock
        .run(&CompleteChapterRawProvide {
            id: &task.chapter_id,
        })
        .await?;

    if advanced {
        mock.develop(Event::ChapterWorkflowCompleted(
            ChapterWorkflowCompletedPayload {
                chapter_id: task.chapter_id.clone(),
                completed_stage: Stage::RawProvide,
            },
        ))
        .await;
    }

    accept(())
}
