//! Instr DTOs for the image domain.

//! Shared image-upload request and response data.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::image::{ImageExt, ImageHash};

/// Content-bound image reservation request.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveImageInstr {
    //
    /// SHA-256 identity of the exact upload bytes.
    pub image_hash: ImageHash,
    /// Upload size used only for validation and PUT signing.
    pub new_byte_len: u64,
    /// File format persisted as part of the image identity.
    pub ext: ImageExt,
}

/// Request to confirm one reserved image version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkImageUploadedInstr {
    /// Version returned in the upload slot.
    pub image_version: u32,
}
