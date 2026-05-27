use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::DomainResl;

pub trait TokenCodec {
    // Transform an unsigned token into a signed token.
    fn sign(&self, unsigned_token: &UserToken) -> DomainResl<String>;

    // Transform a signed token string back into an unsigned token.
    fn parse(&self, signed_token: &str) -> DomainResl<UserToken>;
}
