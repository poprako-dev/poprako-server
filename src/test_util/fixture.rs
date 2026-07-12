//! Shared model fixtures used across use-case tests.

use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::model::team_model;
use crate::model::user_model;
use crate::model::workset_model;

/// Builds a [`UserInfo`] fixture with default timestamps and no avatar.
pub fn user(id: &str, qid: &str, nickname: &str) -> user_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    user_model::Info {
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

/// Builds a [`UserCredential`] with a properly hashed password.
pub fn credential(user_id: &str, password: &str) -> user_model::Credential {
    //
    let password_hash = match UserComplex::hash_password(password) {
        //
        Ok(password_hash) => password_hash,

        Err(_) => panic!("failed to hash password"),
    };

    user_model::Credential {
        user_id: user_id.into(),
        password_hash,
    }
}

/// Builds a [`UserCredential`] that will never match any real password.
pub fn invalid_credential(user_id: &str) -> user_model::Credential {
    user_model::Credential {
        user_id: user_id.into(),
        password_hash: "invalid-password-hash".into(),
    }
}

/// Builds a [`TeamInfo`] fixture with default timestamps and no avatar.
pub fn team(id: &str, name: &str, description: &str) -> team_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    team_model::Info {
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

/// Builds a [`WorksetInfo`] fixture.
pub fn workset(id: &str, team_id: &str) -> workset_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    workset_model::Info {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 0,
        comic_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}
