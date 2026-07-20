//! Handler for deferred chapter workflow advancement.

use tracing::instrument;

use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;

/// Completes raw provision only when all chapter pages are uploaded.
#[instrument(level = "info", skip_all)]
pub async fn handle<R>(repo: &R, task: &CheckUploadFinish) -> TaskFlow
where
    R: ChapterRepo<RdbContext> + Send + Sync,
{
    let result = repo
        .run(&CompleteChapterRawProvide {
            id: &task.chapter_id,
        })
        .await;

    match result {
        //
        Ok(_) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
