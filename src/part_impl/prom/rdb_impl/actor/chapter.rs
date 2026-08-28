//! Actor for deferred chapter workflow advancement.

use poprako_orchestra::{Nucl, OperStep as _};
use tracing::instrument;

use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part_impl::prom::rdb_impl::actor::task_flow::TaskFlow;
use crate::result::{BaseError, accept};
use crate::shared::RdbContext;
use crate::value::chapter::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Attempts raw-provision completion once and completes even while uploads remain pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R, D>(
    nucl: &N,
    repo: &R,
    develop: &D,
    task: &ChapterPayload,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + Send
        + Sync,
    D: Develop + Sync,
{
    match task {
        //
        ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id,
            actor_user_id,
        } => {
            //
            handle_raw_provide(
                nucl,
                repo,
                develop,
                chapter_id,
                actor_user_id.clone(),
            )
            .await
        }
    }
}

// Internal implementation of `handle_raw_provide`.
async fn handle_raw_provide<N, R, D>(
    nucl: &N,
    repo: &R,
    develop: &D,
    chapter_id: &str,
    actor_user_id: Option<String>,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + Send
        + Sync,
    D: Develop + Sync,
{
    let outcome = nucl
        .coord(async move |context| {
            //
            // Internal implementation detail.
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and chapter upload-summary races.
            GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let advanced = CompleteChapterRawProvide { id: chapter_id }
                .step_on(repo, context)
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
                .step_on(repo, context)
                .await?;
            }

            accept(advanced)
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        // Internal implementation detail.
        Ok(true) => {
            //
            // Internal implementation detail.
            Event::ChapterWorkflowCompleted {
                payload: ChapterWorkflowCompletedEvent {
                    chapter_id: chapter_id.to_string(),
                    completed_stage: Stage::RawProvide,
                },
            }
            .develop_on(develop)
            .await;

            TaskFlow::Complete
        }

        Ok(false) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
    }
}
