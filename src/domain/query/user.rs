use async_trait::async_trait;

use crate::domain::model::aggr::user::{User, UserCredential, UserForm};
use crate::domain::query;

#[async_trait]
pub trait UserQeury: Send {
    // query retrieves a user by its id.
    async fn get_by_id(&mut self, id: &str) -> query::QueryResult<User>;

    // qeury retrieves a user credential by its qid.
    async fn get_credentials_by_qid(&mut self, qid: &str) -> query::QueryResult<UserCredential>;

    // query creates a user with the given info, and returns the created user.
    async fn create(&mut self, form: UserForm) -> query::QueryResult<User>;
}
