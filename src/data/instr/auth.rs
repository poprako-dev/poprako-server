//! Instr DTOs for the auth domain.

//! Data transfer objects for authentication use cases.

use serde::Deserialize;

#[cfg(not(debug_assertions))]
use veil::Redact;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Input parameters for user registration.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthInstr {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// Display name for the user.
    pub nickname: String,

    /// User password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub password: String,

    /// Verification code.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub code: String,
}

/// Input parameters for user login.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthInstr {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// User password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub password: String,
}
