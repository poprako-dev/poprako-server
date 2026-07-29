use poprako_orchestra::Run;

use crate::part::repo::oper::chapter::StartChapterStage;
use crate::result::BaseError;
use crate::value::chapter::Stage;

/// Starts requested chapter stages in a detached best-effort task.
pub fn spawn_starts<R>(repo: R, chapter_id: String, stages: Vec<Stage>)
where
    R: for<'a> Run<StartChapterStage<'a>, Error = BaseError>
        + Send
        + Sync
        + 'static,
{
    tokio::spawn(async move {
        for stage in stages {
            //
            let result = repo
                .run(&StartChapterStage {
                    id: &chapter_id,
                    stage,
                })
                .await;

            if let Err(error) = result {
                tracing::warn!(
                    error = ?error,
                    chapter_id = %chapter_id,
                    stage = ?stage,
                    "detached chapter stage advancement failed",
                );
            }
        }
    });
}
