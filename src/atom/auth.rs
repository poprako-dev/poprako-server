use crate::result::ScopeResult;

pub fn hash_password(password: &str) -> ScopeResult<String> {
    // TODO: argon2.
    todo!()
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    // TODO: argon2.
    todo!()
}
