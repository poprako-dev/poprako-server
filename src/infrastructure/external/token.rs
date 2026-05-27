use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::Level;

use crate::domain::external::token::TokenCodec;
use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::{DomainErr, DomainResl};
use crate::util::err::ErrorTrace as _;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject — the user id.
    sub: String,
    /// Issued at (unix timestamp).
    iat: usize,
    /// Expiration (unix timestamp).
    exp: usize,
}

/// A JWT-based implementation of the `TokenCodec` trait.
pub struct JwtCodec {
    expiration_seconds: i64,

    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtCodec {
    pub fn new(secret_key: String, expiration_seconds: i64) -> Self {
        Self {
            expiration_seconds,
            encoding_key: EncodingKey::from_secret(secret_key.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret_key.as_bytes()),
        }
    }
}

impl TokenCodec for JwtCodec {
    #[tracing::instrument(skip(self), level = Level::DEBUG)]
    fn sign(&self, unsigned_token: &UserToken) -> DomainResl<String> {
        let now = OffsetDateTime::now_utc();

        let issued_at = now.unix_timestamp() as usize;
        let expiration = issued_at + self.expiration_seconds as usize;

        let claims = Claims {
            sub: unsigned_token.user_id.clone(),
            iat: issued_at,
            exp: expiration,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| {
                DomainErr::unrecoverable(format!("[JwtCodec::sign] error when encoding: {}", e))
            })
            .trace_error()
    }

    #[tracing::instrument(skip(self), level = Level::DEBUG)]
    fn parse(&self, signed_token: &str) -> DomainResl<UserToken> {
        let validation = Validation::default();

        let token_data: TokenData<Claims> = decode(signed_token, &self.decoding_key, &validation)
            .map_err(|e| {
                DomainErr::unrecoverable(format!("[JwtCodec::parse] error when decoding: {}", e))
            })
            .trace_error()?;

        Ok(UserToken::new(token_data.claims.sub))
    }
}
