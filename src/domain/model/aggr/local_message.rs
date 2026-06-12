use serde_json::Value;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::model::value::local_message::{ImageLocalMessage, IMAGE_TOPIC};

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

impl LocalMessageForm {
    pub fn from_image_message(msg: ImageLocalMessage, delay: Duration) -> Self {
        let payload = serde_json::to_value(msg).unwrap_or(Value::Null);

        LocalMessageForm {
            id: LocalMessageAggr::generate_id(),
            topic: IMAGE_TOPIC.to_string(),
            payload,
            visible_at: OffsetDateTime::now_utc() + delay,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_as_str_maps_all_variants() {
        assert_eq!(LocalMessageStatus::Pending.as_str(), "local_message_status:pending");
        assert_eq!(LocalMessageStatus::Processing.as_str(), "local_message_status:processing");
        assert_eq!(LocalMessageStatus::Completed.as_str(), "local_message_status:completed");
        assert_eq!(LocalMessageStatus::Dead.as_str(), "local_message_status:dead");
    }

    #[test]
    fn generate_id_produces_prefixed_uuid() {
        let id = LocalMessageAggr::generate_id();
        assert!(id.starts_with("local_message-"));
    }

    #[test]
    fn from_image_message_sets_topic_and_payload() {
        let msg = ImageLocalMessage::delete("test-key".into());
        let form = LocalMessageForm::from_image_message(msg, time::Duration::seconds(0));
        assert_eq!(form.topic, crate::domain::model::value::local_message::IMAGE_TOPIC);
        assert!(form.payload.is_object());
    }
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
