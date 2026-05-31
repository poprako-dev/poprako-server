use async_trait::async_trait;

use crate::domain::model::aggregate::member::{MemberAggr, MemberForm};
use crate::domain::result::DomainResult;

/// Mutable persistence contract for [`MemberAggr`], used **only** inside
/// a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait MemberQueryTransactional {
    /// Inserts a new member row from the creation form.
    async fn create<'s, 'a>(&'s mut self, form: &'a MemberForm) -> DomainResult<MemberAggr>;
}
