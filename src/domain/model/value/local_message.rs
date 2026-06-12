use serde::{Deserialize, Serialize};

pub const IMAGE_TOPIC: &str = "local_message_topic:image";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResourceKind {
    UserAvatar,
    TeamAvatar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageLocalMessage {
    CheckUploaded {
        // FIXME: Necessary?
        resource_kind: ImageResourceKind,
        resource_id: String,
        object_key: String,
        image_version: i64,
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
        image_version: i64,
    ) -> Self {
        Self::CheckUploaded {
            resource_kind,
            resource_id,
            object_key,
            image_version,
        }
    }

    pub fn delete(object_key: String) -> Self {
        Self::Delete { object_key }
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
