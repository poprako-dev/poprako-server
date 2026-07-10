// member_invitation_roundtrip_reads_test_database_url(MemberInvitationStep)(positive): member invitation repo creates, lists, and marks invitations used in the local test database.

use super::*;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::member_invitation::{
    MemberInvitationForm, MemberInvitationListSpec,
};
use crate::part::repo::step::member_invitation::MemberInvitationStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-member-invitation-domain-";

#[tokio::test]
async fn member_invitation_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let member_invitation_form = MemberInvitationForm {
        id: format!("{}member-invitation", PREFIX),
        team_id: team_fixture.team_form.id.clone(),
        invitor_id: team_fixture.user_form.id.clone(),
        invitee_qid: format!("{}invitee", PREFIX),
        code: format!("{}code", PREFIX),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &MemberInvitationStep::create(&member_invitation_form),
            )
            .await?;

            Advance::advance(
                &transactional_repo,
                context,
                &MemberInvitationStep::mark_pending_as_used(
                    &member_invitation_form.id,
                ),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let member_invitation_list_spec = MemberInvitationListSpec {
        team_id: team_fixture.team_form.id.clone(),
        pending: Some(false),
        incl_opt: vec![MemberInvitationInclOpt::Invitor],
        offset: 0,
        limit: 10,
    };

    let member_invitation_infos = Execute::execute(
        &repo,
        &MemberInvitationStep::list_infos(&member_invitation_list_spec),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(member_invitation_infos.len(), 1);
    assert!(!member_invitation_infos[0].pending);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
