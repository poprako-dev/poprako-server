#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// list_infos(list_infos)(positive): team member should list team comments.
// list_infos(list_infos)(positive): user include should be populated only when requested.
// list_infos(list_infos)(negative): non-member should be rejected from team comments.
// create(create)(positive): team member should create a comment.
// create(create)(positive): comment content should be trimmed before persistence.
// create(create)(negative): non-member should be rejected without mutation.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::comment::{CreateCommentInstr, ListCommentInfosInstr};
use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{assert_expected_variant, now};
use crate::value::comment::CommentInclOpt;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    // Build token fixture for comment perm checks.
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    // Build predictable credential fixture for auth-gated comment actions.
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, nickname: &str) -> UserInfo {
    //
    // Build a user fixture with stable avatar defaults.
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: nickname.into(),
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
    // Build a team member fixture with a fixed role assignment.
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

fn list_instr(
    team_id: &str,
    incl_opt: Vec<CommentInclOpt>,
) -> ListCommentInfosInstr {
    ListCommentInfosInstr {
        team_id: team_id.into(),
        incl_opt,
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    }
}

fn create_instr(team_id: &str) -> CreateCommentInstr {
    CreateCommentInstr {
        team_id: team_id.into(),
        content: "created".into(),
    }
}

fn seed_member(mock: &Mock, user_id: &str, team_id: &str) {
    mock.seed_member(member(
        "member--00000000001",
        user_id,
        team_id,
        RoleMask::from(RoleField::TRANSLATOR),
    ));
}

#[tokio::test]
async fn list_infos_team_member_lists_team_comments() {
    //
    let mock = Mock::new();

    let time = now();

    seed_member(&mock, "user--00000000001", "team--00000000001");

    mock.seed_comment(comment(
        "comment--00000000001",
        "team--00000000001",
        "user--00000000002",
        time,
    ));

    mock.seed_comment(comment(
        "comment--00000000002",
        "team--00000000002",
        "user--00000000002",
        time,
    ));

    let comment_info_vals = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await;

    assert!(comment_info_vals.is_ok());

    let comment_info_vals = comment_info_vals.unwrap();

    assert_eq!(comment_info_vals.len(), 1);

    assert_eq!(
        comment_info_vals.first().unwrap().id,
        "comment--00000000001"
    );
}

#[tokio::test]
async fn list_infos_user_include_follows_request() {
    //
    let mock = Mock::new();

    let time = now();

    seed_member(&mock, "user--00000000001", "team--00000000001");

    mock.seed_user(
        user("user--00000000002", "Author"),
        credential("user--00000000002"),
    );

    mock.seed_comment(comment(
        "comment--00000000001",
        "team--00000000001",
        "user--00000000002",
        time,
    ));

    let without_user = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await;

    assert!(without_user.is_ok());

    assert!(without_user.unwrap().first().unwrap().user.is_none());

    let with_user = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr("team--00000000001", vec![CommentInclOpt::User]),
    )
    .await;

    assert!(with_user.is_ok());

    let with_user = with_user.unwrap();

    assert_eq!(
        with_user.first().unwrap().user.as_ref().unwrap().id,
        "user--00000000002"
    );
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_comment(comment(
        "comment--00000000001",
        "team--00000000001",
        "user--00000000002",
        now(),
    ));

    let error = list_infos(
        (&mock, &mock),
        token("outsider-user"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_team_member_creates_comment() {
    //
    let mock = Mock::new();

    seed_member(&mock, "user--00000000001", "team--00000000001");

    let created_comment = create(
        &mock,
        token("user--00000000001"),
        create_instr("team--00000000001"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comments.len(), 1);

    assert_eq!(snapshot.comments.first().unwrap().id, created_comment.id);

    assert_eq!(
        snapshot.comments.first().unwrap().team_id,
        "team--00000000001"
    );

    assert_eq!(
        snapshot.comments.first().unwrap().user_id,
        "user--00000000001"
    );
}

#[tokio::test]
async fn create_trims_unicode_whitespace_from_comment_content() {
    //
    let mock = Mock::new();

    seed_member(&mock, "user--00000000001", "team--00000000001");

    let mut instr = create_instr("team--00000000001");

    instr.content = "\u{2003}comment content\u{2003}".into();

    create(&mock, token("user--00000000001"), instr)
        .await
        .unwrap();

    assert_eq!(
        mock.snapshot().comments.first().unwrap().content,
        "comment content"
    );
}

#[tokio::test]
async fn create_non_member_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    let error = create(
        &mock,
        token("outsider-user"),
        create_instr("team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert!(mock.snapshot().comments.is_empty());
}
