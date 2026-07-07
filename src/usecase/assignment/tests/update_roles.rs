// update_roles(update_roles)(positive): reviewer should create missing assignment.
// update_roles(update_roles)(positive): reviewer should overwrite existing assignment roles.
// update_roles(update_roles)(positive): self role reduction should update the assignment.
// update_roles(update_roles)(negative): self role expansion should be rejected.
// update_roles(update_roles)(negative): self role reduction should require member role.
// update_roles(update_roles)(negative): non-reviewer should not update another user.
// update_roles(update_roles)(negative): admin role should be rejected.
// update_roles(update_roles)(negative): target member role mismatch should be rejected.
// update_roles(update_roles)(negative): only chapter admin should not remove own admin role.

use super::*;

use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

#[tokio::test]
async fn update_roles_reviewer_creates_missing_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        roles(RoleField::ADMIN, RoleField::REVIEWER),
    ));
    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));

    update_roles(
        &mock,
        &mock,
        token("reviewer-user"),
        update_roles_data("chapter-1", "target-user", role(RoleField::TRANSLATOR)),
    )
    .await
    .unwrap();
    assert!(
        mock.snapshot()
            .assignments
            .iter()
            .any(|assignment_info| assignment_info.user_id == "target-user")
    );
}

#[tokio::test]
async fn update_roles_reviewer_overwrites_existing_assignment_roles() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        roles(RoleField::ADMIN, RoleField::REVIEWER),
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::TRANSLATOR),
    ));
    mock.seed_member(member("target-user", role(RoleField::PROOFREADER)));

    update_roles(
        &mock,
        &mock,
        token("reviewer-user"),
        update_roles_data("chapter-1", "target-user", role(RoleField::PROOFREADER)),
    )
    .await
    .unwrap();
    let snapshot = mock.snapshot();
    let assignment_info = snapshot
        .assignments
        .iter()
        .find(|assignment_info| assignment_info.user_id == "target-user")
        .unwrap();

    assert_eq!(assignment_info.roles, role(RoleField::PROOFREADER));
}

#[tokio::test]
async fn update_roles_self_role_reduction_updates_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));
    mock.seed_member(member(
        "worker-user",
        roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
    ));

    update_roles(
        &mock,
        &mock,
        token("worker-user"),
        update_roles_data("chapter-1", "worker-user", role(RoleField::TRANSLATOR)),
    )
    .await
    .unwrap();
    assert_eq!(
        mock.snapshot().assignments[0].roles,
        role(RoleField::TRANSLATOR)
    );
}

#[tokio::test]
async fn update_roles_self_role_expansion_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));

    let err = update_roles(
        &mock,
        &mock,
        token("worker-user"),
        update_roles_data(
            "chapter-1",
            "worker-user",
            roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
        ),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_self_role_reduction_requires_member_role() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
    ));
    mock.seed_member(member("worker-user", role(RoleField::PROOFREADER)));

    let err = update_roles(
        &mock,
        &mock,
        token("worker-user"),
        update_roles_data("chapter-1", "worker-user", role(RoleField::TRANSLATOR)),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_non_reviewer_does_not_update_another_user() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));
    mock.seed_member(member("target-user", role(RoleField::PROOFREADER)));

    let err = update_roles(
        &mock,
        &mock,
        token("worker-user"),
        update_roles_data("chapter-1", "target-user", role(RoleField::PROOFREADER)),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_admin_role_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));
    mock.seed_member(member("target-user", role(RoleField::ADMIN)));

    let err = update_roles(
        &mock,
        &mock,
        token("admin-user"),
        update_roles_data("chapter-1", "target-user", role(RoleField::ADMIN)),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_roles_target_member_role_mismatch_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));
    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));

    let err = update_roles(
        &mock,
        &mock,
        token("admin-user"),
        update_roles_data("chapter-1", "target-user", role(RoleField::PROOFREADER)),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_only_chapter_admin_does_not_remove_own_admin_role() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        roles(RoleField::ADMIN, RoleField::TRANSLATOR),
    ));
    mock.seed_member(member(
        "admin-user",
        roles(RoleField::ADMIN, RoleField::TRANSLATOR),
    ));

    let err = update_roles(
        &mock,
        &mock,
        token("admin-user"),
        update_roles_data("chapter-1", "admin-user", role(RoleField::TRANSLATOR)),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}
