use poprako_orchestra::Run as _;

use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::BaseResult;

/// Process a [`PurgeExpiredInvitation`] event by dispatching to the matching repo run.
pub async fn process(
    mock: &Mock,
    event: &PurgeExpiredInvitation,
) -> BaseResult<()> {
    //
    match event {
        //
        PurgeExpiredInvitation::Assignment { invitation_id } => {
            mock.run(&PurgeExpiredAssignmentInvitation { id: invitation_id })
                .await
        }

        PurgeExpiredInvitation::Member { invitation_id } => {
            mock.run(&PurgeExpiredMemberInvitation { id: invitation_id })
                .await
        }
    }
}
