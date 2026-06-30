// delete(delete)(positive): owner should delete own assignment.
// delete(delete)(positive): reviewer should delete another user's assignment.
// delete(delete)(negative): non-reviewer should not delete another user's assignment.

use super::*;

use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

#[tokio::test]
async fn delete_owner_deletes_own_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));

    let result = delete(
        &mock,
        &mock,
        token("worker-user"),
        "assignment-chapter-1-worker-user".into(),
    )
    .await;

    assert!(result.is_ok());
    assert!(mock.snapshot().assignments.is_empty());
}

#[tokio::test]
async fn delete_reviewer_deletes_another_user_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        role(RoleField::REVIEWER),
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::TRANSLATOR),
    ));

    let result = delete(
        &mock,
        &mock,
        token("reviewer-user"),
        "assignment-chapter-1-target-user".into(),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(mock.snapshot().assignments.len(), 1);
}

#[tokio::test]
async fn delete_non_reviewer_does_not_delete_another_user_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "worker-user",
        role(RoleField::TRANSLATOR),
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::PROOFREADER),
    ));

    let err = delete(
        &mock,
        &mock,
        token("worker-user"),
        "assignment-chapter-1-target-user".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
    assert_eq!(mock.snapshot().assignments.len(), 2);
}
