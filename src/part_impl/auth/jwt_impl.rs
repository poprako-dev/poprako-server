//! JWT-backed authentication token signer.

use anyhow::Context as _;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::user::UserToken;
use crate::part::auth::TokenAuth;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
mod tests;

/// JWT issuer for user session tokens.
pub struct JwtAuth {
    //
    /// Token lifetime in seconds from issuance.
    expiration_seconds: i64,

    /// RSA/DSA key material used to sign outgoing tokens.
    encoding_key: EncodingKey,

    /// RSA/DSA key material used to verify incoming tokens.
    decoding_key: DecodingKey,
}

/// Internal JWT claim structure used for token signing.
#[derive(Debug, Serialize)]
struct SignClaims<'a> {
    //
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
    pub fn new(secret: &str, expiration_hours: i64) -> BaseResult<Self> {
        //
        if expiration_hours <= 0 {
            return Err(BaseError::Unrecoverable {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS must be positive"
                    .to_string(),
            });
        }

        let expiration_seconds = expiration_hours * 3600;

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        accept(Self {
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
            BaseError::Expected { message, .. }
            | BaseError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
            }
        })
    }
}

impl TokenAuth for JwtAuth {
    #[instrument(level = "info", err(Debug), skip_all)]
    fn sign_token(&self, token: &UserToken) -> BaseResult<String> {
        //
        let now = OffsetDateTime::now_utc();

        let issued_at = now.unix_timestamp() as usize;

        let expiration =
            (now.unix_timestamp() + self.expiration_seconds) as usize;

        let claims = SignClaims {
            sub: &token.user_id,
            user_id: &token.user_id,
            iat: issued_at,
            nbf: issued_at,
            exp: expiration,
            iss: "poprako-server",
        };

        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &self.encoding_key).map_err(|err| {
            BaseError::Unrecoverable {
                message: format!(
                    "[JwtAuth::sign_token] error when encoding: {}",
                    err
                ),
            }
        })
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    fn verify_token(&self, raw: &str) -> BaseResult<UserToken> {
        //
        let token_data = decode::<TokenClaims>(
            raw,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| {
            //
            tracing::debug!("[JwtAuth::verify_token] decode failed: {}", err);

            BaseError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-unauthorized"),
            }
        })?;

        accept(UserToken {
            user_id: token_data.claims.user_id,
        })
    }
}
