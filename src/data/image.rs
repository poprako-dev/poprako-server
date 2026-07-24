//! Shared image-upload request and response data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::image::{ImageExt, ImageHash};

/// Content-bound image reservation request.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveImageParams {
    /// SHA-256 identity of the exact upload bytes.
    pub image_hash: ImageHash,
    /// Upload size used only for validation and PUT signing.
    pub byte_length: u64,
    /// File format persisted as part of the image identity.
    pub ext: ImageExt,
}

/// Presigned capability for one pending image upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImageUploadSlotVal {
    /// Presigned PUT URL.
    pub put_url: String,
    /// Monotonic image identity version.
    pub image_version: u32,
    /// Headers bound into the PUT signature.
    pub headers: BTreeMap<String, String>,
}

/// Single-resource image reservation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveImagePayload {
    /// Upload capability, absent when this content is already uploaded.
    pub slot: Option<ImageUploadSlotVal>,
}

/// Request to confirm one reserved image version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkImageUploadedParams {
    /// Version returned in the upload slot.
    pub image_version: u32,
}
