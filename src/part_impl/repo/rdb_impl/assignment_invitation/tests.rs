// assignment_invitation_roundtrip_uses_testcontainer(CreateAssignmentInvitation, ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed)(positive): assignment invitation repo creates, lists, and marks invitations used in an isolated PostgreSQL container.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationListKind,
    AssignmentInvitationListSpec,
};
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, ListAssignmentInvitationInfos,
    MarkAssignmentInvitationUsed,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::BaseError;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-invitation-domain-";

/// Verifies assignment invitation roundtrip via testcontainers.
/// Verifies assignment invitation roundtrip via testcontainers.
/// Verifies assignment invitation roundtrip via testcontainers.
pub async fn assignment_invitation_roundtrip_uses_testcontainer(
    shared: RdbCore,
) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let assignment_invitation_entry = AssignmentInvitationEntry {
        id: format!("{}assignment-invitation", PREFIX),
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        inviter_id: chapter_fixture.creator_form.id.clone(),
        invitee_qid: format!("{}invitee", PREFIX),
        code: format!("{}code", PREFIX),
        roles: RoleMask::from(RoleField::REVIEWER),
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreateAssignmentInvitation {
                    entry: &assignment_invitation_entry,
                },
            )
            .await?;

            repo.step(
                context,
                &MarkAssignmentInvitationUsed {
                    id: &assignment_invitation_entry.id,
                },
            )
            .await?;

            Ok::<(), BaseError>(())
        })
        .await
        .ok()
        .unwrap();

    let assignment_invitation_list_spec = AssignmentInvitationListSpec {
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        kind: AssignmentInvitationListKind::Used,
        offset: 0,
        limit: 10,
    };

    let assignment_invitation_infos = repo
        .run(&ListAssignmentInvitationInfos {
            spec: &assignment_invitation_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(assignment_invitation_infos.len(), 1);

    assert!(!assignment_invitation_infos[0].pending);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
