use async_trait::async_trait;

use crate::domain::model::aggr::member::{MemberAggr, MemberForm};
use crate::domain::result::DomainResult;

/// Mutable persistence contract for [`MemberAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait MemberQueryTransactional {
    /// Inserts a new member row from the creation form.
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr>;

    /// Updates the nickname on all member rows belonging to the given user.
    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()>;

    /// Updates the last active timestamp on all member rows belonging to the given user.
    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()>;
}
