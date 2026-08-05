//! Shared model fixtures used across use-case tests.

use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::read::proj::workset::WorksetInfo;

/// Builds a [`UserInfo`] fixture with default timestamps and no avatar.
pub fn user(id: &str, qid: &str, nickname: &str) -> UserInfo {
    //
    let time = OffsetDateTime::now_utc();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a [`UserCredential`] with a properly hashed password.
pub fn credential(user_id: &str, password: &str) -> UserCredential {
    //
    let password_hash = match UserComplex::hash_password_for_test(password) {
        //
        Ok(password_hash) => password_hash,

        Err(_) => panic!("failed to hash password"),
    };

    UserCredential {
        user_id: user_id.into(),
        password_hash,
    }
}

/// Builds a [`UserCredential`] that will never match any real password.
pub fn invalid_credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "invalid-password-hash".into(),
    }
}

/// Builds a [`TeamInfo`] fixture with default timestamps and no avatar.
pub fn team(id: &str, name: &str, description: &str) -> TeamInfo {
    //
    let time = OffsetDateTime::now_utc();

    TeamInfo {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a [`WorksetInfo`] fixture.
pub fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 0,
        created_at: time,
        updated_at: time,
    }
}
