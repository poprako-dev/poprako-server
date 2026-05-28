use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResl;

pub fn hash_password(password: &str) -> DomainResl<String> {
    unimplemented!()
}

pub fn sign_token(unsigned_token: &UserToken) -> DomainResl<String> {
    unimplemented!()
}

pub fn parse_token(signed_token: &str) -> DomainResl<UserToken> {
    unimplemented!()
}
