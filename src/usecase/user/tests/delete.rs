use super::*;

use poprako_obj_dept::model::task::ObjTask;

fn seed_user_with_avatar(mock: &Mock, is_avail: bool) {
    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    seed_user_avatar(mock, "user-1", 2);

    mock.state
        .lock()
        .unwrap()
        .objs
        .get_mut("user_avatar")
        .unwrap()
        .get_mut("user-1")
        .unwrap()
        .meta
        .as_mut()
        .unwrap()
        .is_avail = is_avail;
}

#[tokio::test]
async fn delete_removes_user_credentials_members_and_enqueues_avatar_delete() {
    //
    let mock = Mock::new();

    seed_user_with_avatar(&mock, true);

    mock.seed_member(member("member-1", "user-1", "Nick", "team-1"));

    mock.seed_member(member("member-2", "user-1", "Nick", "team-2"));

    mock.seed_member(member(
        "member-team-1-admin",
        "team-1-admin",
        "Team 1 Admin",
        "team-1",
    ));

    mock.seed_member(member(
        "member-team-2-admin",
        "team-2-admin",
        "Team 2 Admin",
        "team-2",
    ));

    crate::usecase::user::delete::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());

    assert!(snapshot.credentials.is_empty());

    assert_eq!(snapshot.members.len(), 2);

    assert!(
        snapshot
            .members
            .iter()
            .all(|member_info| member_info.user_id != "user-1")
    );

    assert!(snapshot.objs["user_avatar"].is_empty());

    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.id == "user-1" && key.ver == 2)
    }));
}

#[tokio::test]
async fn delete_rejects_last_admin_membership_and_rolls_back_every_team() {
    //
    let mock = Mock::new();

    seed_user_with_avatar(&mock, true);

    mock.seed_member(member("member-1", "user-1", "Nick", "team-1"));

    mock.seed_member(member("member-2", "user-1", "Nick", "team-2"));

    mock.seed_member(member(
        "member-team-1-admin",
        "team-1-admin",
        "Team 1 Admin",
        "team-1",
    ));

    let err = crate::usecase::user::delete::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.credentials.len(), 1);

    assert_eq!(snapshot.members.len(), 3);

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.objs["user_avatar"].contains_key("user-1"));
}

#[tokio::test]
async fn delete_pending_avatar_enqueues_exact_object_delete() {
    //
    let mock = Mock::new();

    seed_user_with_avatar(&mock, false);

    crate::usecase::user::delete::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
    )
    .await
    .unwrap();

    assert!(mock.snapshot().obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.id == "user-1" && key.ver == 2)
    }));
}

#[tokio::test]
async fn delete_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let err = crate::usecase::user::delete::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user-2"),
        "user-1".into(),
    )
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

    let err = crate::usecase::user::delete::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}
