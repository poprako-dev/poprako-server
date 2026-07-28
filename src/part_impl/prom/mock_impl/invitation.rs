use poprako_orchestra::OperRun as _;

use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::BaseRest;

/// Process a [`InvitationPayload`] event by dispatching to the matching repo run.
pub async fn process(mock: &Mock, event: &InvitationPayload) -> BaseRest<()> {
    match event {
        //
        InvitationPayload::Assignment { invitation_id } => {
            PurgeExpiredAssignmentInvitation { id: invitation_id }
                .run_on(mock)
                .await
        }

        InvitationPayload::Member { invitation_id } => {
            PurgeExpiredMemberInvitation { id: invitation_id }
                .run_on(mock)
                .await
        }
    }
}
