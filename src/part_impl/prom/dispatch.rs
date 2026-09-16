//! Delivery mapping from persisted Prom tasks to domain use cases.

use poprako_orchestra::{Context, Nucl};

use poprako_obj_dept::ObjDeptView;

use crate::part::effect::Develop;
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::result::{BaseError, BaseRest};
use crate::usecase::chapter::stage as chapter_stage_usecase;
use crate::usecase::{
    assignment_invitation as assignment_invitation_usecase,
    member_invitation as member_invitation_usecase,
};

/// Delivers one decoded Prom task to its domain use case.
pub async fn dispatch<C, N, R, V, D>(
    (nucl, repo, obj_dept_view, develop): (&N, &R, &V, &D),
    task: TaskPayload,
) -> TaskFlow
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: AssignmentInvitationRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + MemberInvitationRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    V: ObjDeptView<PageImage, C> + Sync,
    D: Develop + Sync,
{
    match task {
        //
        TaskPayload::Chapter { payload } => match payload {
            //
            ChapterPayload::TryAdvanceRawProvideStage {
                chapter_id,
                actor_user_id,
            } => {
                //
                let rest = chapter_stage_usecase::try_advance_raw_provide(
                    (nucl, repo, obj_dept_view, develop),
                    &chapter_id,
                    Some(actor_user_id),
                )
                .await;

                chapter_flow(rest)
            }
        },

        TaskPayload::Invitation { payload } => {
            //
            let rest = match payload {
                //
                InvitationPayload::PurgeExpiredAssignmentInvitation {
                    invitation_id,
                } => {
                    //
                    assignment_invitation_usecase::purge_expired::<C, R>(
                        (repo,),
                        &invitation_id,
                    )
                    .await
                }

                InvitationPayload::PurgeExpiredMemberInvitation {
                    invitation_id,
                } => {
                    //
                    member_invitation_usecase::purge_expired::<C, R>(
                        (repo,),
                        &invitation_id,
                    )
                    .await
                }
            };

            retry_flow(rest)
        }
    }
}

// Map chapter advancement outcomes to Prom delivery policy.
fn chapter_flow(
    rest: BaseRest<chapter_stage_usecase::RawProvideAdvance>,
) -> TaskFlow {
    //
    match rest {
        //
        Ok(
            chapter_stage_usecase::RawProvideAdvance::Advanced
            | chapter_stage_usecase::RawProvideAdvance::Unchanged,
        )
        | Err(BaseError::Expected { .. }) => TaskFlow::Complete,

        Ok(chapter_stage_usecase::RawProvideAdvance::Pending) => {
            //
            TaskFlow::Wait {
                err_message: "page objects are pending".into(),
            }
        }

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
    }
}

// Map generic task outcomes to retry or completion policy.
fn retry_flow(rest: BaseRest<()>) -> TaskFlow {
    //
    match rest {
        //
        Ok(()) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry {
            err_message: format!("{:?}", error),
        },
    }
}
