use poprako_orchestra::{Nucl as _, Step as _};

use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::part::repo::oper::assignment_invitation::PurgeExpiredAssignmentInvitation;
use crate::part::repo::oper::member_invitation::PurgeExpiredMemberInvitation;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseResult, accept};

pub(super) async fn process(
    mock: &Mock,
    event: &PurgeExpiredInvitation,
) -> BaseResult<()> {
    //
    mock.coord(async move |context| match event {
        //
        PurgeExpiredInvitation::Assignment { invitation_id } => {
            mock.step(
                context,
                &PurgeExpiredAssignmentInvitation { id: invitation_id },
            )
            .await
        }

        PurgeExpiredInvitation::Member { invitation_id } => {
            mock.step(
                context,
                &PurgeExpiredMemberInvitation { id: invitation_id },
            )
            .await
        }
    })
    .await?;

    accept(())
}
