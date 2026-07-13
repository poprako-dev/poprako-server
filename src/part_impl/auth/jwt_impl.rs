//! JWT-backed authentication token signer.

use anyhow::Context as _;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{Level, instrument};

use poprako_util::i18n::trl;

use crate::model::user::{UserToken, UserTokenRef};
use crate::part::auth::TokenAuth;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

/// JWT issuer for user session tokens.
pub struct JwtAuth {
    expiration_seconds: i64,

    encoding_key: EncodingKey,

    decoding_key: DecodingKey,
}

/// Internal JWT claim structure used for token signing.
#[derive(Debug, Serialize)]
struct SignClaims<'a> {
    sub: &'a str,
    user_id: &'a str,

    iat: usize,
    nbf: usize,
    exp: usize,

    iss: &'static str,
}

/// Decoded JWT token claims containing user identification.
#[derive(Debug, Deserialize)]
struct TokenClaims {
    user_id: String,
}

impl JwtAuth {
    /// Creates a JWT signer from a shared secret and token lifetime.
    pub fn new(secret: &str, expiration_hours: i64) -> RegularResult<Self> {
        //
        if expiration_hours <= 0 {
            return Err(RegularError::Unrecoverable {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS must be positive"
                    .to_string(),
            });
        }

        let expiration_seconds = expiration_hours * 3600;

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        Ok(Self {
            expiration_seconds,
            encoding_key,
            decoding_key,
        })
    }

    /// Reads JWT settings from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        let secret = std::env::var("JWT_SECRET")
            .with_context(|| "[JwtAuth::from_env] JWT_SECRET is not set")?;

        let expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
            .with_context(|| "[JwtAuth::from_env] JWT_EXPIRATION_HOURS is not set")?
            .parse()
            .with_context(
                || "[JwtAuth::from_env] JWT_EXPIRATION_HOURS must be an integer",
            )?;

        Self::new(&secret, expiration_hours).map_err(|err| match err {
            RegularError::Expected { message, .. }
            | RegularError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
            }
        })
    }
}

impl TokenAuth for JwtAuth {
    #[instrument(err(Debug), skip(self, token), level = Level::DEBUG)]
    fn sign_token(&self, token: &UserTokenRef) -> RegularResult<String> {
        //
        let now = OffsetDateTime::now_utc();

        let issued_at = now.unix_timestamp() as usize;

        let expiration =
            (now.unix_timestamp() + self.expiration_seconds) as usize;

        let claims = SignClaims {
            sub: token.user_id,
            user_id: token.user_id,
            iat: issued_at,
            nbf: issued_at,
            exp: expiration,
            iss: "poprako-server",
        };

        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &self.encoding_key).map_err(|err| {
            RegularError::Unrecoverable {
                message: format!(
                    "[JwtAuth::sign_token] error when encoding: {}",
                    err
                ),
            }
        })
    }

    #[instrument(err(Debug), skip(self), level = Level::DEBUG)]
    fn verify_token(&self, raw: &str) -> RegularResult<UserToken> {
        //
        let token_data = decode::<TokenClaims>(
            raw,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| {
            //
            tracing::debug!("[JwtAuth::verify_token] decode failed: {}", err);

            RegularError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-unauthorized"),
            }
        })?;

        Ok(UserToken {
            user_id: token_data.claims.user_id,
        })
    }
}

#[cfg(test)]
mod tests;
