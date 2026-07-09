//! Mock implementation of [TokenAuth] for testing token signing with deterministic output.

use poprako_util::i18n::trl;

use crate::model::user::{UserToken, UserTokenRef};
use crate::part::auth::TokenAuth;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

/// Mock implementation of [TokenAuth].
///
/// Returns a deterministic token (`"token:{user_id}"`) by default.
/// Configure [Mock::with_token_failure] to test sign failures.
impl TokenAuth for Mock {
    fn sign_token(&self, token: &UserTokenRef) -> RegularResult<String> {
        //
        if self.flags.lock().unwrap().token_failure {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-token-sign-failed"),
            });
        }

        Ok(format!("token:{}", token.user_id))
    }

    fn verify_token(&self, raw: &str) -> RegularResult<UserToken> {
        //
        if self.flags.lock().unwrap().token_failure {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-unauthorized"),
            });
        }

        let user_id = raw.strip_prefix("token:").unwrap_or(raw).to_string();

        Ok(UserToken { user_id })
    }
}

// sign_returns_stable_token(TokenAuth::sign)(positive): token signing should return the deterministic mock token.
// sign_failure_returns_expected_auth(TokenAuth::sign)(negative): configured token failures should return an expected auth error.

#[test]
fn sign_returns_stable_token() {
    let mock = Mock::new();

    let signed =
        TokenAuth::sign_token(&mock, &UserTokenRef { user_id: "user-1" });
    assert!(signed.is_ok());
    let signed = signed.ok().unwrap();

    assert_eq!(signed, "token:user-1");
}

#[test]
fn sign_failure_returns_expected_auth() {
    let mock = Mock::new().with_token_failure();

    let err = TokenAuth::sign_token(&mock, &UserTokenRef { user_id: "user-1" })
        .err()
        .unwrap();

    assert!(matches!(
        err,
        RegularError::Expected {
            variant: ExpectedVariant::Auth,
            ..
        }
    ));
}
