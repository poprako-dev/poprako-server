use crate::domain::external::token::TokenCodec;
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

pub fn sign_token<C>(codec: &C, unsigned_token: &UserToken) -> DomainResult<String>
where
    C: TokenCodec,
{
    codec.sign(unsigned_token)
}

pub fn parse_token<C>(codec: &C, signed_token: &str) -> DomainResult<UserToken>
where
    C: TokenCodec,
{
    codec.parse(signed_token)
}
