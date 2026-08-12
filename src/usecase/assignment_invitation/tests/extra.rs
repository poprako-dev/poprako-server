use super::{
    Mock, assert_expected_variant, credential, invitation, join, join_data,
    member, role, seed_scope, token, user,
};

use crate::result::ExpectedVariant;
use crate::value::role::RoleField;

#[tokio::test]
async fn join_mismatched_qid_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );

    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "other-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = join((&mock, &mock, &mock), token("target-user"), join_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.assignment_invitations[0].is_pending);
}
