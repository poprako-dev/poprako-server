use super::*;

#[tokio::test]
async fn reserve_avatar_updates_state_enqueues_check_and_returns_put_url() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let val = reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        reserve_instr("png"),
    )
    .await
    .unwrap();

    assert_eq!(val.slot.as_ref().unwrap().image_version, 1);

    assert_eq!(
        val.slot.as_ref().unwrap().put_url,
        "https://test.local/put/team_avatar/team-1-1.png"
    );

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.teams[0].avatar_key.as_deref(),
        Some("team_avatar/team-1-1.png")
    );

    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::TeamAvatar,
        "team-1",
        "team_avatar/team-1-1.png",
        1,
    );
}

#[tokio::test]
async fn reserve_avatar_replacing_avatar_enqueues_delete_and_check() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar(
        "team-1", "Team", "Desc", "old-key", true, 1,
    ));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        reserve_instr("jpg"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(count_delete_records(&snapshot.prom_records, "old-key"), 1);

    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::TeamAvatar,
        "team-1",
        "team_avatar/team-1-2.jpg",
        2,
    );
}

#[tokio::test]
async fn reserve_avatar_rolls_back_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        reserve_instr("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.teams.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_avatar_propagates_put_url_failure_after_commit() {
    //
    let mock = Mock::new().with_image_put_failure();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        reserve_instr("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.teams[0].avatar_key.as_deref(),
        Some("team_avatar/team-1-1.png")
    );

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn reserve_avatar_rejects_byte_length_above_configured_team_limit() {
    //
    let mock = Mock::new();

    let image_config = ImageConfig {
        team_avatar_limit: 1,
        ..IMAGE_CONFIG
    };

    let instr = ReserveTeamAvatarInstr {
        new_byte_len: 1024 * 1024 + 1,
        ..reserve_instr("png")
    };

    let err = reserve_avatar(
        (&mock, &mock, &mock, &mock, &image_config),
        token("user-1"),
        "team-1".into(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().teams.is_empty());
}

#[tokio::test]
async fn mark_avatar_uploaded_marks_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar("team-1", "Team", "Desc", "key", false, 2));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "team-1".into(),
        mark_instr(2),
    )
    .await
    .unwrap();

    assert_eq!(mock.snapshot().teams[0].is_avatar_uploaded, Some(true));
}

#[tokio::test]
async fn mark_avatar_uploaded_accepts_repeated_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar("team-1", "Team", "Desc", "key", false, 2));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let first = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "team-1".into(),
        mark_instr(2),
    )
    .await;

    assert!(first.is_ok());

    let second = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "team-1".into(),
        mark_instr(2),
    )
    .await;

    assert!(second.is_ok());

    assert_eq!(mock.snapshot().teams[0].is_avatar_uploaded, Some(true));
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_stale_version() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar("team-1", "Team", "Desc", "key", false, 2));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "team-1".into(),
        mark_instr(1),
    )
    .await
    .err()
    .unwrap();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-avatar-upload",
    );

    assert_ne!(mock.snapshot().teams[0].is_avatar_uploaded, Some(true));
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_old_reservation_replay() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar(
        "team-1", "Team", "Desc", "old-key", true, 1,
    ));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let reserved = reserve_avatar(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        reserve_instr("png"),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 2);

    let err = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "team-1".into(),
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

    assert_ne!(snapshot.teams[0].is_avatar_uploaded, Some(true));

    assert_eq!(snapshot.teams[0].avatar_version, Some(2));
}

#[tokio::test]
async fn delete_removes_team_worksets_descendant_comics_and_avatar() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        true,
        2,
    ));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_workset(workset("workset-2", "team-1"));

    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover-1.png",
    ));

    mock.seed_comic(comic_with_uploaded_cover(
        "comic-2",
        "workset-2",
        "cover-2.png",
    ));

    delete((&mock, &mock, &mock), token("user-1"), "team-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.teams.is_empty());

    assert!(snapshot.worksets.is_empty());

    assert!(snapshot.comics.is_empty());

    assert_eq!(
        count_delete_records(&snapshot.prom_records, "cover-1.png"),
        1
    );

    assert_eq!(
        count_delete_records(&snapshot.prom_records, "cover-2.png"),
        1
    );

    assert_eq!(
        count_delete_records(&snapshot.prom_records, "avatar-key"),
        1
    );
}

#[tokio::test]
async fn delete_without_uploaded_avatar_does_not_enqueue_prom() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        false,
        2,
    ));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    delete((&mock, &mock, &mock), token("user-1"), "team-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_rolls_back_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = delete((&mock, &mock, &mock), token("user-1"), "team-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().teams.is_empty());

    assert!(mock.snapshot().prom_records.is_empty());
}
