use poprako_orchestra::{OperRun as _, Run};

use crate::part::repo::oper::chapter::StartChapterStage;
use crate::result::BaseError;
use crate::value::chapter::Stage;

/// Advances chapter stages in detached best-effort tasks.
pub fn advance_stages<R>((repo,): (R,), chapter_id: String, stages: Vec<Stage>)
where
    R: for<'a> Run<StartChapterStage<'a>, Error = BaseError>
        + Send
        + Sync
        + 'static,
{
    // NOTE: Advancements are intentionally best-effort. A task may be dropped
    // or fail because the same stage can be advanced manually when needed.
    tokio::spawn(async move {
        //
        for stage in stages {
            //
            let outcome = StartChapterStage {
                id: &chapter_id,
                stage,
            }
            .run_on(&repo)
            .await;

            if let Err(error) = outcome {
                //
                tracing::warn!(
                    err = ?error,
                    chapter_id = %chapter_id,
                    stage = ?stage,
                    "detached chapter stage advancement failed",
                );
            }
        }
    });
}
