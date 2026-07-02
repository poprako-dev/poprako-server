//! JWT-backed authentication token signer.

use anyhow::Context as _;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;
use time::OffsetDateTime;
use tracing::{Level, instrument};

use crate::model::user::UserTokenRef;
use crate::part::auth::TokenAuth;
use crate::result::{RegularError, RegularResult};

/// JWT issuer for user session tokens.
pub struct JwtAuth {
    expiration_seconds: i64,

    encoding_key: EncodingKey,
}

#[derive(Debug, Serialize)]
struct SignClaims<'a> {
    sub: &'a str,
    user_id: &'a str,

    iat: usize,
    nbf: usize,
    exp: usize,

    iss: &'static str,
}

impl JwtAuth {
    /// Creates a JWT signer from a shared secret and token lifetime.
    pub fn new(secret: &str, expiration_hours: i64) -> RegularResult<Self> {
        if expiration_hours <= 0 {
            return Err(RegularError::Unrecoverable {
                message: "[JwtAuth::new] JWT_EXPIRATION_HOURS must be positive".to_string(),
            });
        }

        let expiration_seconds = expiration_hours * 3600;

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        Ok(Self {
            expiration_seconds,
            encoding_key,
        })
    }

    /// Reads JWT settings from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .with_context(|| "[JwtAuth::from_env] JWT_SECRET is not set")?;

        let expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
            .with_context(|| "[JwtAuth::from_env] JWT_EXPIRATION_HOURS is not set")?
            .parse()
            .with_context(|| "[JwtAuth::from_env] JWT_EXPIRATION_HOURS must be an integer")?;

        Self::new(&secret, expiration_hours).map_err(|err| match err {
            RegularError::Expected { message, .. } | RegularError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
            }
        })
    }
}

impl TokenAuth for JwtAuth {
    #[instrument(err(Debug), skip(self, token), level = Level::DEBUG)]
    fn sign_token(&self, token: &UserTokenRef) -> RegularResult<String> {
        let now = OffsetDateTime::now_utc();

        let issued_at = now.unix_timestamp() as usize;

        let expiration = (now.unix_timestamp() + self.expiration_seconds) as usize;

        let claims = SignClaims {
            sub: token.user_id,
            user_id: token.user_id,
            iat: issued_at,
            nbf: issued_at,
            exp: expiration,
            iss: "poprako-r",
        };

        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &self.encoding_key).map_err(|err| RegularError::Unrecoverable {
            message: format!("[JwtAuth::sign_token] error when encoding: {}", err),
        })
    }
}

#[cfg(test)]
mod tests {
    // sign_token(JwtAuth::sign_token)(positive): signed JWT should contain the requested user id.
    // new_rejects_non_positive_expiration(JwtAuth::new)(negative): non-positive lifetimes should fail during construction.

    use super::*;

    use jsonwebtoken::{DecodingKey, Validation, decode};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestClaims {
        sub: String,
        user_id: String,
    }

    #[test]
    fn sign_token() {
        let auth = JwtAuth::new("test-secret", 1).unwrap();

        let signed_token = TokenAuth::sign_token(&auth, &UserTokenRef { user_id: "user-1" });
        assert!(signed_token.is_ok());

        let signed_token = signed_token.ok().unwrap();

        let token_data = decode::<TestClaims>(
            &signed_token,
            &DecodingKey::from_secret("test-secret".as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, "user-1");

        assert_eq!(token_data.claims.user_id, "user-1");
    }

    #[test]
    fn new_rejects_non_positive_expiration() {
        let err = JwtAuth::new("test-secret", 0).err().unwrap();

        assert!(matches!(err, RegularError::Unrecoverable { .. }));
    }
}
