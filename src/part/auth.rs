//! Authentication token port.

use crate::model::user::UserToken;
use crate::result::BaseResult;

/// Signs and verifies authentication tokens for user sessions.
///
/// Takes a [`UserToken`] domain model (containing the user's identifier)
/// and produces a signed token string suitable for use as a bearer token.
pub trait TokenAuth {
    /// Signs a authorized token with states embedded.
    fn sign_token(&self, token: &UserToken) -> BaseResult<String>;

    /// Verifies a raw bearer token and returns the decoded [`UserToken`].
    fn verify_token(&self, raw: &str) -> BaseResult<UserToken>;
}
