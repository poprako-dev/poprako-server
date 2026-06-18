use crate::model::user::UserToken;
use crate::result::RootResult;

pub trait TokenIssuer {
    fn sign(&self, token: &UserToken) -> RootResult<String>;
}
