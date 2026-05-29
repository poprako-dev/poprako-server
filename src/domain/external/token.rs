use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResult;

/// Signs and parses user authentication tokens (JWT).
pub trait TokenCodec {
    /// Signs an unsigned [`UserToken`] and returns the encoded string.
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String>;

    /// Parses a signed token string back into a [`UserToken`],
    /// verifying signature and expiry in the process.
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken>;
}
