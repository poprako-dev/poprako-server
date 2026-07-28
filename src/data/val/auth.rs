//! Val DTOs for the auth domain.

//! Data transfer objects for authentication use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from a successful registration.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthVal {
    //
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}

/// Return value from a successful login.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthVal {
    //
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}
