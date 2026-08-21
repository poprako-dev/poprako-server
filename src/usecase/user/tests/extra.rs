use super::{
    ExpectedVariant, IMAGE_CONFIG, Mock, assert_expected_message, credential,
    mark_avatar_uploaded, mark_instr, reserve_avatar, reserve_instr, token,
    user_with_avatar,
};

#[tokio::test]
async fn mark_avatar_uploaded_rejects_old_reservation_replay() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "old-key", true, 1),
        credential("user-1", "password"),
    );

    let reserved = reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        reserve_instr("png"),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 2);

    let err = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
        mark_instr(1),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-avatar-upload",
    );

    assert_ne!(snapshot.users[0].is_avatar_uploaded, Some(true));

    assert_eq!(snapshot.users[0].avatar_version, Some(2));
}
