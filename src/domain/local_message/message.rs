use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::Duration;
use time::OffsetDateTime;

use crate::domain::model::aggr::local_message::{LocalMessageAggr, LocalMessageForm};

pub const IMAGE_TOPIC: &str = "local_message_topic:image";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResourceKind {
    UserAvatar,
    TeamAvatar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageLocalMessage {
    CheckUploaded {
        resource_kind: ImageResourceKind,
        resource_id: String,
        object_key: String,
        avatar_version: i64,
    },
    Delete {
        object_key: String,
    },
}

impl ImageLocalMessage {
    pub fn check_uploaded(
        resource_kind: ImageResourceKind,
        resource_id: String,
        object_key: String,
        avatar_version: i64,
    ) -> Self {
        Self::CheckUploaded {
            resource_kind,
            resource_id,
            object_key,
            avatar_version,
        }
    }

    pub fn delete(object_key: String) -> Self {
        Self::Delete { object_key }
    }

    pub fn into_form(self, delay: Duration) -> LocalMessageForm {
        let payload = serde_json::to_value(self).unwrap_or(Value::Null);

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
    // image_local_message_serde_roundtrip(ImageLocalMessage serde)(positive): image local messages should round-trip through JSON.

    use super::*;

    #[test]
    fn image_local_message_serde_roundtrip() {
        let message = ImageLocalMessage::check_uploaded(
            ImageResourceKind::UserAvatar,
            "user-1".to_string(),
            "user_avatar/user-1-1.png".to_string(),
            1,
        );

        let value = serde_json::to_value(&message).unwrap();
        let decoded: ImageLocalMessage = serde_json::from_value(value).unwrap();

        assert_eq!(decoded, message);
    }
}
