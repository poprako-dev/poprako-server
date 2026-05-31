use crate::domain::external::token::{TokenParse, TokenSign};
use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::{DomainError, DomainResult};
use crate::util::err::ErrorTrace as _;

/// Hashes a password with bcrypt using the default cost factor.
pub fn hash_password(password: &str) -> DomainResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| {
            DomainError::unrecoverable(format!(
                "[user::hash_password] bcrypt hashing failed: {}",
                e
            ))
        })
        .trace_error()
}

pub fn sign_token<C: TokenSign>(codec: &C, unsigned_token: &UserToken) -> DomainResult<String> {
    codec.sign(unsigned_token)
}

pub fn parse_token<C: TokenParse>(codec: &C, signed_token: &str) -> DomainResult<UserToken> {
    codec.parse(signed_token)
}
