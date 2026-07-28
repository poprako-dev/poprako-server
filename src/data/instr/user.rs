//! Instr DTOs for the user domain.

//! Data transfer objects for user profile use cases.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

pub use crate::data::instr::image::{
    MarkImageUploadedInstr as MarkUserAvatarUploadedInstr,
    ReserveImageInstr as ReserveUserAvatarInstr,
};

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
