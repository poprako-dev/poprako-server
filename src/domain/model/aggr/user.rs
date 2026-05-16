use std::mem;

use crate::domain::model::event::{DomainEvent, EventBuffer, EventEmit};

use time::OffsetDateTime;

pub struct UserToken {
    pub id: String,
}

pub struct User {
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

impl User {
    pub fn generate_one_token(&self) -> UserToken {
        UserToken {
            id: self.id.clone(),
        }
    }

    pub fn generate_avatar_key(&self) -> String {
        format!(
            "user/{}/avatar/{}",
            self.id,
            OffsetDateTime::now_utc().unix_timestamp()
        )
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
    pub qid: String,
    pub nickname: String,

    pub password: String,

    events: Vec<DomainEvent>,
}

impl UserForm {
    pub fn generate_password_hash(&self) -> String {
        unimplemented!()
    }
}

impl EventBuffer for UserForm {
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
