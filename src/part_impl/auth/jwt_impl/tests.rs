// sign_token(JwtAuth::sign_token)(positive): signed JWT should contain the requested user id.
// new_rejects_non_positive_expiration(JwtAuth::new)(negative): non-positive lifetimes should fail during construction.

use super::*;

use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::Deserialize;

use crate::model::user::UserToken;

#[derive(Debug, Deserialize)]
struct TestClaims {
    sub: String,
    user_id: String,
}

#[test]
fn sign_token() {
    //
    let auth = JwtAuth::new("test-secret", 1).unwrap();

    let signed_token = TokenAuth::sign_token(
        &auth,
        &UserToken {
            user_id: "user-1".into(),
        },
    );

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
    //
    let err = JwtAuth::new("test-secret", 0).err().unwrap();

    assert!(matches!(err, RegularError::Unrecoverable { .. }));
}
