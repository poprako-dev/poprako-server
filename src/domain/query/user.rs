use async_trait::async_trait;

use crate::domain::model::aggr::user::{User, UserCredential, UserForm};
use crate::domain::query;

#[async_trait]
pub trait UserQeury: Send {
    async fn get_by_id(&self, id: &str) -> query::QueryRetVal<User>;
    async fn get_credentials_by_qid(&self, qid: &str) -> query::QueryRetVal<UserCredential>;
    async fn create(&self, form: UserForm) -> query::QueryRetVal<User>;
}

#[async_trait]
pub trait UserQeuryMut: Send {
    async fn create(&mut self, form: UserForm) -> query::QueryRetVal<User>;
}
