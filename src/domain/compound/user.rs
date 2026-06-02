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

#[cfg(test)]
mod tests {
    use crate::domain::external::token::{TokenParse, TokenSign};

    use super::*;

    struct FakeCodec {
        fail: bool,
    }

    impl TokenSign for FakeCodec {
        fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
            if self.fail {
                return Err(DomainError::unrecoverable("sign failed".into()));
            }

            Ok(format!("signed:{}", unsigned_token.user_id))
        }
    }

    impl TokenParse for FakeCodec {
        fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
            if self.fail {
                return Err(DomainError::unrecoverable("parse failed".into()));
            }

            Ok(UserToken::new(signed_token.replace("signed:", "")))
        }
    }

    #[test]
    fn hash_password_returns_bcrypt_prefix() {
        let hash = hash_password("my-password").unwrap();
        assert!(hash.starts_with("$2b$"));
    }

    #[test]
    fn hash_password_same_input_produces_different_hashes() {
        let h1 = hash_password("my-password").unwrap();
        let h2 = hash_password("my-password").unwrap();
        assert_ne!(h1, h2, "bcrypt must use a random salt for each hash");
    }

    #[test]
    fn hash_password_empty_string() {
        let hash = hash_password("").unwrap();
        assert!(hash.starts_with("$2b$"));
    }

    #[test]
    fn hash_password_can_be_verified_by_bcrypt() {
        let hash = hash_password("my-password").unwrap();
        assert!(bcrypt::verify("my-password", &hash).unwrap());
        assert!(!bcrypt::verify("wrong-password", &hash).unwrap());
    }

    #[test]
    fn sign_token_delegates_to_codec() {
        let codec = FakeCodec { fail: false };
        let token = UserToken::new("user-1".into());

        let signed = sign_token(&codec, &token).unwrap();

        assert_eq!(signed, "signed:user-1");
    }

    #[test]
    fn sign_token_returns_codec_error() {
        let codec = FakeCodec { fail: true };
        let token = UserToken::new("user-1".into());

        let err = sign_token(&codec, &token).err().unwrap();

        assert!(matches!(err, DomainError::Unrecoverable { .. }));
    }

    #[test]
    fn parse_token_delegates_to_codec() {
        let codec = FakeCodec { fail: false };

        let parsed = parse_token(&codec, "signed:user-1").unwrap();

        assert_eq!(parsed.user_id, "user-1");
    }

    #[test]
    fn parse_token_returns_codec_error() {
        let codec = FakeCodec { fail: true };

        let err = parse_token(&codec, "signed:user-1").err().unwrap();

        assert!(matches!(err, DomainError::Unrecoverable { .. }));
    }
}
