//! JWT-backed authentication token signer.

#[cfg(test)]
mod tests;

use std::env::var;

use anyhow::Context as _;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::shared::user::UserToken;
use crate::part::auth::TokenAuth;
use crate::result::{BaseError, BaseRest, ExpectedVariant};

/// JWT issuer for user session tokens.
pub struct JwtAuth {
    // Internal state field `expiration_seconds`.
    /// Token lifetime in seconds from issuance.
    expiration_seconds: i64,

    /// RSA/DSA key material used to sign outgoing tokens.
    encoding_key: EncodingKey,

    /// RSA/DSA key material used to verify incoming tokens.
    decoding_key: DecodingKey,
}

impl JwtAuth {
    // Creates a JWT signer from a shared secret and token lifetime in hours.
    /// Creates a JWT signer from a shared secret and token lifetime.
    ///
    /// # Errors
    ///
    /// Returns an error when the expiration is non-positive or overflows.
    pub fn new(secret: &str, expiration_hours: i64) -> BaseRest<Self> {
        //
        // Internal implementation detail.
        if expiration_hours <= 0 {
            //
            return Err(unrecoverable_error! {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS must be positive"
                    .to_string();
                expiration_hours,
                "JWT_EXPIRATION_HOURS must be positive",
            });
        }

        let Some(expiration_seconds) = expiration_hours.checked_mul(3600)
        else {
            //
            return Err(unrecoverable_error! {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS is too large"
                    .to_string();
                expiration_hours,
                "JWT_EXPIRATION_HOURS is too large",
            });
        };

        let (encoding_key, decoding_key) = (
            EncodingKey::from_secret(secret.as_bytes()),
            DecodingKey::from_secret(secret.as_bytes()),
        );

        accept!(Self {
            expiration_seconds,
            encoding_key,
            decoding_key,
        })
    }

    // Reads token TTL from env vars and builds a signer with validated config.
    /// Reads JWT settings from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error when required environment variables are missing or
    /// malformed.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        // Internal implementation detail.
        let secret = var("JWT_SECRET")
            .with_context(|| "[JwtAuth::from_env] JWT_SECRET is not set")?;

        let expiration_hours = var("JWT_EXPIRATION_HOURS")
            .with_context(|| "[JwtAuth::from_env] JWT_EXPIRATION_HOURS is not set")?
            .parse()
            .with_context(
                || "[JwtAuth::from_env] JWT_EXPIRATION_HOURS must be an integer",
            )?;

        Self::new(&secret, expiration_hours).map_err(|error| match error {
            //
            BaseError::Expected { message, .. }
            | BaseError::Retryable { message }
            | BaseError::Unavailable { message }
            | BaseError::Unrecoverable { message } => {
                anyhow::Error::msg(message)
            }
        })
    }
}

impl TokenAuth for JwtAuth {
    // Signs a user token by encoding JWT claims with configured expiration.
    #[instrument(level = "info", skip_all)]
    fn sign_token(&self, token: &UserToken) -> BaseRest<String> {
        //
        // Internal implementation detail.
        let now = OffsetDateTime::now_utc();

        let timestamp = now.unix_timestamp();

        let issued_at = usize::try_from(timestamp).map_err(|_| {
            //
            unrecoverable_error! {
                message: "JWT issue timestamp is outside the supported range"
                    .to_string();
                timestamp,
                "JWT issue timestamp is outside the supported range",
            }
        })?;

        let expiration_timestamp = timestamp
            .checked_add(self.expiration_seconds)
            .ok_or_else(|| unrecoverable_error! {
                message:
                    "JWT expiration timestamp is outside the supported range"
                    .to_string();
                timestamp,
                expiration_seconds = self.expiration_seconds,
                "JWT expiration timestamp is outside the supported range",
            })?;

        let expiration =
            usize::try_from(expiration_timestamp).map_err(|_| {
                //
                unrecoverable_error! {
                    message:
                        "JWT expiration timestamp is outside \
                            the supported range"
                        .to_string();
                    expiration_timestamp,
                    "JWT expiration timestamp is outside the supported range",
                }
            })?;

        let claims = SignClaims {
            sub: &token.user_id,
            user_id: &token.user_id,
            iat: issued_at,
            nbf: issued_at,
            exp: expiration,
            iss: "poprako-server",
        };

        let header = Header::new(Algorithm::HS256);

        jsonwebtoken::encode(&header, &claims, &self.encoding_key).map_err(
            |error| {
                //
                unrecoverable_error! {
                    message: [
                        "[JwtAuth::sign_token] error when encoding: ",
                        &error.to_string(),
                    ]
                    .concat();
                    operation = "sign_token",
                    sdk_error = ?error,
                    "JWT SDK error",
                }
            },
        )
    }

    // Verifies a JWT token string and returns the decoded user token.
    #[instrument(level = "info", skip_all)]
    fn verify_token(&self, raw: &str) -> BaseRest<UserToken> {
        //
        // Internal implementation detail.
        let token_data = jsonwebtoken::decode::<TokenClaims>(
            raw,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|error| {
            //
            expected_error! {
                ExpectedVariant::Auth,
                message: trl("error-unauthorized");
                operation = "verify_token",
                sdk_error = ?error,
                "JWT verification failed",
            }
        })?;

        accept!(UserToken {
            user_id: token_data.claims.user_id,
        })
    }
}

/// Internal JWT claim structure used for token signing.
#[derive(Serialize)]
// Holds JWT standard + app-specific claim fields emitted by the signer.
struct SignClaims<'a> {
    // Internal state field `sub`.
    // JWT standard subject claim carrying the user primary key.
    sub: &'a str,
    // Copy of the user primary key, consistent with the business model.
    user_id: &'a str,

    // Token issued-at time (Unix second timestamp).
    iat: usize,
    // Token not-before time, preventing use before the validity window opens.
    nbf: usize,
    // Token expiration time (Unix second timestamp).
    exp: usize,

    // Token issuer identifier for origin verification during validation.
    iss: &'static str,
}

/// Decoded JWT token claims containing user identification.
#[derive(Deserialize)]
// Exposes the user identifier after verifying a token.
struct TokenClaims {
    // User primary key recovered from the payload.
    user_id: String,
}
