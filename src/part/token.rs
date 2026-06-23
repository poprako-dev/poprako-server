//! Authentication token port.

use crate::model::user::UserToken;
use crate::result::RootResult;

/// Signs and verifies authentication tokens for user sessions.
///
/// Takes a [`UserToken`] domain model (containing the user's identifier)
/// and produces a signed token string suitable for use as a bearer token.
pub trait TokenAuth {
    fn sign(&self, token: &UserToken) -> RootResult<String>;
}
