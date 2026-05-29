use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResult;

pub fn hash_password(password: &str) -> DomainResult<String> {
    unimplemented!()
}

pub fn sign_token(unsigned_token: &UserToken) -> DomainResult<String> {
    unimplemented!()
}

pub fn parse_token(signed_token: &str) -> DomainResult<UserToken> {
    unimplemented!()
}
