use async_trait::async_trait;

use crate::domain::model::aggregate::user::{UserAggr, UserCredential, UserForm};
use crate::domain::result::DomainResult;

/// Read-only persistence contract for [`UserAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[async_trait]
pub trait UserQuery {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;

    /// Returns credentials (hashed password) for the given qualified ID.
    async fn get_credentials_by_qid(&self, qid: String) -> DomainResult<UserCredential>;
}

/// Mutable persistence contract for [`UserAggr`], used **only** inside
/// [`Transactional::run_in_transaction`](crate::domain::query::Transactional::run_in_transaction).
///
/// Takes `&mut self` to enforce single-writer semantics within a transaction.
#[async_trait]
pub trait UserQueryTransactional {
    /// Creates a new user from the registration form inside a transaction.
    async fn create<'s, 'a>(&'s mut self, form: &'a UserForm) -> DomainResult<UserAggr>;
}
