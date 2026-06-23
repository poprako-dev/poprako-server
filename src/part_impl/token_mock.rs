use poprako_util::i18n::trl;

use crate::model::user::UserToken;
use crate::part::token::TokenAuth;
use crate::part_impl::repo_mock::Mock;
use crate::result::{ExpectedVariant, RootError, RootResult};

impl TokenAuth for Mock {
    fn sign(&self, token: &UserToken) -> RootResult<String> {
        if self.flags.lock().unwrap().token_failure {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Auth,
                message: trl("error-token-sign-failed"),
            });
        }

        Ok(format!("token:{}", token.user_id))
    }
}

#[cfg(test)]
mod tests {
    // sign_returns_stable_token(TokenAuth::sign)(positive): token signing should return the deterministic mock token.
    // sign_failure_returns_expected_auth(TokenAuth::sign)(negative): configured token failures should return an expected auth error.

    use super::*;

    #[test]
    fn sign_returns_stable_token() {
        let mock = Mock::new();

        let signed = TokenAuth::sign(
            &mock,
            &UserToken {
                user_id: "user-1".into(),
            },
        );
        assert!(signed.is_ok());
        let signed = signed.ok().unwrap();

        assert_eq!(signed, "token:user-1");
    }

    #[test]
    fn sign_failure_returns_expected_auth() {
        let mock = Mock::new().with_token_failure();

        let err = TokenAuth::sign(
            &mock,
            &UserToken {
                user_id: "user-1".into(),
            },
        )
        .err()
        .unwrap();

        assert!(matches!(
            err,
            RootError::Expected {
                variant: ExpectedVariant::Auth,
                ..
            }
        ));
    }
}
