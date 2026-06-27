//! Deferred image operation types used by the prom system.

use serde::{Deserialize, Serialize};

/// The prom topic for image-related deferred actions.
///
/// Used as the `topic` field in [`Append`] steps so the prom worker
/// can filter and route image intentions to the correct handler.
///
/// [`Append`]: crate::part::prom::Append
pub const IMAGE_TOPIC: &str = "image";

/// Discriminates the resource type an [`ImageIntention`] targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageKind {
    UserAvatar,
    TeamAvatar,
    ComicCover,
    PageImage,
}

/// A deferred image operation to be executed after transaction commit.
///
/// These intentions are serialized into [`Payload::Image`] prom records.
/// The prom worker deserializes and executes them once their `visible_at`
/// timestamp has passed.
///
/// [`Payload::Image`]: crate::part::prom::Payload::Image
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageIntention {
    /// Verify that an upload completed by checking the object exists in storage.
    ///
    /// Visible after a short delay (typically 15 minutes) to give the client
    /// time to complete the upload.
    CheckUploaded {
        kind: ImageKind,
        resource_id: String,
        object_key: String,
        image_version: i64,
    },
    /// Delete an object from storage (e.g., an old avatar after a replacement).
    Delete { object_key: String },
}
