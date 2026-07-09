// member_roundtrip_reads_test_database_url(MemberStep)(positive): member repo creates, lists, fetches, and updates roles in the local test database.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::member::{MemberForm, MemberListSpec, MemberRoleUpdate};
use crate::part::repo::step::member::MemberStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-member-domain-";

#[tokio::test]
async fn member_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let admin_role = RoleMask::from(RoleField::ADMIN);

    let member_role = RoleMask::from(RoleField::TRANSLATOR);

    let member_form = MemberForm {
        id: format!("{}member", PREFIX),
        user_id: team_fixture.user_form.id.clone(),
        user_nickname: team_fixture.user_form.nickname.clone(),
        team_id: team_fixture.team_form.id.clone(),
        roles: admin_role,
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &MemberStep::create(&member_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let member_list_spec = MemberListSpec::Team {
        team_id: team_fixture.team_form.id.clone(),
        fuzzy_nickname: Some("RDB".into()),
        role: Some(RoleField::ADMIN),
        incl_opt: vec![MemberInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let member_infos =
        Execute::execute(&repo, &MemberStep::list_infos(&member_list_spec))
            .await
            .ok()
            .unwrap();

    assert_eq!(member_infos.len(), 1);
    assert_eq!(
        member_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_form.id
    );

    let member_role_update = MemberRoleUpdate {
        id: member_form.id.clone(),
        roles: member_role,
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &MemberStep::update_role(&member_role_update),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let member_info = Execute::execute(
        &repo,
        &MemberStep::get_info_by_id(&member_form.id, &[MemberInclOpt::User]),
    )
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
