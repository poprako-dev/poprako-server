//! Authentication token port.

use crate::model::user::UserTokenRef;
use crate::result::RegularResult;

/// Signs and verifies authentication tokens for user sessions.
///
/// Takes a [`UserToken`] domain model (containing the user's identifier)
/// and produces a signed token string suitable for use as a bearer token.
pub trait TokenAuth {
    /// Signs a authorized token with states embedded.
    fn sign_token(&self, token: &UserTokenRef) -> RegularResult<String>;
}
