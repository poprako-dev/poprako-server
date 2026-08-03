use super::*;

use time::OffsetDateTime;

use crate::model::read::proj::member::MemberInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::role::{RoleField, RoleMask};

// Builds an authenticated user token for online-user use cases.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

// Builds a team membership accepted by the shared membership gate.
fn member(id: &str, user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    }
}

#[tokio::test]
async fn mark_self_online_adds_member_to_team_list() {
    let mock = Mock::new();

    mock.seed_member(member(
        "online-usecase-member-1",
        "online-usecase-user-1",
        "online-usecase-team-1",
    ));

    mark_self_online(
        (&mock,),
        token("online-usecase-user-1"),
        "online-usecase-team-1".into(),
    )
    .await
    .unwrap();

    let online_user_ids = list_online_user_ids(
        (&mock,),
        token("online-usecase-user-1"),
        "online-usecase-team-1".into(),
    )
    .await
    .unwrap();

    assert_eq!(online_user_ids, ["online-usecase-user-1"]);
}

#[tokio::test]
async fn mark_self_online_rejects_non_member() {
    let mock = Mock::new();

    let err = mark_self_online(
        (&mock,),
        token("online-usecase-outsider-1"),
        "online-usecase-team-2".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_online_user_ids_rejects_non_member() {
    let mock = Mock::new();

    let err = list_online_user_ids(
        (&mock,),
        token("online-usecase-outsider-2"),
        "online-usecase-team-3".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn team_online_user_lists_remain_independent() {
    let mock = Mock::new();

    mock.seed_member(member(
        "online-usecase-member-4a",
        "online-usecase-user-4a",
        "online-usecase-team-4a",
    ));

    mock.seed_member(member(
        "online-usecase-member-4b",
        "online-usecase-user-4b",
        "online-usecase-team-4b",
    ));

    mark_self_online(
        (&mock,),
        token("online-usecase-user-4a"),
        "online-usecase-team-4a".into(),
    )
    .await
    .unwrap();

    mark_self_online(
        (&mock,),
        token("online-usecase-user-4b"),
        "online-usecase-team-4b".into(),
    )
    .await
    .unwrap();

    let online_user_ids = list_online_user_ids(
        (&mock,),
        token("online-usecase-user-4a"),
        "online-usecase-team-4a".into(),
    )
    .await
    .unwrap();

    assert_eq!(online_user_ids, ["online-usecase-user-4a"]);

    let online_user_ids = list_online_user_ids(
        (&mock,),
        token("online-usecase-user-4b"),
        "online-usecase-team-4b".into(),
    )
    .await
    .unwrap();

    assert_eq!(online_user_ids, ["online-usecase-user-4b"]);
}
