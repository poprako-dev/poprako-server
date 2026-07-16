// member_invitation_roundtrip_reads_test_database_url(MemberInvitationRepo)(positive): member invitation repo creates, lists, and marks invitations used in the local test database.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationListKind, MemberInvitationListSpec,
};
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, ListMemberInvitationInfos, UpdateMemberInvitation,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::BaseError;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-member-invitation-domain-";

#[tokio::test]
async fn member_invitation_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let nucl = RdbDrive::new(shared.clone());

    let member_invitation_entry = MemberInvitationEntry {
        id: format!("{}member-invitation", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        invitor_id: team_fixture.user_entry.id.clone(),
        invitee_qid: format!("{}invitee", PREFIX),
        code: format!("{}code", PREFIX),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateMemberInvitation {
                entry: &member_invitation_entry,
            },
        )
        .await?;

        repo.step(
            context,
            &UpdateMemberInvitation::MarkUsed {
                id: &member_invitation_entry.id,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let member_invitation_list_spec = MemberInvitationListSpec {
        team_id: team_fixture.team_entry.id.clone(),
        kind: MemberInvitationListKind::Used,
        incl_opt: vec![MemberInvitationInclOpt::Invitor],
        offset: 0,
        limit: 10,
    };

    let member_invitation_infos = repo
        .run(&ListMemberInvitationInfos {
            spec: &member_invitation_list_spec,
        })
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
