#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// assignment_invitation_roundtrip_uses_testcontainer(CreateAssignmentInvitation, ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed)(positive): assignment invitation repo creates, lists, and marks invitations used in an isolated PostgreSQL container.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, ListAssignmentInvitationInfos,
    MarkAssignmentInvitationUsed,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-invitation-domain-";

/// Verifies assignment invitation roundtrip via testcontainers.
/// Verifies assignment invitation roundtrip via testcontainers.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn assignment_invitation_roundtrip_uses_testcontainer(
    shared: RdbCore,
) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let assignment_invitation_entry = AssignmentInvitationEntry {
        id: format!("{}assignment-invitation", PREFIX),
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        inviter_id: chapter_fixture.creator_form.id.clone(),
        invitee_qid: format!("{}invitee", PREFIX),
        code: format!("{}code", PREFIX),
        roles: RoleMask::from(RoleField::REVIEWER),
    };

    nucl.coord(async |context| {
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
    .unwrap();

    let assignment_invitation_list_spec = AssignmentInvitationListSpec {
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        is_pending: Some(false),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let assignment_invitation_infos = repo
        .run(&ListAssignmentInvitationInfos {
            spec: &assignment_invitation_list_spec,
        })
        .await
        .unwrap();

    assert_eq!(assignment_invitation_infos.len(), 1);

    assert!(
        !assignment_invitation_infos
            .as_slice()
            .first()
            .unwrap()
            .is_pending
    );

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}
