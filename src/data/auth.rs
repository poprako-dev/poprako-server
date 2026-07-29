//! Data transfer objects for authentication use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Input parameters for user registration.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthParams {
    //
    /// Unique user identifier for authentication.
    pub qid: String,
    /// Display name for the user.
    pub nickname: String,

    /// User password.
    pub password: String,

    /// Verification code.
    pub code: String,
}

/// Return value from a successful registration.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthPayload {
    //
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}

/// Input parameters for user login.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthParams {
    //
    /// Unique user identifier for authentication.
    pub qid: String,
    /// User password.
    pub password: String,
}

/// Return value from a successful login.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthPayload {
    //
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}
