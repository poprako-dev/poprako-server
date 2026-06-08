use poprako_macro::{forward_ref, forward_ref_sub};

use crate::domain::model::aggr::user::UserToken;
use crate::domain::result::DomainResult;

/// Signs unsigned [`UserToken`]s into encoded strings (JWT).
#[forward_ref]
pub trait TokenSign {
    /// Signs an unsigned [`UserToken`] and returns the encoded string.
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String>;
}

/// Parses signed token strings back into [`UserToken`]s, verifying signature and expiry.
#[forward_ref]
pub trait TokenParse {
    /// Parses a signed token string back into a [`UserToken`],
    /// verifying signature and expiry in the process.
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken>;
}

/// Composite token issuing contract for signing and parsing authentication tokens.
#[forward_ref_sub]
pub trait TokenIssuer: TokenSign + TokenParse {}

impl<T> TokenIssuer for T where T: TokenSign + TokenParse {}
