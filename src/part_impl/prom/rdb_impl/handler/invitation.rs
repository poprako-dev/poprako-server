//! Handler for expired invitation purge events.

use tracing::instrument;

use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::shared::RdbContext;

/// Purges an expired invitation when it is still pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<R>(repo: &R, event: &PurgeExpiredInvitation) -> TaskFlow
where
    R: AssignmentInvitationRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + Send
        + Sync,
{
    let outcome = match event {
        //
        PurgeExpiredInvitation::Assignment { invitation_id } => {
            repo.run(&PurgeExpiredAssignmentInvitation { id: invitation_id })
                .await
        }

        PurgeExpiredInvitation::Member { invitation_id } => {
            repo.run(&PurgeExpiredMemberInvitation { id: invitation_id })
                .await
        }
    };

    match outcome {
        //
        Ok(()) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}
