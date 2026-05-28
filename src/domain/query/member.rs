use async_trait::async_trait;

/// Mutable persistence contract for [`Member`](crate::domain::model::aggregate::member::Member),
/// used **only** inside a transaction via [`TransactionalQuery`](crate::domain::query::TransactionalQuery).
#[async_trait]
pub trait MemberQueryMut {
    /// Inserts a new member row from the creation form.
    async fn create(
        &mut self,
        form: crate::domain::model::aggregate::member::MemberForm,
    ) -> crate::domain::result::DomainResl<crate::domain::model::aggregate::member::Member>;
}
