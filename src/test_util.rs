use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::model::member::MemberInfo;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::role::RoleMask;
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::result::{ExpectedVariant, RootError};

pub fn assert_expected_variant(err: RootError, expected: ExpectedVariant) {
    let RootError::Expected { variant, .. } = err else {
        panic!("expected RootError::Expected");
    };

    match (variant, expected) {
        (ExpectedVariant::Args, ExpectedVariant::Args)
        | (ExpectedVariant::Auth, ExpectedVariant::Auth)
        | (ExpectedVariant::Perm, ExpectedVariant::Perm) => {}
        _ => panic!("unexpected ExpectedVariant"),
    }
}

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

// TODO: move to seperate files.

pub fn user(id: &str, qid: &str, nickname: &str) -> UserInfo {
    let time = now();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
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

pub fn user_with_avatar(
    id: &str,
    qid: &str,
    nickname: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: i64,
) -> UserInfo {
    UserInfo {
        avatar_key: Some(avatar_key.into()),
        avatar_uploaded,
        avatar_version,
        ..user(id, qid, nickname)
    }
}

pub fn credential(user_id: &str, password: &str) -> UserCredential {
    let password_hash = match UserComplex::hash_password(password) {
        Ok(password_hash) => password_hash,
        Err(_) => panic!("failed to hash password"),
    };

    UserCredential {
        user_id: user_id.into(),
        password_hash,
    }
}

pub fn invalid_credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "invalid-password-hash".into(),
    }
}

pub fn member(id: &str, user_id: &str, user_nickname: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        team_id: team_id.into(),
    }
}

pub fn invitation(
    id: &str,
    team_id: &str,
    invitor_id: &str,
    invitee_qid: &str,
    code: &str,
    pending: bool,
) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.into(),
        team_id: team_id.into(),
        invitor_id: invitor_id.into(),
        invitee_qid: invitee_qid.into(),
        code: code.into(),
        pending,
        role_mask: RoleMask(1),
    }
}

pub fn team(id: &str, name: &str, description: &str) -> TeamInfo {
    let time = now();

    TeamInfo {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        workset_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

pub fn team_with_avatar(
    id: &str,
    name: &str,
    description: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: i64,
) -> TeamInfo {
    TeamInfo {
        avatar_key: Some(avatar_key.into()),
        avatar_uploaded,
        avatar_version,
        ..team(id, name, description)
    }
}

pub fn workset(id: &str, team_id: &str) -> WorksetInfo {
    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
    }
}
