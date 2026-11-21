//! Val DTOs for the auth domain.
//! Data transfer objects for authentication use cases.
/// Return value from a successful registration.
pub struct RegisterAuthVal {
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}

/// Return value from a successful login.
pub struct LoginAuthVal {
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}
