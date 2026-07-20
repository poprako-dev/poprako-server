//! Handler for deferred chapter workflow advancement.

use tracing::instrument;

use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::chapter::CompleteChapterRawProvide;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::BaseResult;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_uploads_are_retried() {
        assert!(matches!(resolve_task_flow(Ok(false)), TaskFlow::Retry(_)));
    }

    #[test]
    fn resolved_uploads_are_completed() {
        assert!(matches!(resolve_task_flow(Ok(true)), TaskFlow::Complete));
    }
}

/// Completes raw provision or retries while chapter uploads are incomplete.
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

    resolve_task_flow(result)
}

fn resolve_task_flow(result: BaseResult<bool>) -> TaskFlow {
    match result {
        //
        Ok(true) => TaskFlow::Complete,

        Ok(false) => {
            TaskFlow::Retry("chapter page uploads are incomplete".into())
        }

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
