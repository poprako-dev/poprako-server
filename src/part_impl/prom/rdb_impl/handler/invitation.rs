//! Handler for expired invitation purge events.

use poprako_orchestra::Nucl;
use tracing::instrument;

use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part_impl::prom::rdb_impl::handler::TaskFlow;
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult, accept};

/// Purges an expired invitation when it is still pending.
#[instrument(level = "info", skip_all)]
pub async fn handle<N, R>(
    nucl: &N,
    repo: &R,
    event: &PurgeExpiredInvitation,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + Send
        + Sync,
{
    let result = execute(nucl, repo, event).await;

    match result {
        //
        Ok(()) => TaskFlow::Complete,

        Err(error) => TaskFlow::Retry(format!("{:?}", error)),
    }
}

async fn execute<N, R>(
    nucl: &N,
    repo: &R,
    event: &PurgeExpiredInvitation,
) -> BaseResult<()>
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + Send
        + Sync,
{
    nucl.coord(async move |context| purge(repo, context, event).await)
        .await?;

    accept(())
}

async fn purge<R>(
    repo: &R,
    context: &mut RdbContext,
    event: &PurgeExpiredInvitation,
) -> BaseResult<()>
where
    R: AssignmentInvitationRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + Send
        + Sync,
{
    match event {
        //
        PurgeExpiredInvitation::Assignment { invitation_id } => {
            repo.step(
                context,
                &PurgeExpiredAssignmentInvitation { id: invitation_id },
            )
            .await
        }

        PurgeExpiredInvitation::Member { invitation_id } => {
            repo.step(
                context,
                &PurgeExpiredMemberInvitation { id: invitation_id },
            )
            .await
        }
    }
}
