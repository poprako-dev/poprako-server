//! Authentication context shared by the HTTP and application layers.

/// A deserialized authentication token identifying a user session.
#[derive(Clone, Debug)]
pub struct UserToken {
    /// Identifier of the user this token authenticates.
    pub user_id: String,
}
