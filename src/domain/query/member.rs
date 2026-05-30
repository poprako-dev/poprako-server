use async_trait::async_trait;

use crate::domain::model::aggregate::member::{Member, MemberForm};
use crate::domain::result::DomainResult;

/// Mutable persistence contract for [`Member`](crate::domain::model::aggregate::member::Member),
/// used **only** inside a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait MemberQueryTransactional {
    /// Inserts a new member row from the creation form.
    async fn create(&mut self, form: MemberForm) -> DomainResult<Member>;
}
