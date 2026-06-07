use async_trait::async_trait;
use poprako_macro::forward_ref;

use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserForm, UserInfoUpdate};
use crate::domain::result::DomainResult;

/// Persistence contract for [`UserAggr`].
#[forward_ref]
#[async_trait]
pub trait UserQuery {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;

    /// Returns credentials (hashed password) for the given qualified ID.
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential>;

    /// Sets the avatar key for the given user.
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()>;

    /// Marks the user's avatar as uploaded.
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()>;
}

/// Mutable persistence contract for [`UserAggr`], used **only** inside
/// [`Transactional::run_in_transaction`](crate::domain::query::Transactional::run_in_transaction).
///
/// Takes `&mut self` to enforce single-writer semantics within a transaction.
#[async_trait]
pub trait UserQueryTransactional {
    /// Creates a new user from the registration form inside a transaction.
    async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr>;

    /// Updates user profile fields via PUT semantics inside a transaction.
    async fn update_info(&mut self, input: &UserInfoUpdate) -> DomainResult<UserAggr>;

    /// Updates the user's last active timestamp inside a transaction.
    async fn touch_last_active(&mut self, id: &str) -> DomainResult<()>;
}
