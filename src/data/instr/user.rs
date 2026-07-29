//! Instr DTOs for the user domain.

//! Data transfer objects for user profile use cases.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::image::{ImageExt, ImageHash};

/// Request to reserve a user avatar upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveUserAvatarInstr {
    //
    /// SHA-256 identity of the exact avatar bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Avatar file format.
    pub ext: ImageExt,
}

/// Request to confirm one reserved user avatar version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkUserAvatarUploadedInstr {
    /// Version returned in the avatar upload slot.
    pub image_version: u32,
}

/// Input parameters for updating a user's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateUserInfoInstr {
    //
    /// User identifier to update.
    pub id: String,

    /// Updated qualified identifier for login.
    pub qid: String,
    /// Updated display nickname.
    pub nickname: String,
}

/// Input parameters for replacing the authenticated user's password.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateUserPasswordInstr {
    //
    /// Current password for verification before change.
    pub current_password: String,
    /// Desired new password.
    pub new_password: String,
}
