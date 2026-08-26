//! Mock implementation of [TokenAuth] for testing token signing with deterministic output.

use poprako_util::i18n::trl;

use crate::model::shared::user::UserToken;
use crate::part::auth::TokenAuth;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Mock implementation of [TokenAuth].
///
/// Returns a deterministic token (`"token:{user_id}"`) by default.
/// Configure [Mock::with_token_failure] to test sign failures.
impl TokenAuth for Mock {
    // Internal implementation of `sign_token`.
    fn sign_token(&self, token: &UserToken) -> BaseRest<String> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().token_failure {
            return Err(BaseError::Unrecoverable {
                message: "mock token signing failed".into(),
            });
        }

        accept(format!("token:{}", token.user_id))
    }

    // Internal implementation of `verify_token`.
    fn verify_token(&self, raw: &str) -> BaseRest<UserToken> {
        //
        // Internal implementation detail.
        if self.flags.lock().unwrap().token_failure {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-unauthorized"),
            });
        }

        let user_id = raw.strip_prefix("token:").unwrap_or(raw).to_string();

        accept(UserToken { user_id })
    }
}

// sign_returns_stable_token(TokenAuth::sign)(positive): token signing should return the deterministic mock token.
// sign_failure_returns_expected_auth(TokenAuth::sign)(negative): configured token failures should return an expected auth error.

/// Mock helper that returns a stable deterministic token.
#[test]
fn sign_returns_stable_token() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    let signed = TokenAuth::sign_token(
        &mock,
        &UserToken {
            user_id: "user-1".into(),
        },
    );

    assert!(signed.is_ok());

    let signed = signed.ok().unwrap();

    assert_eq!(signed, "token:user-1");
}

/// Mock helper that verifies signing failure returns an unrecoverable error.
#[test]
fn sign_failure_returns_unrecoverable() {
    //
    // Internal implementation detail.
    let mock = Mock::new().with_token_failure();

    let err_token = TokenAuth::sign_token(
        &mock,
        &UserToken {
            user_id: "user-1".into(),
        },
    )
    .err()
    .unwrap();

    assert!(matches!(err_token, BaseError::Unrecoverable { .. }));
}
