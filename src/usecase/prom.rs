//! Deferred local-message business use cases.

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::oper::ListObjMetas;

use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part::repo::oper::page::ListPageInfos;
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, accept};
use crate::shared::RdbContext;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Business action requested after handling one deferred local message.
pub enum PromTaskAction {
    //
    /// The business task completed and the message may be completed.
    Complete,

    /// The business task failed transiently and should consume retry budget.
    Retry {
        /// Diagnostic message retained for the next attempt.
        message: String,
    },

    /// The business task is waiting without consuming retry budget.
    Wait {
        /// Diagnostic message retained for the next attempt.
        message: String,
    },
}

/// Handles one deferred chapter workflow task.
#[instrument(level = "info", skip_all)]
pub async fn handle_chapter<N, R, V, D>(
    (nucl, repo, obj_view, develop): (&N, &R, &V, &D),
    task: &ChapterPayload,
) -> PromTaskAction
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    V: ObjDeptView<PageImage, RdbContext> + Sync,
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
                obj_view,
                develop,
                chapter_id,
                actor_user_id.clone(),
            )
            .await
        }
    }
}

/// Handles one deferred invitation task without an outer transaction.
#[instrument(level = "info", skip_all)]
pub async fn handle_invitation<R>(
    (repo,): (&R,),
    task: &InvitationPayload,
) -> PromTaskAction
where
    R: AssignmentInvitationRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + Send
        + Sync,
{
    let rest = match task {
        //
        InvitationPayload::Assignment { invitation_id } => {
            //
            PurgeExpiredAssignmentInvitation { id: invitation_id }
                .run_on(repo)
                .await
        }

        InvitationPayload::Member { invitation_id } => {
            //
            PurgeExpiredMemberInvitation { id: invitation_id }
                .run_on(repo)
                .await
        }
    };

    match rest {
        //
        Ok(()) => PromTaskAction::Complete,

        Err(error) => PromTaskAction::Retry {
            message: format!("{:?}", error),
        },
    }
}

// Attempts raw-provision completion and waits while uploads remain pending.
async fn handle_raw_provide<N, R, V, D>(
    nucl: &N,
    repo: &R,
    obj_view: &V,
    develop: &D,
    chapter_id: &str,
    actor_user_id: Option<String>,
) -> PromTaskAction
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    R: ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync,
    V: ObjDeptView<PageImage, RdbContext> + Sync,
    D: Develop + Sync,
{
    let rest = nucl
        .coord(async move |context| {
            //
            GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let page_infos =
                ListPageInfos { chapter_id }.step_on(repo, context).await?;

            let page_ids = page_infos
                .iter()
                .map(|page_info| page_info.id.clone())
                .collect::<Vec<_>>();

            let obj_metas = ListObjMetas::<PageImage>::new(&page_ids)
                .step_on(obj_view, context)
                .await
                .map_err(BaseError::from)?;

            let are_images_uploaded = page_infos.iter().all(|page_info| {
                //
                obj_metas
                    .get(&page_info.id)
                    .is_some_and(|obj_meta| obj_meta.is_avail)
            });

            if !are_images_uploaded {
                return accept(None);
            }

            let is_advanced = CompleteChapterRawProvide { id: chapter_id }
                .step_on(repo, context)
                .await?;

            if is_advanced {
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

            accept(Some(is_advanced))
        })
        .await
        .map_err(Into::into);

    match rest {
        //
        Ok(Some(true)) => {
            //
            Event::ChapterWorkflowCompleted {
                payload: ChapterWorkflowCompletedEvent {
                    chapter_id: chapter_id.to_string(),
                    completed_stage: Stage::RawProvide,
                },
            }
            .develop_on(develop)
            .await;

            PromTaskAction::Complete
        }

        Ok(Some(false)) | Err(BaseError::Expected { .. }) => {
            PromTaskAction::Complete
        }

        Ok(None) => PromTaskAction::Wait {
            message: "page objects are pending".into(),
        },

        Err(error) => PromTaskAction::Retry {
            message: format!("{:?}", error),
        },
    }
}
