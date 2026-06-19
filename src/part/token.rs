use crate::model::user::UserToken;
use crate::result::RootResult;

pub trait TokenAuth {
    fn sign(&self, token: &UserToken) -> RootResult<String>;
}
