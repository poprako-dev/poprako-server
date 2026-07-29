use super::*;

#[tokio::test]
async fn create_rejects_preset_role_missing_from_membership() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let mut instr = create_instr("workset-1");

    instr.preset_assignment_roles = Some(RoleMask::from(RoleField::TRANSLATOR));

    let err = create((&mock, &mock), token("user-1"), instr)
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());
}
