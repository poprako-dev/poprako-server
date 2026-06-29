//! Complex domain logic for [User] aggregates — password hashing, ID generation, and avatar storage key construction.

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString};

use crate::result::{Error as RootError, RootResult};
use crate::util::next_snowflake_id;

/// Domain opers for [User] aggregates: password hashing and verification via Argon2id, ID generation, and avatar storage key computation.
pub struct UserComplex;

impl UserComplex {
    /// Generates a unique user identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Hashes a plaintext password with Argon2id and a random salt, returning the encoded hash string.
    pub fn hash_password(password: &str) -> RootResult<String> {
        let salt = SaltString::generate(OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| RootError::Unrecoverable {
                message: format!("[UserComplex::hash_password] argon2 hashing failed: {}", e),
            })
    }

    /// Verifies a plaintext password against an Argon2id-encoded hash. Returns `false` on parse or verification failure.
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

    /// Constructs the object storage key for a user's avatar image from the user ID, version counter, and file extension.
    pub fn gen_avatar_key(id: &str, avatar_version: i64, file_ext: &str) -> String {
        format!("user_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }
}
