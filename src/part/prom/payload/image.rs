use serde::{Deserialize, Serialize};

/// Image-owning resource discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    UserAvatar,
    TeamAvatar,
    ComicCover,
    PageImage,
}

/// Deferred image operation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    CheckUpload {
        resource_kind: ResourceKind,
        resource_id: String,
        object_key: String,
        version: u32,
    },
    Delete {
        object_key: String,
    },
}
