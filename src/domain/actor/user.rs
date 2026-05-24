use crate::domain::{actor::ActorRetVal, model::aggr::user::UserToken};

pub fn hash_password(password: &str) -> ActorRetVal<String> {
    unimplemented!()
}

pub fn sign_token(unsigned_token: &UserToken) -> ActorRetVal<String> {
    unimplemented!()
}

pub fn parse_token(signed_token: &str) -> ActorRetVal<UserToken> {
    unimplemented!()
}
