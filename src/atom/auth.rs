use crate::result::{RootError, RootResult};

pub fn hash_password(password: &str) -> RootResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| RootError::Unrecoverable {
        message: format!("[auth::hash_password] bcrypt hashing failed: {}", e),
    })
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    bcrypt::verify(password, password_hash).unwrap_or(false)
}
