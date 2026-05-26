use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainRetVal;

pub fn hash_password(password: &str) -> DomainRetVal<String> {
    unimplemented!()
}

pub fn sign_token(unsigned_token: &UserToken) -> DomainRetVal<String> {
    unimplemented!()
}

pub fn parse_token(signed_token: &str) -> DomainRetVal<UserToken> {
    unimplemented!()
}
