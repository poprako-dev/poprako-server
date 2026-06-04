use anyhow::Context;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::Level;
use tracing::instrument;

use crate::domain::external::token::{TokenParse, TokenSign};
use crate::domain::model::aggr::user::UserToken;
use crate::domain::result::{DomainError, DomainResult};

#[derive(Debug, Serialize)]
struct SignClaims<'a> {
    /// Subject — the user id.
    sub: &'a str,
    /// Issued at (unix timestamp).
    iat: usize,
    /// Expiration (unix timestamp).
    exp: usize,
}

#[derive(Debug, Deserialize)]
struct Claims {
    /// Subject — the user id.
    sub: String,
    /// Issued at (unix timestamp).
    #[allow(dead_code)]
    iat: usize,
    /// Expiration (unix timestamp).
    #[allow(dead_code)]
    exp: usize,
}

pub struct JwtIssuer {
    expiration_seconds: i64,

    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtIssuer {
    pub fn from_env() -> anyhow::Result<Self> {
        let secret_key = std::env::var("JWT_SECRET_KEY")
            .with_context(|| "[JwtCodec::from_env] JWT_SECRET_KEY is not set")?;
        let expiration_hours: i64 = std::env::var("JWT_EXPIRATION_HOURS")
            .with_context(|| "[JwtCodec::from_env] JWT_EXPIRATION_HOURS is not set")?
            .parse()
            .with_context(|| "[JwtCodec::from_env] JWT_EXPIRATION_HOURS must be a valid integer")?;

        let expiration_seconds = expiration_hours * 3600;

        tracing::debug!(
            expiration_hours = %expiration_hours,
            "[JwtCodec::from_env] configured",
        );

        Ok(Self {
            expiration_seconds,
            encoding_key: EncodingKey::from_secret(secret_key.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret_key.as_bytes()),
        })
    }
}

impl TokenSign for JwtIssuer {
    #[instrument(skip(self), level = Level::DEBUG)]
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        let now = OffsetDateTime::now_utc();

        let issued_at = now.unix_timestamp() as usize;
        let expiration = issued_at + self.expiration_seconds as usize;

        let claims = SignClaims {
            sub: &unsigned_token.user_id,
            iat: issued_at,
            exp: expiration,
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            DomainError::unrecoverable(format!("[JwtCodec::sign] error when encoding: {}", e))
                .trace()
        })
    }
}

impl TokenParse for JwtIssuer {
    #[instrument(skip(self), level = Level::DEBUG)]
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        let validation = Validation::default();

        let token_data: TokenData<Claims> = decode(signed_token, &self.decoding_key, &validation)
            .map_err(|e| {
            DomainError::unrecoverable(format!("[JwtCodec::parse] error when decoding: {}", e))
                .trace()
        })?;

        let Claims { sub, .. } = token_data.claims;

        Ok(UserToken { user_id: sub })
    }
}
