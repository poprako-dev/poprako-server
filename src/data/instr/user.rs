//! Instr DTOs for the user domain.
//! Data transfer objects for user profile use cases.
#[cfg(not(debug_assertions))]
use veil::Redact;

use crate::value::image::{ImageExt, ImageHash};

/// Request to allocate a user avatar upload.
#[derive(Debug)]
pub struct AllocUserAvatarInstr {
    /// SHA-256 identity of the exact avatar bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Avatar file format.
    pub ext: ImageExt,
}

/// Request to confirm one allocated user avatar version.
#[derive(Debug)]
pub struct MarkUserAvatarUploadedInstr {
    /// Version returned in the avatar upload slot.
    pub image_ver: u32,
}

/// Input parameters for updating a user's profile.
#[derive(Debug)]
pub struct UpdateUserInfoInstr {
    /// User identifier to update.
    pub id: String,

    /// Updated qualified identifier for login.
    pub qid: String,
    /// Updated display nickname.
    pub nickname: String,
}

/// Input parameters for replacing the authenticated user's password.
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
pub struct UpdateUserPasswordInstr {
    /// Current password for verification before change.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub current_password: String,
    /// Desired new password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub new_password: String,
}
