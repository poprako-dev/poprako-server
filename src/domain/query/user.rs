use async_trait::async_trait;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::result::DomainRetVal;

#[async_trait]
pub trait UserQeury: Send {
    async fn get_by_id(&self, id: &str) -> DomainRetVal<UserAggr>;
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainRetVal<UserCredential>;
    async fn create(&self, form: UserForm) -> DomainRetVal<UserAggr>;
}

#[async_trait]
pub trait UserQeuryMut: Send {
    async fn create(&mut self, form: UserForm) -> DomainRetVal<UserAggr>;
}
