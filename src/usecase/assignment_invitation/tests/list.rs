//! Assignment invitation listing tests.

use super::*;

#[tokio::test]
async fn list_infos_non_reviewer_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = list_infos((&mock,), token("normal-user"), list_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}
