//! Complex domain logic for [User] aggregates — password hashing, ID generation, and avatar storage key construction.

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{
    PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString,
};

use crate::result::{BaseError, BaseRest};
use crate::util::next_snowflake_id;

/// Domain opers for [User] aggregates: password hashing and verification via Argon2id, ID generation, and avatar storage key computation.
pub struct UserComplex;

impl UserComplex {
    /// Generates a unique user identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Hashes a plaintext password on Tokio's blocking pool and returns its Argon2id-encoded value.
    pub async fn hash_password(password: &str) -> BaseRest<String> {
        //
        let password = password.to_owned();

        tokio::task::spawn_blocking(move || hash_password_sync(&password))
            .await
            .map_err(|error| BaseError::Unrecoverable {
                message: format!(
                    "[UserComplex::hash_password] blocking task failed: {}",
                    error
                ),
            })?
    }

    /// Verifies a plaintext password on Tokio's blocking pool against an Argon2id-encoded hash.
    /// TODO: no need to return bool.
    pub async fn verify_password(password: &str, password_hash: &str) -> bool {
        //
        let (password, password_hash) =
            (password.to_owned(), password_hash.to_owned());

        match tokio::task::spawn_blocking(move || {
            verify_password_sync(&password, &password_hash)
        })
        .await
        {
            Ok(is_valid) => is_valid,

            Err(error) => {
                //
                tracing::error!(
                    err = %error,
                    "[UserComplex::verify_password] blocking task failed",
                );

                false
            }
        }
    }

    /// Hashes a plaintext password using the sync runtime (test-only helper).
    #[cfg(test)]
    pub fn hash_password_for_test(password: &str) -> BaseRest<String> {
        hash_password_sync(password)
    }

    /// Constructs the object storage key for a user's avatar image from the user ID, version counter, and file extension.
    pub fn gen_avatar_key(
        id: &str,
        avatar_version: u32,
        file_ext: &str,
    ) -> String {
        format!("user_avatar/{}-{}.{}", id, avatar_version, file_ext)
    }
}

// Hashes a plaintext password with the Argon2id algorithm on the current thread.
fn hash_password_sync(password: &str) -> BaseRest<String> {
    //
    let salt = SaltString::generate(OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| BaseError::Unrecoverable {
            message: format!(
                "[UserComplex::hash_password] argon2 hashing failed: {}",
                e
            ),
        })
}

// Verifies a plaintext password against an Argon2id hash on the current thread.
fn verify_password_sync(password: &str, password_hash: &str) -> bool {
    //
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
