use super::*;

use crate::usecase::user::delete as user_delete_usecase;
use poprako_obj_dept::model::task::ObjTask;

fn seed_user_with_avatar(mock: &Mock, is_avail: bool) {
    mock.seed_user(
        user("user--00000000001", "qid-1", "Nick"),
        credential("user--00000000001", "password"),
    );

    seed_user_avatar(mock, "user--00000000001", 2);

    mock.state
        .lock()
        .unwrap()
        .objs
        .get_mut("user_avatar")
        .unwrap()
        .get_mut("user--00000000001")
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

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "Nick",
        "team--00000000001",
    ));

    mock.seed_member(member(
        "member--00000000002",
        "user--00000000001",
        "Nick",
        "team--00000000002",
    ));

    mock.seed_member(member(
        "member--13944406377",
        "team--67193363129",
        "Team 1 Admin",
        "team--00000000001",
    ));

    mock.seed_member(member(
        "member--85994136460",
        "team--19017062342",
        "Team 2 Admin",
        "team--00000000002",
    ));

    user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "user--00000000001".into(),
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
            .all(|member_info| member_info.user_id != "user--00000000001")
    );

    assert!(snapshot.objs.get("user_avatar").unwrap().is_empty());

    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(
            task,
            ObjTask::Delete { key }
                if key.id == "user--00000000001" && key.ver == 2
        )
    }));
}

#[tokio::test]
async fn delete_removes_memberships_from_tombstoned_teams() {
    let mock = Mock::new();

    seed_user_with_avatar(&mock, true);

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "Nick",
        "team--00000000001",
    ));

    mock.state
        .lock()
        .unwrap()
        .deleted_team_ids
        .insert("team--00000000001".into());

    user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "user--00000000001".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());
    assert!(snapshot.credentials.is_empty());
    assert!(snapshot.members.is_empty());
}

#[tokio::test]
async fn delete_rejects_last_admin_membership_and_rolls_back_every_team() {
    //
    let mock = Mock::new();

    seed_user_with_avatar(&mock, true);

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "Nick",
        "team--00000000001",
    ));

    mock.seed_member(member(
        "member--00000000002",
        "user--00000000001",
        "Nick",
        "team--00000000002",
    ));

    mock.seed_member(member(
        "member--13944406377",
        "team--67193363129",
        "Team 1 Admin",
        "team--00000000001",
    ));

    let error = user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "user--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.credentials.len(), 1);

    assert_eq!(snapshot.members.len(), 3);

    assert!(snapshot.obj_tasks.is_empty());

    assert!(
        snapshot
            .objs
            .get("user_avatar")
            .unwrap()
            .contains_key("user--00000000001")
    );
}

#[tokio::test]
async fn delete_pending_avatar_enqueues_exact_object_delete() {
    //
    let mock = Mock::new();

    seed_user_with_avatar(&mock, false);

    user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "user--00000000001".into(),
    )
    .await
    .unwrap();

    assert!(mock.snapshot().obj_tasks.iter().any(|(_, task)| {
        matches!(
            task,
            ObjTask::Delete { key }
                if key.id == "user--00000000001" && key.ver == 2
        )
    }));
}

#[tokio::test]
async fn delete_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user--00000000001", "qid-1", "Nick"),
        credential("user--00000000001", "password"),
    );

    let error = user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000002"),
        "user--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.credentials.len(), 1);
}

#[tokio::test]
async fn delete_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let error = user_delete_usecase::delete::<_, MockContext, _, _>(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "user--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}
