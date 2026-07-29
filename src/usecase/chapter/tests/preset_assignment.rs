use super::*;

#[tokio::test]
async fn create_rejects_preset_role_missing_from_membership() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let err = create(
        (&mock, &mock),
        token("user-1"),
        CreateChapterInstr {
            comic_id: "comic-1".into(),
            subtitle: None,
            preset_assignment_roles: Some(RoleMask::from(
                RoleField::TRANSLATOR,
            )),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(mock.snapshot().chapters.is_empty());

    assert!(mock.snapshot().assignments.is_empty());
}
