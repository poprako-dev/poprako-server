//! Instr DTOs for the auth domain.
//! Data transfer objects for authentication use cases.
#[cfg(not(debug_assertions))]
use veil::Redact;

/// Input parameters for user registration.
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
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
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
pub struct LoginAuthInstr {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// User password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub password: String,
}
