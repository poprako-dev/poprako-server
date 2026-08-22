use super::*;

use crate::test_util;

// Builds one team member with a deterministic role mask.
fn member(id: &str, role: RoleField) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: format!("user-{}", id),
        user_nickname: id.into(),
        user_last_active_at: test_util::now(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: RoleMask::from(role),
    }
}

#[test]
fn role_update_rejects_removing_the_only_admin() {
    //
    let admin_member_info = member("admin", RoleField::ADMIN);

    let member_infos = vec![
        admin_member_info.clone(),
        member("worker", RoleField::TRANSLATOR),
    ];

    assert!(!MemberComplex::team_has_admin_after_role_update(
        &member_infos,
        &admin_member_info,
        RoleMask::from(RoleField::REVIEWER),
    ));
}

#[test]
fn role_update_allows_removing_one_of_two_admins() {
    //
    let subject_member_info = member("subject", RoleField::ADMIN);

    let member_infos = vec![
        subject_member_info.clone(),
        member("remaining", RoleField::ADMIN),
    ];

    assert!(MemberComplex::team_has_admin_after_role_update(
        &member_infos,
        &subject_member_info,
        RoleMask::from(RoleField::TRANSLATOR),
    ));
}

#[test]
fn role_update_allows_assigning_an_admin_to_an_adminless_team() {
    //
    let subject_member_info = member("subject", RoleField::TRANSLATOR);

    let member_infos = vec![
        subject_member_info.clone(),
        member("worker", RoleField::REVIEWER),
    ];

    assert!(MemberComplex::team_has_admin_after_role_update(
        &member_infos,
        &subject_member_info,
        RoleMask::from(RoleField::ADMIN),
    ));
}

#[test]
fn deletion_requires_another_admin() {
    //
    let subject_member_info = member("subject", RoleField::ADMIN);

    let sole_admin_member_infos = vec![
        subject_member_info.clone(),
        member("worker", RoleField::TRANSLATOR),
    ];

    assert!(!MemberComplex::team_has_admin_after_delete(
        &sole_admin_member_infos,
        &subject_member_info,
    ));

    let remaining_admin_member_infos = vec![
        subject_member_info.clone(),
        member("remaining", RoleField::ADMIN),
    ];

    assert!(MemberComplex::team_has_admin_after_delete(
        &remaining_admin_member_infos,
        &subject_member_info,
    ));
}
