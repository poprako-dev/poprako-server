use serde::{Deserialize, Serialize};

use crate::value::image::ImageKind;

/// Deferred image payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePayload {
    //
    /// Verify that an uploaded image object exists and confirm the current DB ownership.
    CheckUpload {
        /// Discriminator for the resource type that owns this image.
        image_kind: ImageKind,
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
