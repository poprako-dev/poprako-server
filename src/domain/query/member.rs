#[async_trait::async_trait]
pub trait MemberQueryMut {
    async fn create(
        &mut self,
        form: crate::domain::model::aggregate::member::MemberForm,
    ) -> crate::domain::result::DomainResl<crate::domain::model::aggregate::member::Member>;
}
