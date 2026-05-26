use std::mem;

use crate::domain::model::event::{DomainEvent, EventEmit, EventSink};

use time::OffsetDateTime;
use uuid::Uuid;

pub struct UserToken {
    pub user_id: String,
}

impl UserToken {
    pub fn new(user_id: String) -> Self {
        Self { user_id }
    }
}

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
}

impl UserAggr {
    pub fn generate_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_key(&self) -> String {
        format!("user_avatar/{}", self.id,)
    }
}

pub struct UserCredential {
    pub qid: String,
    pub password_hash: String,
}

impl UserCredential {
    pub fn verify_password(&self, password: &str) -> bool {
        unimplemented!()
    }
}

pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,

    events: Vec<DomainEvent>,
}

impl UserForm {
    pub fn new(qid: String, nickname: String, password: String) -> Self {
        Self {
            id: UserAggr::generate_id(),
            qid,
            nickname,
            password_hash: password,
            events: Vec::new(),
        }
    }
}

impl EventSink for UserForm {
    fn push_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }
}

impl EventEmit for UserForm {
    fn pull_events(&mut self) -> Vec<DomainEvent> {
        // A swap-and-clear pattern to avoid cloning the events.
        mem::take(&mut self.events)
    }
}

// UserInfoUpdate is used for **PUT** update of user info.
// TODO: No password update support for now.
pub struct UserInfoUpdate {
    pub id: String,

    pub qid: String,
    pub nickname: String,
}
