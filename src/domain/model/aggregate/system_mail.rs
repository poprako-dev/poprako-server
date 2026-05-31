use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggregate::PrivateMarker;

pub struct SystemMailAggr {
    pub id: String,

    pub receiver_id: String,
    pub read: bool,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl SystemMailAggr {
    pub fn new(
        id: String,
        receiver_id: String,
        read: bool,
        title: String,
        content: String,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            receiver_id,
            read,
            title,
            content,
            created_at,
            _m: PrivateMarker,
        }
    }
}

#[derive(Debug)]
pub struct SystemMailForm {
    pub id: String,

    pub receiver_id: String,

    pub title: String,
    pub content: String,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl SystemMailForm {
    /// Returns a new [`SystemMailForm`] with a generated ID.
    pub fn new(receiver_id: String, title: String, content: String) -> Self {
        Self {
            id: format!("sys_mail-{}", Uuid::now_v7()),
            receiver_id,
            title,
            content,
            _m: PrivateMarker,
        }
    }
}
