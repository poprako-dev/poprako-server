use serde::{Deserialize, Serialize};

/// Image-owning resource discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    /// Avatar image for a user.
    UserAvatar,
    /// Avatar image for a team.
    TeamAvatar,
    /// Cover image for a comic.
    ComicCover,
    /// Page image for a chapter page.
    PageImage,
}

/// Deferred image operation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    /// Verify that an uploaded image object exists and confirm the current DB ownership.
    CheckUpload {
        resource_kind: ResourceKind,
        resource_id: String,
        object_key: String,
        version: u32,
    },
    /// Delete an image object by object-storage key.
    Delete { object_key: String },
}
