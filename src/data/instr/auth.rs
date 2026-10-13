//! Instr DTOs for the auth domain.

//! Data transfer objects for authentication use cases.

use serde::Deserialize;

use poprako_sensitive::SensitiveDebug;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Input parameters for user registration.
#[derive(SensitiveDebug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthInstr {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// Display name for the user.
    pub nickname: String,

    /// User password.
    #[sensitive]
    pub password: String,

    /// Verification code.
    #[sensitive]
    pub code: String,
}

/// Input parameters for user login.
#[derive(SensitiveDebug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthInstr {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// User password.
    #[sensitive]
    pub password: String,
}
