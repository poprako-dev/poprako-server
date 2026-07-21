use super::*;

// run_reads_seeded_user(GetUserInfo)(positive): a seeded user should be readable outside a transaction.
// nucl_coord_commits_repo_and_prom(CreateMember, Defer)(positive): successful coordination should commit repo and prom state together.
// nucl_coord_rolls_back_repo_and_prom(CreateMember, Defer)(negative): failed coordination should discard repo and prom state together.

/// Build a minimal `UserInfo` for test seeding.
fn user(id: &str) -> UserInfo {
    //
    let time = now();

    UserInfo {
        id: id.into(),
        qid: "qid".into(),
        nickname: "nick".into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

/// Mock helper that verifies a seeded user is readable outside a transaction.
#[tokio::test]
async fn run_reads_seeded_user() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1"),
        UserCredential {
            user_id: "user-1".into(),
            password_hash: "hash".into(),
        },
    );

    let found = mock.run(&GetUserInfo::Id { id: "user-1" }).await;

    assert!(found.is_ok());

    let found = found.ok().unwrap();

    assert_eq!(found.id, "user-1");
}

#[tokio::test]
async fn nucl_coord_commits_repo_and_prom() {
    //
    let mock = Mock::new();

    let member_entry = MemberEntry {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };

    let repo = mock.clone();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            let create_member = CreateMember {
                entry: &member_entry,
            };

            repo.step(context, &create_member).await?;

            let prom_id = "prom-1".to_string();

            let payload = Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "key".to_string(),
                version: 1,
            });

            let task = Task {
                id: &prom_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;

            Ok::<(), BaseError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.members.len(), 1);

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn nucl_coord_rolls_back_repo_and_prom() {
    //
    let mock = Mock::new();

    let member_entry = MemberEntry {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "nick".into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    };

    let repo = mock.clone();

    let prom = mock.clone();

    let err = mock
        .coord(async move |context| {
            //
            repo.step(
                context,
                &CreateMember {
                    entry: &member_entry,
                },
            )
            .await?;

            let prom_id = "prom-1".to_string();

            let payload = Payload::Image(image::Payload::Delete {
                object_key: "key".to_string(),
            });

            let task = Task {
                id: &prom_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;

            Err::<(), _>(unrecoverable(
                "[nucl_coord_rolls_back_repo_and_prom] fail",
            ))
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(
        err,
        NuclError::Step(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.members.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn nucl_coord_commits_state() {
    //
    let mock = Mock::new();

    Nucl::coord(&mock, async |context| {
        //
        context.state.users.push(user("user-1"));

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);
}

#[tokio::test]
async fn nucl_coord_rolls_back_state() {
    //
    let mock = Mock::new();

    let error = Nucl::coord(&mock, async |context| {
        //
        context.state.users.push(user("user-1"));

        Err::<(), _>(unrecoverable("[nucl_coord_rolls_back_state] fail"))
    })
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        NuclError::Step(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());
}
