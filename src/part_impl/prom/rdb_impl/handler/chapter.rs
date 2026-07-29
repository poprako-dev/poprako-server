//! Handler for deferred chapter workflow advancement.

use tracing::instrument;

use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::BaseResult;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod tests;

/// Attempts raw-provision completion once and completes even while uploads remain pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<R>(repo: &R, task: &CheckUploadFinish) -> TaskFlow
where
    R: ChapterRepo<RdbContext> + Send + Sync,
{
    let outcome = repo
        .run(&CompleteChapterRawProvide {
            id: &task.chapter_id,
        })
        .await;

    resolve_task_flow(outcome)
}

fn resolve_task_flow(outcome: BaseResult<bool>) -> TaskFlow {
    match outcome {
        //
        Ok(true) => TaskFlow::Complete,

        Ok(false) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
