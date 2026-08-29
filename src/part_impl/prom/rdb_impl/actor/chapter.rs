//! Actor for deferred chapter workflow advancement.

use poprako_orchestra::{Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::{ObjDept, obj_inst};

use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::page::ListPageInfos;
use crate::part::repo::page::PageRepo;
use crate::part_impl::prom::rdb_impl::actor::task_flow::TaskFlow;
use crate::result::{BaseError, accept};
use crate::shared::RdbContext;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Attempts raw-provision completion and waits while uploads remain pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R, O, D>(
    nucl: &N,
    repo: &R,
    obj_dept: &O,
    develop: &D,
    task: &ChapterPayload,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    O: ObjDept<PageImage, RdbContext> + Sync,
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
                obj_dept,
                develop,
                chapter_id,
                actor_user_id.clone(),
            )
            .await
        }
    }
}

// Internal implementation of `handle_raw_provide`.
async fn handle_raw_provide<N, R, O, D>(
    nucl: &N,
    repo: &R,
    obj_dept: &O,
    develop: &D,
    chapter_id: &str,
    actor_user_id: Option<String>,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    O: ObjDept<PageImage, RdbContext> + Sync,
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

            let page_infos =
                ListPageInfos { chapter_id }.step_on(repo, context).await?;

            for page_info in &page_infos {
                //
                let obj_meta = obj_inst! {
                    GetObjMeta<PageImage> { id: &page_info.id }
                }
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

                if !obj_meta.is_some_and(|obj_meta| obj_meta.f_is_uploaded) {
                    return accept(None);
                }
            }

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

            accept(Some(advanced))
        })
        .await
        .map_err(Into::into);

    match outcome {
        //
        // Internal implementation detail.
        Ok(Some(true)) => {
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

        Ok(Some(false)) | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Ok(None) => TaskFlow::Wait {
            err_message: "page objects are pending".into(),
        },

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
    }
}
