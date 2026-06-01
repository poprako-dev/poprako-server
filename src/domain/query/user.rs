use async_trait::async_trait;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::result::DomainResult;

/// Persistence contract for [`UserAggr`].
#[async_trait]
pub trait UserQuery {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;

    /// Returns credentials (hashed password) for the given qualified ID.
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential>;
}

/// Mutable persistence contract for [`UserAggr`], used **only** inside
/// [`Transactional::run_in_transaction`](crate::domain::query::Transactional::run_in_transaction).
///
/// Takes `&mut self` to enforce single-writer semantics within a transaction.
#[async_trait]
pub trait UserQueryTransactional {
    /// Creates a new user from the registration form inside a transaction.
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr>;
}
