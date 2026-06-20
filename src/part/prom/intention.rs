use serde::{Deserialize, Serialize};

pub const IMAGE_TOPIC: &str = "image";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageKind {
    UserAvatar,
    TeamAvatar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageIntention {
    CheckUploaded {
        kind: ImageKind,
        resource_id: String,
        object_key: String,
        image_version: i64,
    },
    Delete {
        object_key: String,
    },
}
