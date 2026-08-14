//! JWT-backed authentication token signer.

#[cfg(test)]
mod tests;

use anyhow::Context as _;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::shared::user::UserToken;
use crate::part::auth::TokenAuth;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

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
    pub fn new(secret: &str, expiration_hours: i64) -> BaseRest<Self> {
        //
        // Internal implementation detail.
        if expiration_hours <= 0 {
            //
            return Err(BaseError::Unrecoverable {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS must be positive"
                    .to_string(),
            });
        }

        let expiration_seconds = expiration_hours * 3600;

        let (encoding_key, decoding_key) = (
            EncodingKey::from_secret(secret.as_bytes()),
            DecodingKey::from_secret(secret.as_bytes()),
        );

        accept(Self {
            expiration_seconds,
            encoding_key,
            decoding_key,
        })
    }

    // Reads token TTL from env vars and builds a signer with validated config.
    /// Reads JWT settings from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        // Internal implementation detail.
        let secret = std::env::var("JWT_SECRET")
            .with_context(|| "[JwtAuth::from_env] JWT_SECRET is not set")?;

        let expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
            .with_context(|| "[JwtAuth::from_env] JWT_EXPIRATION_HOURS is not set")?
            .parse()
            .with_context(
                || "[JwtAuth::from_env] JWT_EXPIRATION_HOURS must be an integer",
            )?;

        Self::new(&secret, expiration_hours).map_err(|err| match err {
            //
            BaseError::Expected { message, .. }
            | BaseError::Retryable { message }
            | BaseError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
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
            //
            tracing::error!(
                operation = "sign_token",
                sdk_err = ?err,
                "JWT SDK error",
            );

            BaseError::Unrecoverable {
                message: format!(
                    "[JwtAuth::sign_token] error when encoding: {}",
                    err
                ),
            }
        })
    }

    // Verifies a JWT token string and returns the decoded user token.
    #[instrument(level = "info", skip_all)]
    fn verify_token(&self, raw: &str) -> BaseRest<UserToken> {
        //
        // Internal implementation detail.
        let token_data = decode::<TokenClaims>(
            raw,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| {
            //
            let err_message = trl("error-unauthorized");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Auth,
                err_message = %err_message,
                operation = "verify_token",
                sdk_err = ?err,
                "JWT SDK error converted to expected authentication error",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Auth,
                message: err_message,
            }
        })?;

        accept(UserToken {
            user_id: token_data.claims.user_id,
        })
    }
}

/// Internal JWT claim structure used for token signing.
#[derive(Debug, Serialize)]
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
#[derive(Debug, Deserialize)]
// Exposes the user identifier after verifying a token.
struct TokenClaims {
    // User primary key recovered from the payload.
    user_id: String,
}
