use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalMessageStatus {
    Pending,
    Processing,
    Completed,
    Dead,
}

impl LocalMessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "local_message_status:pending",
            Self::Processing => "local_message_status:processing",
            Self::Completed => "local_message_status:completed",
            Self::Dead => "local_message_status:dead",
        }
    }
}

#[cfg_attr(test, derive(Clone))]
pub struct LocalMessageAggr {
    pub id: String,

    pub topic: String,
    pub status: LocalMessageStatus,
    pub payload: Value,

    pub last_error: Option<String>,
    pub retried_count: i64,
    pub lease: i64,

    pub visible_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl LocalMessageAggr {
    pub fn generate_id() -> String {
        format!("local_message-{}", Uuid::now_v7())
    }
}

pub struct LocalMessageForm {
    pub id: String,

    pub topic: String,
    pub payload: Value,

    pub visible_at: OffsetDateTime,
}

/// Requested state transition for a claimed local message.
///
/// `Processing` is marked by query infra when `claim` returns.
pub enum LocalMessageMark {
    Pending {
        id: String,
        lease: i64,
        next_visible_at: OffsetDateTime,
        last_error: String,
    },
    Completed {
        id: String,
        lease: i64,
    },
    Dead {
        id: String,
        lease: i64,
        last_error: String,
    },
}
