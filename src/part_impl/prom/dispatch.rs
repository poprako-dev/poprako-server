//! Delivery mapping from persisted Prom tasks to domain use cases.

use poprako_orchestra::{Context, Nucl};

use poprako_obj_dept::ObjDeptView;
use poprako_prom::general::dispatch_flow::DispatchFlow;

use crate::part::effect::EventBus;
use crate::part::obj_dept::PageSourceImage;
use crate::part::prom::payload::PromPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page_source::PageSourceRepo;
use crate::result::{BaseError, BaseRest};
use crate::usecase::chapter::stage as chapter_stage_usecase;
use crate::usecase::{
    assignment_invitation as assignment_invitation_usecase,
    member_invitation as member_invitation_usecase,
};

/// Delivers one decoded Prom task to its domain use case.
pub async fn dispatch<C, N, R, V, B>(
    (nucl, repo, obj_dept_view, bus): (&N, &R, &V, &B),
    task: PromPayload,
) -> DispatchFlow
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: AssignmentInvitationRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + MemberInvitationRepo<C>
        + PageSourceRepo<C>
        + Send
        + Sync,
    V: ObjDeptView<PageSourceImage, C> + Sync,
    B: EventBus + Sync,
{
    match task {
        //
        PromPayload::Chapter { payload } => match payload {
            //
            ChapterPayload::TryAdvanceRawProvideStage {
                chapter_id,
                actor_user_id,
            } => {
                //
                let rest = chapter_stage_usecase::try_advance_raw_provide(
                    (nucl, repo, obj_dept_view, bus),
                    &chapter_id,
                    Some(actor_user_id),
                )
                .await;

                chapter_flow(rest)
            }
        },

        PromPayload::Invitation { payload } => {
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
) -> DispatchFlow {
    //
    match rest {
        //
        Ok(
            chapter_stage_usecase::RawProvideAdvance::Advanced
            | chapter_stage_usecase::RawProvideAdvance::Unchanged,
        )
        | Err(BaseError::Expected { .. }) => DispatchFlow::Complete,

        Ok(chapter_stage_usecase::RawProvideAdvance::Pending) => {
            DispatchFlow::Wait {
                error_message: "page objects are pending".into(),
            }
        }

        Err(error) => DispatchFlow::Retry {
            error_message: format!("{:?}", &error),
        },
    }
}

// Map generic task outcomes to retry or completion policy.
fn retry_flow(rest: BaseRest<()>) -> DispatchFlow {
    //
    match rest {
        //
        Ok(()) => DispatchFlow::Complete,

        Err(error) => DispatchFlow::Retry {
            error_message: format!("{:?}", &error),
        },
    }
}
