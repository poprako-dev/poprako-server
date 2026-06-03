use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResult;
use crate::util::ForwardRef;

/// Forwarding marker for [`TokenSign`].
pub struct TokenSignForward;

/// Forwarding marker for [`TokenParse`].
pub struct TokenParseForward;

/// Signs unsigned [`UserToken`]s into encoded strings (JWT).
pub trait TokenSign {
    /// Signs an unsigned [`UserToken`] and returns the encoded string.
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String>;
}

impl<T> TokenSign for T
where
    T: ForwardRef<TokenSignForward>,
    T::Target: TokenSign,
{
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        self.forward_ref().sign(unsigned_token)
    }
}

/// Parses signed token strings back into [`UserToken`]s, verifying signature and expiry.
pub trait TokenParse {
    /// Parses a signed token string back into a [`UserToken`],
    /// verifying signature and expiry in the process.
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken>;
}

impl<T> TokenParse for T
where
    T: ForwardRef<TokenParseForward>,
    T::Target: TokenParse,
{
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        self.forward_ref().parse(signed_token)
    }
}

// /// Signs and parses user authentication tokens (JWT).
// pub trait TokenCodec: TokenSigner + TokenParser {}
//
// impl<T> TokenCodec for T where T: TokenSigner + TokenParser {}
