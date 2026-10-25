use super::*;

#[tokio::test]
async fn update_roles_admin_updates_member_role_mask() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let update_member_role = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member--79630565486"),
    )
    .await;

    assert!(update_member_role.is_ok());

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member--79630565486")
        .unwrap();

    assert_eq!(member_info.roles, RoleMask::from(RoleField::REVIEWER));
}

#[tokio::test]
async fn update_roles_one_of_two_admins_removes_own_admin_role() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member--24859484410",
        "other-admin-user",
        "Other Admin",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    ));

    let update_member_role = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member--66825055253"),
    )
    .await;

    assert!(update_member_role.is_ok());

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == "member--66825055253")
        .unwrap();

    assert_eq!(member_info.roles, RoleMask::from(RoleField::REVIEWER));
}

#[tokio::test]
async fn update_roles_last_admin_role_is_retained() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let error = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member--66825055253"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert_eq!(
        mock.snapshot().members.first().unwrap().roles,
        RoleMask::from(RoleField::ADMIN)
    );
}

#[tokio::test]
async fn update_roles_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member--35634922393",
        "normal-user",
        "Normal",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let error = update_roles(
        (&mock, &mock),
        token("normal-user"),
        update_role_instr("member--79630565486"),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member--79630565486")
        .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert_eq!(member_info.roles, RoleMask::from(RoleField::TRANSLATOR));
}

#[tokio::test]
async fn update_roles_missing_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let error = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_role_instr("member--66314807403"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn delete_admin_deletes_member() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let delete_member = delete(
        (&mock, &mock),
        token("admin-user"),
        "member--79630565486".into(),
    )
    .await;

    assert!(delete_member.is_ok());

    assert!(
        !mock
            .snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member--79630565486")
    );
}

#[tokio::test]
async fn delete_one_of_two_admins_deletes_own_member() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member--24859484410",
        "other-admin-user",
        "Other Admin",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    ));

    let delete_member = delete(
        (&mock, &mock),
        token("admin-user"),
        "member--66825055253".into(),
    )
    .await;

    assert!(delete_member.is_ok());

    assert!(
        !mock
            .snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member--66825055253")
    );
}

#[tokio::test]
async fn delete_last_admin_member_is_retained() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let error = delete(
        (&mock, &mock),
        token("admin-user"),
        "member--66825055253".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert!(
        mock.snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member--66825055253")
    );
}

#[tokio::test]
async fn delete_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member--35634922393",
        "normal-user",
        "Normal",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let error = delete(
        (&mock, &mock),
        token("normal-user"),
        "member--79630565486".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert!(
        mock.snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member--79630565486")
    );
}

#[tokio::test]
async fn delete_missing_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let error = delete(
        (&mock, &mock),
        token("admin-user"),
        "member--66314807403".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert_eq!(mock.snapshot().members.len(), 1);
}
