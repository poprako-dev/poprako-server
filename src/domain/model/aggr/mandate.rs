use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MandateStatus {
    Pending,
    Processing,
    Completed,
    Dead,
}

#[cfg_attr(test, derive(Clone))]
pub struct MandateAggr {
    pub id: String,

    pub topic: String,
    pub status: MandateStatus,
    pub payload: Value,

    pub last_error: Option<String>,
    pub retried_count: i64,
    pub lease: i64,

    pub visible_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MandateAggr {
    pub fn generate_id() -> String {
        format!("mandate-{}", Uuid::now_v7())
    }
}

pub struct MandateForm {
    pub id: String,

    pub topic: String,
    pub payload: Value,

    pub visible_at: OffsetDateTime,
}

/// Requested state transition for a claimed mandate.
///
/// `Processing` is marked by query infra when `claim` returns.
pub enum MandateMark {
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
