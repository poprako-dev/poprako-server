use crate::result::RootResult;

pub fn hash_password(password: &str) -> RootResult<String> {
    // TODO: argon2.
    todo!()
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    // TODO: argon2.
    todo!()
}
