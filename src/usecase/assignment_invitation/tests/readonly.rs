//! Read-only lifecycle tests for assignment invitations.

use super::*;

#[tokio::test]
async fn create_rejects_published_chapter() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    {
        let mut state = mock.state.lock().unwrap();

        state.chapters[0].stages = state.chapters[0]
            .stages
            .try_set_phase(Stage::Publish, StagePhase::Completed)
            .unwrap();
    }

    let result = create(
        (&mock, &mock, &mock),
        token("admin-user"),
        create_data("target-qid"),
    )
    .await;

    assert!(matches!(result, Err(BaseError::Expected { .. })));

    assert!(mock.snapshot().assignment_invitations.is_empty());
}

#[tokio::test]
async fn delete_rejects_published_chapter() {
    //
    let mock = Mock::new();

    seed_published_scope(&mock);

    seed_admin(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err =
        delete((&mock, &mock), token("admin-user"), "invitation-1".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(mock.snapshot().assignment_invitations.len(), 1);
}
