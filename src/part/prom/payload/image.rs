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

/// Deferred image payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePayload {
    /// Verify that an uploaded image object exists and confirm the current DB ownership.
    CheckUpload {
        //
        /// Discriminator for the resource type that owns this image.
        resource_kind: ResourceKind,
        /// ID of the resource that owns this image.
        resource_id: String,
        /// Object-storage key of the uploaded image.
        object_key: String,
        /// Version counter for optimistic concurrency.
        version: u32,
    },

    /// Delete an image object by object-storage key.
    Delete {
        /// Object-storage key of the image to delete.
        object_key: String,
    },
}
