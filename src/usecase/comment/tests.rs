// list_infos(list_infos)(positive): team member should list team comments.
// list_infos(list_infos)(positive): user include should be populated only when requested.
// list_infos(list_infos)(negative): non-member should be rejected from team comments.
// create(create)(positive): team member should create a comment.
// create(create)(negative): non-member should be rejected without mutation.

use super::*;

use time::OffsetDateTime;

use crate::data::comment::{CreateCommentData, ListCommentInfosData};
use crate::model::comment::CommentInfo;
use crate::model::member::MemberInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{assert_expected_variant, now};
use crate::value::comment::CommentInclOpt;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, nickname: &str) -> UserInfo {
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: nickname.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn member(
    id: &str,
    user_id: &str,
    team_id: &str,
    role_mask: RoleMask,
) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: now(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn comment(
    id: &str,
    team_id: &str,
    user_id: &str,
    created_at: OffsetDateTime,
) -> CommentInfo {
    CommentInfo {
        id: id.into(),
        team_id: team_id.into(),
        user_id: user_id.into(),
        user: None,
        content: "content".into(),
        created_at,
    }
}

fn list_data(
    team_id: &str,
    incl_opt: Vec<CommentInclOpt>,
) -> ListCommentInfosData {
    ListCommentInfosData {
        team_id: team_id.into(),
        incl_opt,
        offset: 0,
        limit: 10,
    }
}

fn create_data(team_id: &str) -> CreateCommentData {
    CreateCommentData {
        team_id: team_id.into(),
        content: "created".into(),
    }
}

fn seed_member(mock: &Mock, user_id: &str, team_id: &str) {
    mock.seed_member(member(
        "member-1",
        user_id,
        team_id,
        RoleMask::from(RoleField::TRANSLATOR),
    ));
}

#[tokio::test]
async fn list_infos_team_member_lists_team_comments() {
    let mock = Mock::new();
    let time = now();
    seed_member(&mock, "viewer-user", "team-1");
    mock.seed_comment(comment("comment-1", "team-1", "author-user", time));
    mock.seed_comment(comment("comment-2", "team-2", "author-user", time));

    let comment_info_vals = list_infos(
        &mock,
        &mock,
        token("viewer-user"),
        list_data("team-1", Vec::new()),
    )
    .await;

    assert!(comment_info_vals.is_ok());
    let comment_info_vals = comment_info_vals.ok().unwrap();

    assert_eq!(comment_info_vals.len(), 1);
    assert_eq!(comment_info_vals[0].id, "comment-1");
}

#[tokio::test]
async fn list_infos_user_include_follows_request() {
    let mock = Mock::new();
    let time = now();
    seed_member(&mock, "viewer-user", "team-1");
    mock.seed_user(user("author-user", "Author"), credential("author-user"));
    mock.seed_comment(comment("comment-1", "team-1", "author-user", time));

    let without_user = list_infos(
        &mock,
        &mock,
        token("viewer-user"),
        list_data("team-1", Vec::new()),
    )
    .await;
    assert!(without_user.is_ok());
    assert!(without_user.ok().unwrap()[0].user.is_none());

    let with_user = list_infos(
        &mock,
        &mock,
        token("viewer-user"),
        list_data("team-1", vec![CommentInclOpt::User]),
    )
    .await;
    assert!(with_user.is_ok());
    let with_user = with_user.ok().unwrap();

    assert_eq!(with_user[0].user.as_ref().unwrap().id, "author-user");
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    let mock = Mock::new();
    mock.seed_comment(comment("comment-1", "team-1", "author-user", now()));

    let err = list_infos(
        &mock,
        &mock,
        token("outsider-user"),
        list_data("team-1", Vec::new()),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_team_member_creates_comment() {
    let mock = Mock::new();
    seed_member(&mock, "viewer-user", "team-1");

    let created_comment =
        create(&mock, &mock, token("viewer-user"), create_data("team-1"))
            .await
            .ok()
            .unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comments.len(), 1);
    assert_eq!(snapshot.comments[0].id, created_comment.id);
    assert_eq!(snapshot.comments[0].team_id, "team-1");
    assert_eq!(snapshot.comments[0].user_id, "viewer-user");
}

#[tokio::test]
async fn create_non_member_is_rejected_without_mutation() {
    let mock = Mock::new();

    let err =
        create(&mock, &mock, token("outsider-user"), create_data("team-1"))
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert!(mock.snapshot().comments.is_empty());
}
