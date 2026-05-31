use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResult;
use crate::util::DerefTo;

/// Signs and parses user authentication tokens (JWT).
pub trait TokenCodec {
    /// Signs an unsigned [`UserToken`] and returns the encoded string.
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String>;

    /// Parses a signed token string back into a [`UserToken`],
    /// verifying signature and expiry in the process.
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken>;
}

/// Any type whose [`DerefTo::Target`] implements [`TokenCodec`] is itself
/// a [`TokenCodec`], delegating all calls via [`DerefTo::deref_to`].
impl<T> TokenCodec for T
where
    T: DerefTo,
    T::Target: TokenCodec,
{
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        self.deref_to().sign(unsigned_token)
    }

    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        self.deref_to().parse(signed_token)
    }
}
