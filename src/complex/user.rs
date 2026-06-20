use argon2::Argon2;
use argon2::password_hash::PasswordHash;
use argon2::password_hash::PasswordHasher as _;
use argon2::password_hash::PasswordVerifier as _;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use uuid::Uuid;

use crate::result::Error as RootError;
use crate::result::RootResult;

pub struct UserComplex;

impl UserComplex {
    pub fn gen_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    pub fn hash_password(password: &str) -> RootResult<String> {
        let salt = SaltString::generate(OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| RootError::Unrecoverable {
                message: format!("[UserComplex::hash_password] argon2 hashing failed: {}", e),
            })
    }

    pub fn verify_password(password: &str, password_hash: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(password_hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .inspect_err(|e| {
                tracing::warn!(
                    "[UserComplex::verify_password] failed to verify_password: {}",
                    e
                )
            })
            .is_ok()
    }

    // TODO: use.
    pub fn gen_avatar_key(prev_version: Option<&str>) -> String {
        todo!()
    }

    pub fn gen_avatar_delete_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }

    pub fn gen_avatar_check_id() -> String {
        format!("lm-{}", Uuid::now_v7())
    }
}
