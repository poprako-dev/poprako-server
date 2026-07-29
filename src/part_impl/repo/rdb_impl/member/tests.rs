// member_roundtrip_uses_testcontainer(MemberRepo)(positive): member repo creates, lists, fetches, and updates roles in an isolated PostgreSQL container.

use super::*;

use poprako_orchestra::{Nucl as _, Step as _};

use crate::model::member::{MemberEntry, MemberListSpec, MemberRoleUpdate};
use crate::part::repo::oper::member::{
    CreateMember, GetMemberInfo, ListMemberInfos, UpdateMember,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::BaseError;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-member-domain-";

pub async fn member_roundtrip_uses_testcontainer(shared: RdbCore) {
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let nucl = RdbDrive::new(shared.clone());

    let admin_role = RoleMask::from(RoleField::ADMIN);

    let member_role = RoleMask::from(RoleField::TRANSLATOR);

    let member_entry = MemberEntry {
        id: format!("{}member", PREFIX),
        user_id: team_fixture.user_entry.id.clone(),
        user_nickname: team_fixture.user_entry.nickname.clone(),
        team_id: team_fixture.team_entry.id.clone(),
        roles: admin_role,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateMember {
                entry: &member_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let member_list_spec = MemberListSpec::Team {
        team_id: team_fixture.team_entry.id.clone(),
        fuzzy_nickname: Some("RDB".into()),
        role: Some(RoleField::ADMIN),
        incl_opt: vec![MemberInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let member_infos = repo
        .run(&ListMemberInfos::Spec {
            spec: &member_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(member_infos.len(), 1);

    assert_eq!(
        member_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_entry.id
    );

    let member_role_update = MemberRoleUpdate {
        id: member_entry.id.clone(),
        roles: member_role,
    };

    nucl.coord(async |context| {
        //

        repo.step(
            context,
            &UpdateMember::Role {
                update: &member_role_update,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let member_info = repo
        .run(&GetMemberInfo::Id {
            id: &member_entry.id,
            incls: &[MemberInclOpt::User],
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(member_info.roles, member_role);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
