// join(join)(positive): user creates a new assignment when joining with assignable roles.
// join(join)(positive): existing assignment role union preserves earlier role timestamps.
// join(join)(negative): user cannot join with roles outside team membership.

use super::*;

use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

#[tokio::test]
async fn join_creates_assignment() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", role(RoleField::TRANSLATOR)));

    let joined = join(
        &mock,
        &mock,
        token("user-1"),
        assignment_data::JoinChapterData {
            chapter_id: "chapter-1".into(),
            roles: role(RoleField::TRANSLATOR),
        },
    )
    .await;

    assert!(joined.is_ok());

    let joined = joined.ok().unwrap();

    assert_eq!(joined.chapter_id, "chapter-1");

    assert_eq!(mock.snapshot().assignments.len(), 1);
}

#[tokio::test]
async fn join_unions_existing_assignment_roles() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member(
        "user-1",
        roles(RoleField::TRANSLATOR, RoleField::PROOFREADER),
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        role(RoleField::TRANSLATOR),
    ));

    let joined = join(
        &mock,
        &mock,
        token("user-1"),
        assignment_data::JoinChapterData {
            chapter_id: "chapter-1".into(),
            roles: role(RoleField::PROOFREADER),
        },
    )
    .await;

    assert!(joined.is_ok());

    let snapshot = mock.snapshot();

    assert!(
        snapshot.assignments[0]
            .roles
            .has_every_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    );
}

#[tokio::test]
async fn join_rejects_role_outside_member_mask() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", role(RoleField::TRANSLATOR)));

    let err = join(
        &mock,
        &mock,
        token("user-1"),
        assignment_data::JoinChapterData {
            chapter_id: "chapter-1".into(),
            roles: role(RoleField::PROOFREADER),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}
