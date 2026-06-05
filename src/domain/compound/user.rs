use crate::domain::external::token::{TokenParse, TokenSign};
use crate::domain::model::aggr::user::UserToken;
use crate::domain::result::{DomainError, DomainResult};

/// Hashes a password with bcrypt using the default cost factor.
pub fn hash_password(password: &str) -> DomainResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        DomainError::unrecoverable(format!(
            "[user::hash_password] bcrypt hashing failed: {}",
            e
        ))
        .trace()
    })
}

pub fn sign_token<S>(signer: &S, unsigned_token: &UserToken) -> DomainResult<String>
where
    S: TokenSign,
{
    signer.sign(unsigned_token)
}

pub fn parse_token<P>(parser: &P, signed_token: &str) -> DomainResult<UserToken>
where
    P: TokenParse,
{
    parser.parse(signed_token)
}

#[cfg(test)]
mod tests {
    // hash_password_returns_bcrypt_prefix(hash_password)(positive): hashing should return a bcrypt hash with the expected prefix.
    // hash_password_same_input_produces_different_hashes(hash_password)(positive): hashing the same password twice should use different salts.
    // hash_password_empty_string(hash_password)(positive): hashing an empty password should still return a bcrypt hash.
    // hash_password_can_be_verified_by_bcrypt(hash_password)(positive): bcrypt should verify the original password and reject a wrong one.
    // sign_token_delegates_to_codec(sign_token)(positive): token signing should delegate to the provided codec.
    // sign_token_returns_codec_error(sign_token)(negative): token signing should propagate codec errors.
    // parse_token_delegates_to_codec(parse_token)(positive): token parsing should delegate to the provided codec.
    // parse_token_returns_codec_error(parse_token)(negative): token parsing should propagate codec errors.

    use crate::domain::external::token::{TokenParse, TokenSign};

    use super::hash_password;
    use super::parse_token;
    use super::sign_token;
    use crate::domain::model::aggr::user::UserToken;
    use crate::domain::result::DomainError;
    use crate::domain::result::DomainResult;

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

            Ok(UserToken {
                user_id: signed_token.replace("signed:", ""),
            })
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
        let token = UserToken {
            user_id: "user-1".into(),
        };

        let signed = sign_token(&codec, &token).unwrap();

        assert_eq!(signed, "signed:user-1");
    }

    #[test]
    fn sign_token_returns_codec_error() {
        let codec = FakeCodec { fail: true };
        let token = UserToken {
            user_id: "user-1".into(),
        };

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
