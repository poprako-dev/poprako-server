use std::mem;

use crate::domain::model::aggregate::PrivateMarker;
use crate::domain::model::event::{Event, EventEmit, EventSink};
use crate::util::err::ErrorTrace as _;

use time::OffsetDateTime;
use uuid::Uuid;

pub struct UserAggr {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub avatar_key: String,
    pub avatar_uploaded: bool,

    pub is_sadmin: bool,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserAggr {
    pub fn generate_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_key(&self) -> String {
        format!("user_avatar/{}", self.id,)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        nickname: String,
        qid: String,
        is_sadmin: bool,
        avatar_key: String,
        avatar_uploaded: bool,
        last_active_at: OffsetDateTime,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            qid,
            nickname,
            avatar_key,
            avatar_uploaded,
            is_sadmin,
            last_active_at,
            created_at,
            updated_at,
            _m: PrivateMarker,
        }
    }
}

impl EventEmit for UserAggr {
    fn pull_events(&mut self) -> Vec<Event> {
        // UserAggr is a pure read model and should never have any events.
        // This is just a safeguard to catch any accidental misuse.
        Vec::new()
    }
}

#[derive(Debug, Clone)]
pub struct UserToken {
    pub user_id: String,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserToken {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            _m: PrivateMarker,
        }
    }
}

pub struct UserCredential {
    pub qid: String,
    pub password_hash: String,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserCredential {
    pub fn new(qid: String, password_hash: String) -> Self {
        Self {
            qid,
            password_hash,
            _m: PrivateMarker,
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password_hash)
            .trace_error()
            .unwrap_or(false)
    }
}

pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,

    events: Vec<Event>,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserForm {
    pub fn new(qid: String, nickname: String, password: String) -> Self {
        Self {
            id: UserAggr::generate_id(),
            qid,
            nickname,
            password_hash: password,
            events: Vec::new(),
            _m: PrivateMarker,
        }
    }

    pub fn clone_without_events(&self) -> Self {
        // TODO: not elegant.
        Self {
            id: self.id.clone(),
            qid: self.qid.clone(),
            nickname: self.nickname.clone(),
            password_hash: self.password_hash.clone(),
            events: Vec::new(),
            _m: PrivateMarker,
        }
    }
}

impl EventSink for UserForm {
    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }
}

impl EventEmit for UserForm {
    fn pull_events(&mut self) -> Vec<Event> {
        // A swap-and-clear pattern to avoid cloning the events.
        mem::take(&mut self.events)
    }
}

/// Input aggregate for updating user info via PUT.
// TODO: No password update support for now.
pub struct UserInfoUpdate {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserInfoUpdate {
    /// Creates a new `UserInfoUpdate`.
    ///
    /// `id` is the existing user ID (provided by the caller, not generated).
    pub fn new(id: String, qid: String, nickname: String) -> Self {
        Self {
            id,
            qid,
            nickname,
            _m: PrivateMarker,
        }
    }
}

pub struct UserPasswordUpdate {
    pub id: String,

    pub password_hash: String,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl UserPasswordUpdate {
    /// Creates a new `UserPasswordUpdate`.
    ///
    /// `id` is the existing user ID (provided by the caller, not generated).
    pub fn new(id: String, password_hash: String) -> Self {
        Self {
            id,
            password_hash,
            _m: PrivateMarker,
        }
    }
}
