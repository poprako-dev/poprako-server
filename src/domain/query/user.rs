use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::result::DomainResl;

#[async_trait::async_trait]
pub trait UserQeury: Send {
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr>;
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResl<UserCredential>;
    async fn create(&self, form: UserForm) -> DomainResl<UserAggr>;
}

#[async_trait::async_trait]
pub trait UserQeuryMut: Send {
    async fn create(&mut self, form: UserForm) -> DomainResl<UserAggr>;
}
