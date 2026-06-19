use serde::{Deserialize, Serialize};

pub const IMAGE_TOPIC: &str = "local_message_topic:image";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResourceKind {
    UserAvatar,
    TeamAvatar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageIntention {
    CheckUploaded {
        resource_kind: ImageResourceKind,
        resource_id: String,
        object_key: String,
        image_version: i64,
    },
    Delete {
        object_key: String,
    },
}
