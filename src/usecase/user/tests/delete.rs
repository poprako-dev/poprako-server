use super::*;

#[tokio::test]
async fn delete_removes_user_credentials_members_and_enqueues_avatar_delete() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", true, 2),
        credential("user-1", "password"),
    );

    mock.seed_member(member("member-1", "user-1", "Nick", "team-1"));

    mock.seed_member(member("member-2", "user-1", "Nick", "team-2"));

    delete((&mock, &mock, &mock,), token("user-1"), "user-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());

    assert!(snapshot.credentials.is_empty());

    assert!(snapshot.members.is_empty());

    assert_eq!(
        count_delete_records(&snapshot.prom_records, "avatar-key"),
        1
    );
}

#[tokio::test]
async fn delete_without_uploaded_avatar_does_not_enqueue_prom() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", false, 2),
        credential("user-1", "password"),
    );

    delete((&mock, &mock, &mock,), token("user-1"), "user-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let err = delete((&mock, &mock, &mock,), token("user-2"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.credentials.len(), 1);
}

#[tokio::test]
async fn delete_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let err = delete((&mock, &mock, &mock,), token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}
