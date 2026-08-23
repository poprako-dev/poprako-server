use super::*;

#[tokio::test]
async fn update_roles_admin_updates_member_role_mask() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let update_member_role = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member-target"),
    )
    .await;

    assert!(update_member_role.is_ok());

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member-target")
        .unwrap();

    assert_eq!(member_info.roles, RoleMask::from(RoleField::REVIEWER));
}

#[tokio::test]
async fn update_roles_one_of_two_admins_removes_own_admin_role() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-other-admin",
        "other-admin-user",
        "Other Admin",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let update_member_role = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member-admin"),
    )
    .await;

    assert!(update_member_role.is_ok());

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == "member-admin")
        .unwrap();

    assert_eq!(member_info.roles, RoleMask::from(RoleField::REVIEWER));
}

#[tokio::test]
async fn update_roles_last_admin_role_is_retained() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member-admin"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(
        mock.snapshot().members[0].roles,
        RoleMask::from(RoleField::ADMIN)
    );
}

#[tokio::test]
async fn update_roles_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err = update_roles(
        (&mock, &mock),
        token("normal-user"),
        update_role_instr("member-target"),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member-target")
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(member_info.roles, RoleMask::from(RoleField::TRANSLATOR));
}

#[tokio::test]
async fn update_roles_missing_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member-missing"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn delete_admin_deletes_member() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let delete_member =
        delete((&mock, &mock), token("admin-user"), "member-target".into())
            .await;

    assert!(delete_member.is_ok());

    assert!(
        !mock
            .snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-target")
    );
}

#[tokio::test]
async fn delete_one_of_two_admins_deletes_own_member() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-other-admin",
        "other-admin-user",
        "Other Admin",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let delete_member =
        delete((&mock, &mock), token("admin-user"), "member-admin".into())
            .await;

    assert!(delete_member.is_ok());

    assert!(
        !mock
            .snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-admin")
    );
}

#[tokio::test]
async fn delete_last_admin_member_is_retained() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err =
        delete((&mock, &mock), token("admin-user"), "member-admin".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(
        mock.snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-admin")
    );
}

#[tokio::test]
async fn delete_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err =
        delete((&mock, &mock), token("normal-user"), "member-target".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(
        mock.snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-target")
    );
}

#[tokio::test]
async fn delete_missing_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err =
        delete((&mock, &mock), token("admin-user"), "member-missing".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(mock.snapshot().members.len(), 1);
}
