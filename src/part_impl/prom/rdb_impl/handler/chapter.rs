//! Handler for deferred chapter workflow advancement.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::effect::EffectDevelop;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedPayload;
use crate::part::prom::payload::chapter::AdvanceRawProvide;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded,
};
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, accept};
use crate::value::chapter::Stage;

/// Attempts raw-provision completion once and completes even while uploads remain pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R, V>(
    nucl: &N,
    repo: &R,
    develop: &V,
    task: &AdvanceRawProvide,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: ChapterRepo<RdbContext> + Send + Sync,
    V: EffectDevelop + Sync,
{
    let outcome: BaseResult<bool> = nucl
        .coord(async move |context| {
            //
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            repo.step(
                context,
                &GetChapterInfoExcluded {
                    id: &task.chapter_id,
                    incls: &[],
                },
            )
            .await?;

            let advanced = repo
                .step(
                    context,
                    &CompleteChapterRawProvide {
                        id: &task.chapter_id,
                    },
                )
                .await?;

            accept(advanced)
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        Ok(true) => {
            //
            develop
                .develop(Event::ChapterWorkflowCompleted(
                    ChapterWorkflowCompletedPayload {
                        chapter_id: task.chapter_id.clone(),
                        completed_stage: Stage::RawProvide,
                    },
                ))
                .await;

            TaskFlow::Complete
        }

        Ok(false) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
