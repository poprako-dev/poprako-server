// update_roles(update_roles)(positive): reviewer should create missing assignment.
// update_roles(update_roles)(positive): reviewer should overwrite existing assignment roles.
// update_roles(update_roles)(positive): self role reduction should update the assignment.
// update_roles(update_roles)(negative): self role expansion should be rejected.
// update_roles(update_roles)(negative): self role reduction should require member role.
// update_roles(update_roles)(negative): non-reviewer should not update another user.
// update_roles(update_roles)(negative): admin role should be rejected.
// update_roles(update_roles)(negative): target member role mismatch should be rejected.
// update_roles(update_roles)(negative): only chapter admin should not remove own admin role.
// update_roles(update_roles)(positive): chapter admins can leave worker roles while retaining admin.
// update_roles(update_roles)(negative): retaining admin cannot grant unsupported worker roles.
// update_roles(update_roles)(negative): existing assignments cannot gain admin through role updates.

use super::*;

use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

#[tokio::test]
async fn update_roles_reviewer_creates_missing_assignment() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        roles(RoleField::ADMIN, RoleField::REVIEWER),
    ));

    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));

    update_roles(
        (&mock, &mock),
        token("reviewer-user"),
        update_roles_data(
            "chapter-1",
            "target-user",
            role(RoleField::TRANSLATOR),
        ),
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
    //
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
        (&mock, &mock),
        token("reviewer-user"),
        update_roles_data(
            "chapter-1",
            "target-user",
            role(RoleField::PROOFREADER),
        ),
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
    //
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
        (&mock, &mock),
        token("worker-user"),
        update_roles_data(
            "chapter-1",
            "worker-user",
            role(RoleField::TRANSLATOR),
        ),
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
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));

    let err = update_roles(
        (&mock, &mock),
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
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
    ));

    mock.seed_member(member("worker-user", role(RoleField::PROOFREADER)));

    let err = update_roles(
        (&mock, &mock),
        token("worker-user"),
        update_roles_data(
            "chapter-1",
            "worker-user",
            role(RoleField::TRANSLATOR),
        ),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_non_reviewer_does_not_update_another_user() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member("target-user", role(RoleField::PROOFREADER)));

    let err = update_roles(
        (&mock, &mock),
        token("worker-user"),
        update_roles_data(
            "chapter-1",
            "target-user",
            role(RoleField::PROOFREADER),
        ),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_admin_role_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));

    mock.seed_member(member("target-user", role(RoleField::ADMIN)));

    let err = update_roles(
        (&mock, &mock),
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
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));

    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));

    let err = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_roles_data(
            "chapter-1",
            "target-user",
            role(RoleField::PROOFREADER),
        ),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_only_chapter_admin_does_not_remove_own_admin_role() {
    //
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
        (&mock, &mock),
        token("admin-user"),
        update_roles_data(
            "chapter-1",
            "admin-user",
            role(RoleField::TRANSLATOR),
        ),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_chapter_admin_can_join_and_leave_review() {
    for remaining_roles in [
        role(RoleField::ADMIN),
        roles(RoleField::ADMIN, RoleField::TRANSLATOR),
    ] {
        let mock = Mock::new();

        seed_scope(&mock);

        mock.seed_assignment(assignment(
            "chapter-1",
            "admin-user",
            remaining_roles,
        ));

        mock.seed_member(member(
            "admin-user",
            roles(RoleField::REVIEWER, RoleField::TRANSLATOR),
        ));

        let joined = join(
            (&mock, &mock),
            token("admin-user"),
            JoinChapterAssignmentInstr {
                chapter_id: "chapter-1".into(),
                roles: role(RoleField::REVIEWER),
            },
        )
        .await
        .unwrap();

        let joined_roles = remaining_roles.union(role(RoleField::REVIEWER));

        assert_eq!(joined.roles, joined_roles);

        update_roles(
            (&mock, &mock),
            token("admin-user"),
            update_roles_data("chapter-1", "admin-user", remaining_roles),
        )
        .await
        .unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.assignments.len(), 1);

        assert_eq!(snapshot.assignments[0].id, joined.id);

        assert_eq!(snapshot.assignments[0].roles, remaining_roles);

        assert_eq!(snapshot.chapter_workflow_records.len(), 2);

        let workflow_record = &snapshot.chapter_workflow_records[1];

        assert_eq!(workflow_record.chapter_id, "chapter-1");

        assert_eq!(
            workflow_record.actor_user_id.as_deref(),
            Some("admin-user")
        );

        assert!(matches!(
            &workflow_record.payload,
            ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } if subject_user_id == "admin-user"
                && *previous_roles == joined_roles
                && *next_roles == remaining_roles
        ));
    }
}

#[tokio::test]
async fn update_roles_chapter_admin_cannot_add_unsupported_worker_roles() {
    let mock = Mock::new();

    seed_scope(&mock);

    let original_roles = roles(RoleField::ADMIN, RoleField::REVIEWER);

    mock.seed_assignment(assignment("chapter-1", "admin-user", original_roles));

    mock.seed_member(member("admin-user", role(RoleField::REVIEWER)));

    let err = update_roles(
        (&mock, &mock),
        token("admin-user"),
        update_roles_data(
            "chapter-1",
            "admin-user",
            roles(RoleField::ADMIN, RoleField::PUBLISHER),
        ),
    )
    .await
    .unwrap_err();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignments[0].roles, original_roles);

    assert!(snapshot.chapter_workflow_records.is_empty());
}

#[tokio::test]
async fn update_roles_existing_assignment_cannot_gain_admin() {
    for actor_user_id in ["worker-user", "admin-user"] {
        let mock = Mock::new();

        seed_scope(&mock);

        mock.seed_assignment(assignment(
            "chapter-1",
            "admin-user",
            role(RoleField::ADMIN),
        ));

        mock.seed_assignment(assignment(
            "chapter-1",
            "worker-user",
            role(RoleField::REVIEWER),
        ));

        mock.seed_member(member(
            "worker-user",
            roles(RoleField::ADMIN, RoleField::REVIEWER),
        ));

        let err = update_roles(
            (&mock, &mock),
            token(actor_user_id),
            update_roles_data(
                "chapter-1",
                "worker-user",
                roles(RoleField::ADMIN, RoleField::REVIEWER),
            ),
        )
        .await
        .unwrap_err();

        let expected_variant = match actor_user_id {
            "worker-user" => ExpectedVariant::Perm,
            _ => ExpectedVariant::Args,
        };

        assert_expected_variant(err, expected_variant);

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.assignments[1].roles, role(RoleField::REVIEWER));

        assert!(snapshot.chapter_workflow_records.is_empty());
    }
}
